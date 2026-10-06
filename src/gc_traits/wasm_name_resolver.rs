//! Field names of WASM GC structs, read from a module's name section

use super::*;
use std::sync::LazyLock;
use wasmparser as wp;


static REGISTRY: LazyLock<Mutex<FieldNameRegistry>> =
    LazyLock::new(|| Mutex::new(FieldNameRegistry::default()));

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
        mapping
            .index_of(field_name)
            .ok_or_else(|| unknown_field_error(field_name, &mapping))
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

        Err(anyhow!(
            "Failed to resolve struct fields from registered modules"
        ))
    }
}

fn unknown_field_error(field_name: &str, mapping: &StructFieldMapping) -> anyhow::Error {
    let mut available: Vec<&str> = mapping.available_names();
    available.sort_unstable();
    anyhow!(
        "Unknown field '{}'. Available fields: {}",
        field_name,
        if available.is_empty() {
            "<none>".to_string()
        } else {
            available.join(", ")
        }
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
	fn new(parsed: &ParsedStructType) -> Self {
		let mut lookup = HashMap::new();
		for (idx, name) in parsed.field_names.iter().enumerate() {
			if let Some(name) = name {
				lookup.insert(name.clone(), idx);
			}
		}
		Self {
			type_name: parsed.type_name.clone(),
			names: parsed.field_names.clone(),
			lookup,
		}
	}

	fn index_of(&self, name: &str) -> Option<usize> {
		self.lookup.get(name).copied()
	}

	fn available_names(&self) -> Vec<&str> {
		self.names.iter().filter_map(|n| n.as_deref()).collect()
	}
}

struct ParsedModule {
	hash: u64,
	types: Vec<ParsedTypeInfo>,
}

impl ParsedModule {
	fn parse(bytes: &[u8], hash: u64) -> Result<Self> {
		let mut types = Vec::new();
		let mut next_index: u32 = 0;
		let mut field_names: HashMap<u32, Vec<(u32, String)>> = HashMap::new();
		let mut type_names: HashMap<u32, String> = HashMap::new();

		for payload in wp::Parser::new(0).parse_all(bytes) {
			match payload? {
				wp::Payload::TypeSection(reader) => {
					next_index = Self::read_type_section(reader, next_index, &mut types)?;
				}
				wp::Payload::CustomSection(section) => {
					if let wp::KnownCustom::Name(name_reader) = section.as_known() {
						Self::read_name_section(name_reader, &mut field_names, &mut type_names)?;
					}
				}
				_ => {}
			}
		}

		let mut module = Self { hash, types };
		module.apply_names(field_names, type_names);
		Ok(module)
	}

	fn read_type_section(
		reader: wp::TypeSectionReader,
		mut next_index: u32,
		types: &mut Vec<ParsedTypeInfo>,
	) -> Result<u32> {
		for group in reader {
			let group = group?;
			let group_start = next_index;
			for subtype in group.into_types() {
				let actual_index = next_index;
				let info = ParsedTypeInfo::from_subtype(actual_index, subtype, group_start)?;
				types.push(info);
				next_index = actual_index + 1;
			}
		}
		Ok(next_index)
	}

	fn read_name_section(
		reader: wp::NameSectionReader<'_>,
		field_names: &mut HashMap<u32, Vec<(u32, String)>>,
		type_names: &mut HashMap<u32, String>,
	) -> Result<()> {
		for entry in reader {
			match entry? {
				wp::Name::Type(map) => {
					for naming in map {
						let naming = naming?;
						type_names.insert(naming.index, naming.name.to_string());
					}
				}
				wp::Name::Field(map) => {
					for naming in map {
						let naming = naming?;
						let type_index = naming.index;
						let mut names = Vec::new();
						for field in naming.names {
							let field = field?;
							names.push((field.index, field.name.to_string()));
						}
						if !names.is_empty() {
							field_names.entry(type_index).or_default().extend(names);
						}
					}
				}
				_ => {}
			}
		}
		Ok(())
	}

	fn apply_names(&mut self, field_names: HashMap<u32, Vec<(u32, String)>>, type_names: HashMap<u32, String>) {
		// Apply type names
		for (type_index, name) in type_names {
			if let Some(ParsedTypeInfo::Struct(struct_ty)) = self.types.get_mut(type_index as usize) {
				struct_ty.type_name = Some(name);
			}
		}
		// Apply field names
		for (type_index, entries) in field_names {
			if let Some(ParsedTypeInfo::Struct(struct_ty)) =
				self.types.get_mut(type_index as usize)
			{
				for (field_idx, name) in entries {
					if let Some(slot) = struct_ty.field_names.get_mut(field_idx as usize) {
						*slot = Some(name);
					}
				}
			}
		}
	}

	fn try_match(&self, struct_type: &StructType) -> Option<StructFieldMapping> {
		for (idx, ty) in self.types.iter().enumerate() {
			if let ParsedTypeInfo::Struct(parsed_struct) = ty {
				let mut ctx = MatchContext::new(self);
				if ctx.match_struct(idx as u32, struct_type) {
					return Some(StructFieldMapping::new(parsed_struct));
				}
			}
		}
		None
	}

	fn parsed_type(&self, idx: u32) -> Option<&ParsedTypeInfo> {
		self.types.get(idx as usize)
	}
}

enum ParsedTypeInfo {
	Struct(ParsedStructType),
	Array(ParsedArrayType),
	Func(ParsedFuncType),
	Other,
}

impl ParsedTypeInfo {
	fn from_subtype(
		type_index: u32,
		subtype: wp::SubType,
		group_start: u32,
	) -> Result<Self> {
		use wp::CompositeInnerType::*;
		match subtype.composite_type.inner {
			Struct(ty) => Ok(Self::Struct(ParsedStructType::from_parser(
				type_index,
				ty,
				group_start,
			)?)),
			Array(ty) => Ok(Self::Array(ParsedArrayType::from_parser(
				type_index,
				ty,
				group_start,
			)?)),
			Func(ty) => Ok(Self::Func(ParsedFuncType::from_parser(
				type_index,
				ty,
				group_start,
			)?)),
			_ => Ok(Self::Other),
		}
	}
}

#[derive(Clone)]
struct ParsedStructType {
	#[allow(dead_code)]
	type_index: u32,
	type_name: Option<String>,
	fields: Vec<ParsedField>,
	field_names: Vec<Option<String>>,
}

impl ParsedStructType {
	fn from_parser(type_index: u32, ty: wp::StructType, group_start: u32) -> Result<Self> {
		let mut fields = Vec::with_capacity(ty.fields.len());
		for field in ty.fields.iter() {
			fields.push(ParsedField::from_parser(field, group_start)?);
		}
		let field_names = vec![None; fields.len()];
		Ok(Self {
			type_index,
			type_name: None,
			fields,
			field_names,
		})
	}
}

#[derive(Clone)]
struct ParsedArrayType {
	#[allow(dead_code)]
	type_index: u32,
	field: ParsedField,
}

impl ParsedArrayType {
	fn from_parser(type_index: u32, ty: wp::ArrayType, group_start: u32) -> Result<Self> {
		Ok(Self {
			type_index,
			field: ParsedField::from_parser(&ty.0, group_start)?,
		})
	}
}

#[derive(Clone)]
struct ParsedFuncType {
	#[allow(dead_code)]
	type_index: u32,
	params: Vec<ParsedValType>,
	results: Vec<ParsedValType>,
}

impl ParsedFuncType {
	fn from_parser(type_index: u32, ty: wp::FuncType, group_start: u32) -> Result<Self> {
		let params = ty
			.params()
			.iter()
			.map(|p| ParsedValType::from_parser(p, group_start))
			.collect::<Result<Vec<_>>>()?;
		let results = ty
			.results()
			.iter()
			.map(|r| ParsedValType::from_parser(r, group_start))
			.collect::<Result<Vec<_>>>()?;
		Ok(Self {
			type_index,
			params,
			results,
		})
	}
}

#[derive(Clone)]
struct ParsedField {
	mutable: bool,
	storage: ParsedStorageType,
}

impl ParsedField {
	fn from_parser(field: &wp::FieldType, group_start: u32) -> Result<Self> {
		Ok(Self {
			mutable: field.mutable,
			storage: ParsedStorageType::from_parser(&field.element_type, group_start)?,
		})
	}
}

#[derive(Clone)]
enum ParsedStorageType {
	I8,
	I16,
	Val(ParsedValType),
}

impl ParsedStorageType {
	fn from_parser(ty: &wp::StorageType, group_start: u32) -> Result<Self> {
		Ok(match ty {
			wp::StorageType::I8 => ParsedStorageType::I8,
			wp::StorageType::I16 => ParsedStorageType::I16,
			wp::StorageType::Val(v) => {
				ParsedStorageType::Val(ParsedValType::from_parser(v, group_start)?)
			}
		})
	}
}

#[derive(Clone)]
enum ParsedValType {
	I32,
	I64,
	F32,
	F64,
	V128,
	Ref(ParsedRefType),
}

impl ParsedValType {
	fn from_parser(ty: &wp::ValType, group_start: u32) -> Result<Self> {
		use wp::ValType::*;
		Ok(match ty {
			I32 => ParsedValType::I32,
			I64 => ParsedValType::I64,
			F32 => ParsedValType::F32,
			F64 => ParsedValType::F64,
			V128 => ParsedValType::V128,
			Ref(r) => ParsedValType::Ref(ParsedRefType::from_parser(r, group_start)?),
		})
	}
}

#[derive(Clone)]
struct ParsedRefType {
	nullable: bool,
	heap: ParsedHeapType,
}

impl ParsedRefType {
	fn from_parser(ty: &wp::RefType, group_start: u32) -> Result<Self> {
		Ok(Self {
			nullable: ty.is_nullable(),
			heap: ParsedHeapType::from_parser(&ty.heap_type(), group_start)?,
		})
	}
}

#[derive(Clone)]
enum ParsedHeapType {
	Abstract {
		#[allow(dead_code)]
		shared: bool,
		kind: ParsedAbstractHeapType,
	},
	Concrete(u32),
}

impl ParsedHeapType {
	fn from_parser(ty: &wp::HeapType, group_start: u32) -> Result<Self> {
		Ok(match ty {
			wp::HeapType::Abstract { shared, ty } => ParsedHeapType::Abstract {
				shared: *shared,
				kind: ParsedAbstractHeapType::from_parser(*ty),
			},
			wp::HeapType::Concrete(idx) => {
				let resolved = resolve_index(*idx, group_start)?;
				ParsedHeapType::Concrete(resolved)
			}
			wp::HeapType::Exact(idx) => {
				let resolved = resolve_index(*idx, group_start)?;
				ParsedHeapType::Concrete(resolved)
			}
		})
	}
}

#[derive(Clone, Copy)]
enum ParsedAbstractHeapType {
	Func,
	Extern,
	Any,
	None,
	NoExtern,
	NoFunc,
	Eq,
	Struct,
	Array,
	I31,
	#[allow(dead_code)]
	Exn,
	#[allow(dead_code)]
	NoExn,
	#[allow(dead_code)]
	Cont,
	#[allow(dead_code)]
	NoCont,
}

impl ParsedAbstractHeapType {
	fn from_parser(ty: wp::AbstractHeapType) -> Self {
		use wp::AbstractHeapType::*;
		match ty {
			Func => ParsedAbstractHeapType::Func,
			Extern => ParsedAbstractHeapType::Extern,
			Any => ParsedAbstractHeapType::Any,
			None => ParsedAbstractHeapType::None,
			NoExtern => ParsedAbstractHeapType::NoExtern,
			NoFunc => ParsedAbstractHeapType::NoFunc,
			Eq => ParsedAbstractHeapType::Eq,
			Struct => ParsedAbstractHeapType::Struct,
			Array => ParsedAbstractHeapType::Array,
			I31 => ParsedAbstractHeapType::I31,
			Exn => ParsedAbstractHeapType::Exn,
			NoExn => ParsedAbstractHeapType::NoExn,
			Cont => ParsedAbstractHeapType::Cont,
			NoCont => ParsedAbstractHeapType::NoCont,
		}
	}
}

fn resolve_index(index: wp::UnpackedIndex, group_start: u32) -> Result<u32> {
	Ok(match index {
		wp::UnpackedIndex::Module(i) => i,
		wp::UnpackedIndex::RecGroup(i) => group_start + i,
		_ => bail!("unsupported canonical type index"),
	})
}

struct MatchContext<'a> {
	module: &'a ParsedModule,
	bindings: HashMap<u32, RuntimeTypeId>,
}

