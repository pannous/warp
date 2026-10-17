//! Field names of WASM GC structs, read from a module's name section

use super::*;
use std::sync::LazyLock;
use wasmparser as wp;

static REGISTRY: LazyLock<Mutex<FieldNameRegistry>> = LazyLock::new(|| Mutex::new(FieldNameRegistry::default()));

/// The registry only caches parsed name sections, so a panic elsewhere while it was locked (a should_panic test,
/// say) leaves it usable; that panic already reported itself, and poisoning would fail every later lookup
fn registry() -> std::sync::MutexGuard<'static, FieldNameRegistry> {
	REGISTRY.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub fn register_module(bytes: &[u8]) -> Result<u64> {
	let mut registry = registry();
	registry.register_module(bytes)
}

pub fn lookup_field_index(struct_type: &StructType, field_name: &str, module: Option<u64>) -> Result<usize> {
	let mut registry = registry();
	registry.lookup(struct_type, field_name, module)
}

pub fn field_names(struct_type: &StructType, module: Option<u64>) -> Result<Vec<Option<String>>> {
	let mut registry = registry();
	registry.field_names(struct_type, module)
}

pub fn type_name(struct_type: &StructType, module: Option<u64>) -> Result<Option<String>> {
	let mut registry = registry();
	registry.type_name(struct_type, module)
}

#[derive(Default)]
struct FieldNameRegistry {
	modules: Vec<Arc<ParsedModule>>,
	cache: HashMap<(StructTypeKey, Option<u64>), Arc<StructFieldMapping>>,
}

impl FieldNameRegistry {
	fn register_module(&mut self, bytes: &[u8]) -> Result<u64> {
		let hash = module_hash(bytes);
		if self.modules.iter().any(|m| m.hash == hash) {
			return Ok(hash);
		}
		let module = Arc::new(ParsedModule::parse(bytes, hash)?);
		self.modules.push(module);
		Ok(hash)
	}

	fn lookup(&mut self, struct_type: &StructType, field_name: &str, module: Option<u64>) -> Result<usize> {
		let mapping = self.ensure_mapping(struct_type, module)?;
		mapping.index_of(field_name).ok_or_else(|| unknown_field_error(field_name, &mapping))
	}

	fn field_names(&mut self, struct_type: &StructType, module: Option<u64>) -> Result<Vec<Option<String>>> {
		let mapping = self.ensure_mapping(struct_type, module)?;
		Ok(mapping.names.clone())
	}

	fn type_name(&mut self, struct_type: &StructType, module: Option<u64>) -> Result<Option<String>> {
		let mapping = self.ensure_mapping(struct_type, module)?;
		Ok(mapping.type_name.clone())
	}

	/// Resolve names for a struct type: the owning module when known, otherwise
	/// the most recently registered structurally matching one.
	fn ensure_mapping(&mut self, struct_type: &StructType, module: Option<u64>) -> Result<Arc<StructFieldMapping>> {
		if self.modules.is_empty() {
			return Err(anyhow!(
				"No WASM metadata registered. Call `register_gc_types_from_wasm` before accessing fields by name."
			));
		}

		let key = (StructTypeKey(struct_type.clone()), module);
		if let Some(mapping) = self.cache.get(&key) {
			return Ok(mapping.clone());
		}

		let owning = module.and_then(|hash| self.modules.iter().find(|m| m.hash == hash));
		let candidates: Vec<Arc<ParsedModule>> = match owning {
			Some(module) => vec![module.clone()],
			None => self.modules.iter().rev().cloned().collect(),
		};

		for module in candidates {
			if let Some(mapping) = module.try_match(struct_type) {
				let mapping = Arc::new(mapping);
				self.cache.insert(key.clone(), mapping.clone());
				return Ok(mapping);
			}
		}

		Err(anyhow!("Failed to resolve struct fields from registered modules"))
	}
}

fn unknown_field_error(field_name: &str, mapping: &StructFieldMapping) -> anyhow::Error {
	let mut available: Vec<&str> = mapping.available_names();
	available.sort_unstable();
	anyhow!(
		"Unknown field '{}'. Available fields: {}",
		field_name,
		if available.is_empty() { "<none>".to_string() } else { available.join(", ") }
	)
}

fn module_hash(bytes: &[u8]) -> u64 {
	use std::collections::hash_map::DefaultHasher;
	let mut hasher = DefaultHasher::new();
	bytes.hash(&mut hasher);
	hasher.finish()
}

#[derive(Clone)]
struct StructTypeKey(StructType);

/// The cache holds types of every engine (each run has its own), and StructType::eq panics across engines
impl PartialEq for StructTypeKey {
	fn eq(&self, other: &Self) -> bool {
		Engine::same(self.0.engine(), other.0.engine()) && StructType::eq(&self.0, &other.0)
	}
}