impl<'a> MatchContext<'a> {
	fn new(module: &'a ParsedModule) -> Self {
		Self {
			module,
			bindings: HashMap::new(),
		}
	}

	fn match_struct(&mut self, idx: u32, ty: &StructType) -> bool {
		if let Some(existing) = self.bindings.get(&idx) {
			return existing.matches_struct(ty);
		}
		let parsed = match self.module.parsed_type(idx) {
			Some(ParsedTypeInfo::Struct(s)) => s,
			_ => return false,
		};
		self.bindings.insert(idx, RuntimeTypeId::Struct(ty.clone()));
		self.compare_struct(parsed, ty)
	}

	fn compare_struct(&mut self, parsed: &ParsedStructType, ty: &StructType) -> bool {
		let mut runtime_fields = ty.fields();
		if parsed.fields.len() != runtime_fields.len() {
			return false;
		}
		for (parsed_field, runtime_field) in parsed.fields.iter().zip(runtime_fields.by_ref())
		{
			if parsed_field.mutable != runtime_field.mutability().is_var() {
				return false;
			}
			if !self.match_storage_type(&parsed_field.storage, runtime_field.element_type()) {
				return false;
			}
		}
		true
	}

	fn match_array(&mut self, idx: u32, ty: &ArrayType) -> bool {
		if let Some(existing) = self.bindings.get(&idx) {
			return existing.matches_array(ty);
		}
		let parsed = match self.module.parsed_type(idx) {
			Some(ParsedTypeInfo::Array(a)) => a,
			_ => return false,
		};
		self.bindings.insert(idx, RuntimeTypeId::Array(ty.clone()));
		let runtime_field = ty.field_type();
		parsed.field.mutable == runtime_field.mutability().is_var()
			&& self.match_storage_type(&parsed.field.storage, runtime_field.element_type())
	}

	fn match_func(&mut self, idx: u32, ty: &FuncType) -> bool {
		if let Some(existing) = self.bindings.get(&idx) {
			return existing.matches_func(ty);
		}
		let parsed = match self.module.parsed_type(idx) {
			Some(ParsedTypeInfo::Func(f)) => f,
			_ => return false,
		};
		self.bindings.insert(idx, RuntimeTypeId::Func(ty.clone()));
		if parsed.params.len() != ty.params().len()
			|| parsed.results.len() != ty.results().len()
		{
			return false;
		}
		for (parsed_val, runtime_val) in parsed.params.iter().zip(ty.params()) {
			if !self.match_val_type(parsed_val, &runtime_val) {
				return false;
			}
		}
		for (parsed_val, runtime_val) in parsed.results.iter().zip(ty.results()) {
			if !self.match_val_type(parsed_val, &runtime_val) {
				return false;
			}
		}
		true
	}

	fn match_storage_type(&mut self, parsed: &ParsedStorageType, ty: &StorageType) -> bool {
		match (parsed, ty) {
			(ParsedStorageType::I8, StorageType::I8)
			| (ParsedStorageType::I16, StorageType::I16) => true,
			(ParsedStorageType::Val(p), StorageType::ValType(v)) => self.match_val_type(p, v),
			_ => false,
		}
	}