impl Eq for StructTypeKey {}

impl Hash for StructTypeKey {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.0.hash(state);
	}
}

#[derive(Clone)]
struct StructFieldMapping {
	type_name: Option<String>,
	names: Vec<Option<String>>,
	lookup: HashMap<String, usize>,
}

impl StructFieldMapping {
	fn new(type_name: Option<String>, names: Vec<Option<String>>) -> Self {
		let lookup = names.iter().enumerate().filter_map(|(index, name)| Some((name.clone()?, index))).collect();
		Self { type_name, names, lookup }
	}

	fn index_of(&self, name: &str) -> Option<usize> {
		self.lookup.get(name).copied()
	}

	fn available_names(&self) -> Vec<&str> {
		self.names.iter().filter_map(|n| n.as_deref()).collect()
	}
}

/// A module's types as wasmparser reads them, each with the start of its recursion group (where its group-relative
/// indices count from), and the names its name section gives them and their fields
struct ParsedModule {
	hash: u64,
	types: Vec<(wp::CompositeInnerType, u32)>,
	type_names: HashMap<u32, String>,
	field_names: HashMap<u32, Vec<(u32, String)>>,
}

impl ParsedModule {
	fn parse(bytes: &[u8], hash: u64) -> Result<Self> {
		let mut module = Self { hash, types: vec![], type_names: HashMap::new(), field_names: HashMap::new() };
		for payload in wp::Parser::new(0).parse_all(bytes) {
			match payload? {
				wp::Payload::TypeSection(reader) => {
					for group in reader {
						let group_start = module.types.len() as u32;
						let types = group?.into_types().map(|subtype| (subtype.composite_type.inner, group_start));
						module.types.extend(types);
					}
				}
				wp::Payload::CustomSection(section) => {
					if let wp::KnownCustom::Name(name_reader) = section.as_known() {
						module.read_name_section(name_reader)?;
					}
				}
				_ => {}
			}
		}
		Ok(module)
	}

	fn read_name_section(&mut self, reader: wp::NameSectionReader<'_>) -> Result<()> {
		for entry in reader {
			match entry? {
				wp::Name::Type(map) => {
					for naming in map {
						let naming = naming?;
						self.type_names.insert(naming.index, naming.name.to_string());
					}
				}
				wp::Name::Field(map) => {
					for naming in map {
						let naming = naming?;
						let names = self.field_names.entry(naming.index).or_default();
						for field in naming.names {
							let field = field?;
							names.push((field.index, field.name.to_string()));
						}
					}
				}
				_ => {}
			}
		}
		Ok(())
	}

	fn try_match(&self, struct_type: &StructType) -> Option<StructFieldMapping> {
		self.types.iter().enumerate().find_map(|(index, (ty, _))| match ty {
			wp::CompositeInnerType::Struct(parsed)
				if MatchContext::new(self).match_struct(index as u32, struct_type) =>
			{
				Some(self.mapping(index as u32, parsed.fields.len()))
			}
			_ => None,
		})
	}

	/// The names of the struct type at `index` and of its fields
	fn mapping(&self, index: u32, field_count: usize) -> StructFieldMapping {
		let mut names = vec![None; field_count];
		for (field, name) in self.field_names.get(&index).into_iter().flatten() {
			if let Some(slot) = names.get_mut(*field as usize) {
				*slot = Some(name.clone());
			}
		}
		StructFieldMapping::new(self.type_names.get(&index).cloned(), names)
	}
}

/// A type index of the module, absolute: a group-relative one counts from its recursion group's start
fn resolve_index(index: wp::UnpackedIndex, group_start: u32) -> Option<u32> {
	match index {
		wp::UnpackedIndex::Module(i) => Some(i),
		wp::UnpackedIndex::RecGroup(i) => Some(group_start + i),
		_ => None,
	}
}

/// A structural match of a module's types against the engine's: each module type index binds to the first runtime
/// type it meets, so a recursive type matches itself
struct MatchContext<'a> {
	module: &'a ParsedModule,
	bindings: HashMap<u32, RuntimeTypeId>,
}

impl<'a> MatchContext<'a> {
	fn new(module: &'a ParsedModule) -> Self {
		Self { module, bindings: HashMap::new() }
	}

	fn match_struct(&mut self, idx: u32, ty: &StructType) -> bool {
		if let Some(existing) = self.bindings.get(&idx) {
			return existing.matches_struct(ty);
		}
		let Some((wp::CompositeInnerType::Struct(parsed), group)) = self.module.types.get(idx as usize) else {
			return false;
		};
		self.bindings.insert(idx, RuntimeTypeId::Struct(ty.clone()));
		parsed.fields.len() == ty.fields().len()
			&& parsed.fields.iter().zip(ty.fields()).all(|(field, runtime)| self.match_field(field, &runtime, *group))
	}

	fn match_array(&mut self, idx: u32, ty: &ArrayType) -> bool {
		if let Some(existing) = self.bindings.get(&idx) {
			return existing.matches_array(ty);
		}
		let Some((wp::CompositeInnerType::Array(parsed), group)) = self.module.types.get(idx as usize) else {
			return false;
		};
		self.bindings.insert(idx, RuntimeTypeId::Array(ty.clone()));
		self.match_field(&parsed.0, &ty.field_type(), *group)
	}

	fn match_func(&mut self, idx: u32, ty: &FuncType) -> bool {
		if let Some(existing) = self.bindings.get(&idx) {
			return existing.matches_func(ty);
		}
		let Some((wp::CompositeInnerType::Func(parsed), group)) = self.module.types.get(idx as usize) else {
			return false;
		};
		self.bindings.insert(idx, RuntimeTypeId::Func(ty.clone()));
		parsed.params().len() == ty.params().len()
			&& parsed.results().len() == ty.results().len()
			&& parsed
				.params()
				.iter()
				.zip(ty.params())
				.all(|(parsed, runtime)| self.match_val_type(parsed, &runtime, *group))
			&& parsed
				.results()
				.iter()
				.zip(ty.results())
				.all(|(parsed, runtime)| self.match_val_type(parsed, &runtime, *group))
	}

	fn match_field(&mut self, parsed: &wp::FieldType, ty: &FieldType, group: u32) -> bool {
		parsed.mutable == ty.mutability().is_var()
			&& self.match_storage_type(&parsed.element_type, ty.element_type(), group)
	}

	fn match_storage_type(&mut self, parsed: &wp::StorageType, ty: &StorageType, group: u32) -> bool {
		match (parsed, ty) {
			(wp::StorageType::I8, StorageType::I8) | (wp::StorageType::I16, StorageType::I16) => true,
			(wp::StorageType::Val(parsed), StorageType::ValType(ty)) => self.match_val_type(parsed, ty, group),
			_ => false,
		}
	}

	fn match_val_type(&mut self, parsed: &wp::ValType, ty: &ValType, group: u32) -> bool {
		match (parsed, ty) {
			(wp::ValType::I32, ValType::I32)
			| (wp::ValType::I64, ValType::I64)
			| (wp::ValType::F32, ValType::F32)
			| (wp::ValType::F64, ValType::F64)
			| (wp::ValType::V128, ValType::V128) => true,
			(wp::ValType::Ref(parsed), ValType::Ref(ty)) => {
				parsed.is_nullable() == ty.is_nullable()
					&& self.match_heap_type(parsed.heap_type(), ty.heap_type(), group)
			}
			_ => false,
		}
	}

	fn match_heap_type(&mut self, parsed: wp::HeapType, ty: &HeapType, group: u32) -> bool {
		let index = match parsed {
			wp::HeapType::Abstract { ty: kind, .. } => return matches_abstract_heap(kind, ty),
			wp::HeapType::Concrete(index) | wp::HeapType::Exact(index) => resolve_index(index, group),
		};
		match (index, ty) {
			(Some(idx), HeapType::ConcreteStruct(s)) => self.match_struct(idx, s),
			(Some(idx), HeapType::ConcreteArray(a)) => self.match_array(idx, a),
			(Some(idx), HeapType::ConcreteFunc(f)) => self.match_func(idx, f),
			_ => false,
		}
	}
}

fn matches_abstract_heap(kind: wp::AbstractHeapType, ty: &HeapType) -> bool {
	use wp::AbstractHeapType::*;
	matches!(
		(kind, ty),
		(Extern, HeapType::Extern)
			| (NoExtern, HeapType::NoExtern)
			| (Func, HeapType::Func)
			| (NoFunc, HeapType::NoFunc)
			| (Any, HeapType::Any)
			| (None, HeapType::None)
			| (Eq, HeapType::Eq)
			| (Struct, HeapType::Struct)
			| (Array, HeapType::Array)
			| (I31, HeapType::I31)
	)
}

enum RuntimeTypeId {
	Struct(StructType),
	Array(ArrayType),
	Func(FuncType),
}

impl RuntimeTypeId {
	fn matches_struct(&self, ty: &StructType) -> bool {
		matches!(self, RuntimeTypeId::Struct(existing) if StructType::eq(existing, ty))
	}

	fn matches_array(&self, ty: &ArrayType) -> bool {
		matches!(self, RuntimeTypeId::Array(existing) if ArrayType::eq(existing, ty))
	}

	fn matches_func(&self, ty: &FuncType) -> bool {
		matches!(self, RuntimeTypeId::Func(existing) if FuncType::eq(existing, ty))
	}
}