	fn match_val_type(&mut self, parsed: &ParsedValType, ty: &ValType) -> bool {
		use ParsedValType::*;
		match (parsed, ty) {
			(I32, ValType::I32)
			| (I64, ValType::I64)
			| (F32, ValType::F32)
			| (F64, ValType::F64)
			| (V128, ValType::V128) => true,
			(Ref(parsed_ref), ValType::Ref(runtime_ref)) => {
				if parsed_ref.nullable != runtime_ref.is_nullable() {
					return false;
				}
				self.match_heap_type(&parsed_ref.heap, runtime_ref.heap_type())
			}
			_ => false,
		}
	}

	fn match_heap_type(&mut self, parsed: &ParsedHeapType, ty: &HeapType) -> bool {
		match (parsed, ty) {
			(ParsedHeapType::Abstract { kind, .. }, _) => matches_abstract_heap(*kind, ty),
			(ParsedHeapType::Concrete(idx), HeapType::ConcreteStruct(s)) => {
				self.match_struct(*idx, s)
			}
			(ParsedHeapType::Concrete(idx), HeapType::ConcreteArray(a)) => {
				self.match_array(*idx, a)
			}
			(ParsedHeapType::Concrete(idx), HeapType::ConcreteFunc(f)) => {
				self.match_func(*idx, f)
			}
			_ => false,
		}
	}
}

fn matches_abstract_heap(kind: ParsedAbstractHeapType, ty: &HeapType) -> bool {
	use ParsedAbstractHeapType::*;
	matches!(
    (kind, ty),
    (Extern,   HeapType::Extern)
    | (NoExtern, HeapType::NoExtern)
    | (Func,     HeapType::Func)
    | (NoFunc,   HeapType::NoFunc)
    | (Any,      HeapType::Any)
    | (None,     HeapType::None)
    | (Eq,       HeapType::Eq)
    | (Struct,   HeapType::Struct)
    | (Array,    HeapType::Array)
    | (I31,      HeapType::I31)
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
