//! What a WebAssembly module exports, read once per path: its functions and globals, typed by its own types and the C header beside it
use super::*;

/// What an import from a module does: call its function, read its global (a getter of no parameters) or set it
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Role {
	Function,
	Global { mutable: bool },
	Setter,
}

#[derive(Clone, Debug)]
pub struct Export {
	pub signature: FfiSignature,
	pub role: Role,
	/// a function's parameter names from the module's name section (`$from`), else its header's, else `$0`, `$1` …
	pub parameters: Vec<String>,
	/// how each parameter of the module's function crosses, by the header beside the module (all numbers without one)
	pub c_parameters: Vec<CParameter>,
	pub c_result: CResult,
	/// a function of a module warp compiled taking or giving values (its `(ref $Node)` parameters and result): the kind of
	/// its result by its warp.meta signature; each value is copied between the program's instance and the module's
	pub node_result: Option<crate::type_kinds::Kind>,
}

/// How a parameter of a C function compiled to wasm crosses (the header beside the module, notes/wasm_modules.md)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CParameter {
	/// a number, or a pointer the program got from the module (`struct x *`): its own wasm value
	Number,
	/// `char *` or a const pointer to bytes (zlib's `const Bytef *buf`): the program's text copied into the module's malloc
	Text,
	/// the unsigned count after a text (`const void *input, size_t length`): the text's byte length, which the warp call
	/// may leave out (`XXH32("hello", 0)`), then the text crosses by its bytes, NUL bytes included
	TextLength,
	/// `T **`: left out of the warp call, the module gets a NULL-initialised slot of its malloc
	Out,
	/// a writable buffer followed by its in/out length (`Bytef *dest, uLongf *destLen`): the warp call passes its
	/// capacity, the module gets a block of that size
	Buffer,
	/// the length slot of the buffer before it: left out of the warp call, holds the capacity going in, the length written
	/// coming out
	BufferLength,
}

impl CParameter {
	/// Does the warp call pass a value for it (the program's import has its parameter)
	pub(super) fn is_passed(self) -> bool {
		!matches!(self, CParameter::Out | CParameter::BufferLength)
	}
}

/// What the result of a C function compiled to wasm crosses as
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CResult {
	Number,
	/// an unsigned 32-bit C result (`unsigned long` in wasm32): zero-extended into an Int, never negative
	Unsigned,
	/// `char *`: the NUL-terminated text in the module's memory, read back as a warp text, NULL is ø
	Text,
	/// what the first out-pointer received (as natively, notes/ffi_handles.md): a text for `char **`, else the module's
	/// pointer as a number; NULL there is a loud error naming the C status
	Out {
		text: bool,
	},
	/// the bytes the first buffer received (its length slot says how many) as a warp text; a C status other than 0 is a
	/// loud error (zlib's Z_BUF_ERROR when the capacity is too small)
	Buffer,
}

impl Export {
	/// Does a call need the module's memory: a text or an out-pointer crosses, or the result changes type
	pub fn crosses_c_values(&self) -> bool {
		self.c_result != CResult::Number
			|| self
				.c_parameters
				.iter()
				.any(|parameter| *parameter != CParameter::Number)
	}
}

/// Does an import name a WebAssembly module file (`lib/x.wasm`, `x.wat`) rather than a C library
pub fn is_module_path(name: &str) -> bool {
	Path::new(name)
		.extension()
		.and_then(|extension| extension.to_str())
		.is_some_and(|extension| MODULE_EXTENSIONS.contains(&extension))
}

/// The binary of a module file; a `.wat` file is its text form
pub fn module_bytes(path: &str) -> Result<Vec<u8>, String> {
	let bytes = crate::web::read_bytes(path).ok_or_else(|| format!("cannot read the module {path}"))?;
	if !path.ends_with(".wat") {
		return Ok(bytes);
	}
	#[cfg(feature = "native")]
	return wat::parse_bytes(&bytes)
		.map(|binary| binary.into_owned())
		.map_err(|failure| format!("{path}: {failure}"));
	#[cfg(not(feature = "native"))]
	Err(format!("{path}: a module in WAT text needs the native build"))
}

/// The functions and resource types the component at `path` exports, also those of its exported interfaces, in warp's
/// spelling (`stats_of` for `stats-of`), sorted; constructors and methods are its resources' own. Read with wasmparser's
/// validator, so the page's compiler answers `lib.exports` too (card reflect-page-meta)
pub fn component_export_names(path: &str) -> Result<Vec<String>, String> {
	use wasmparser::component_types::{ComponentAnyTypeId, ComponentEntityType};
	let bytes = module_bytes(path)?;
	let failure = |failure: wasmparser::BinaryReaderError| format!("cannot load the component {path}: {failure}");
	let types = wasmparser::Validator::new_with_features(wasmparser::WasmFeatures::all())
		.validate_all(&bytes)
		.map_err(failure)?;
	let types = types.as_ref();
	let named = |name: &str, entity: &ComponentEntityType| {
		let exported = matches!(
			entity,
			ComponentEntityType::Func(_)
				| ComponentEntityType::Type {
					created: ComponentAnyTypeId::Resource(_),
					..
				}
		);
		(exported && !name.starts_with('[')).then(|| name.replace('-', "_"))
	};
	let mut names = vec![];
	for name in component_export_section_names(&bytes).map_err(failure)? {
		match types.component_item_for_export(&name).map(|item| &item.ty) {
			Some(ComponentEntityType::Instance(interface)) => names.extend(
				types[*interface]
					.exports
					.iter()
					.filter_map(|(inner, item)| named(inner, &item.ty)),
			),
			Some(entity) => names.extend(named(&name, entity)),
			None => {}
		}
	}
	names.sort();
	Ok(names)
}

/// The names in the export sections of the outermost component, not those of the components and modules it nests
pub(crate) fn component_export_section_names(bytes: &[u8]) -> Result<Vec<String>, wasmparser::BinaryReaderError> {
	let mut names = vec![];
	let mut depth = 0;
	for payload in wasmparser::Parser::new(0).parse_all(bytes) {
		match payload? {
			wasmparser::Payload::Version { .. } => depth += 1,
			wasmparser::Payload::End(_) => depth -= 1,
			wasmparser::Payload::ComponentExportSection(reader) if depth == 1 => {
				for export in reader {
					names.push(export?.name.name.to_string());
				}
			}
			_ => {}
		}
	}
	Ok(names)
}

/// The exports of the module at `path` the program can use, by name; read once per path and process
pub fn exports(path: &str) -> &'static HashMap<String, Export> {
	static READ: std::sync::Mutex<Vec<(String, &'static HashMap<String, Export>)>> = std::sync::Mutex::new(Vec::new());
	let mut read = READ.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	if let Some((_, exports)) = read.iter().find(|(read_path, _)| read_path == path) {
		return exports;
	}
	let found = module_bytes(path)
		.and_then(|bytes| read_exports(path, &bytes))
		.unwrap_or_else(|failure| {
			eprintln!("[wasm] {failure}");
			HashMap::new()
		});
	let exports: &'static HashMap<String, Export> = Box::leak(Box::new(found));
	read.push((path.to_string(), exports));
	exports
}

/// The functions and globals a module exports with number types (i32, i64, f32, f64); others are not callable from warp
pub(crate) fn read_exports(path: &str, bytes: &[u8]) -> Result<HashMap<String, Export>, String> {
	use wasmparser::{ExternalKind, Parser, Payload, TypeRef};
	let failure = |error: wasmparser::BinaryReaderError| format!("{path}: {error}");
	let mut types = Vec::new();
	let mut function_types = Vec::new(); // the type of every function, imported ones first
	let mut global_types = Vec::new();
	let mut exported = Vec::new();
	let mut local_names: HashMap<u32, HashMap<u32, String>> = HashMap::new(); // by function index, by local index
	for payload in Parser::new(0).parse_all(bytes) {
		match payload.map_err(failure)? {
			Payload::TypeSection(reader) => {
				for group in reader {
					// a warp module's GC struct and array types too: only function types are of exports warp calls
					types.extend(group.map_err(failure)?.into_types().map(
						|sub_type| match sub_type.composite_type.inner {
							wasmparser::CompositeInnerType::Func(function_type) => Some(function_type),
							_ => None,
						},
					));
				}
			}
			Payload::ImportSection(reader) => {
				for import in reader.into_imports() {
					match import.map_err(failure)?.ty {
						TypeRef::Func(index) => function_types.push(index),
						TypeRef::Global(global) => global_types.push(global),
						_ => {}
					}
				}
			}
			Payload::FunctionSection(reader) => {
				for index in reader {
					function_types.push(index.map_err(failure)?);
				}
			}
			Payload::GlobalSection(reader) => {
				for global in reader {
					global_types.push(global.map_err(failure)?.ty);
				}
			}
			Payload::CustomSection(section) => {
				if let wasmparser::KnownCustom::Name(reader) = section.as_known() {
					for entry in reader {
						let wasmparser::Name::Local(functions) = entry.map_err(failure)? else {
							continue;
						};
						for function in functions {
							let function = function.map_err(failure)?;
							let names = function
								.names
								.into_iter()
								.filter_map(Result::ok)
								.map(|naming| (naming.index, naming.name.to_string()));
							local_names.insert(function.index, names.collect());
						}
					}
				}
			}
			Payload::ExportSection(reader) => {
				for export in reader {
					let export = export.map_err(failure)?;
					exported.push((export.name.to_string(), export.kind, export.index as usize));
				}
			}
			_ => {}
		}
	}
	// a warp module names its functions' parameters in warp.meta, other modules may in the name section
	let functions_meta = crate::formats::meta_section::entry(bytes, crate::lowering::reflection::META_FUNCTIONS);
	let meta_parameters = |name: &str| {
		functions_meta.as_ref().map(|functions| {
			crate::lowering::reflection::entry_names(&functions[name][crate::lowering::reflection::PARAMS_WORDS[0]])
		})
	};
	let library: &'static str = Box::leak(path.to_string().into_boxed_str());
	let mut exports = HashMap::new();
	let mut add = |name: String,
	               params: Vec<wasm_encoder::ValType>,
	               results: Vec<wasm_encoder::ValType>,
	               role: Role,
	               parameters: Vec<String>,
	               node_result: Option<crate::type_kinds::Kind>| {
		let import_name: &'static str = Box::leak(name.clone().into_boxed_str());
		exports.insert(
			name,
			Export {
				signature: FfiSignature::new(import_name, library, params, results),
				role,
				parameters,
				c_parameters: vec![],
				c_result: CResult::Number,
				node_result,
			},
		);
	};
	for (name, kind, index) in exported {
		match kind {
			ExternalKind::Func => {
				let Some(function_type) = function_types
					.get(index)
					.and_then(|type_index| types.get(*type_index as usize))
					.and_then(Option::as_ref)
				else {
					continue;
				};
				let node_result = functions_meta
					.as_ref()
					.and_then(|functions| node_result(&functions[name.as_str()]));
				let value_types =
					|types: &[wasmparser::ValType]| number_types(types).or_else(|| node_result.and(node_types(types)));
				if let (Some(params), Some(results)) = (
					value_types(function_type.params()),
					value_types(function_type.results()),
				) {
					let names = local_names.get(&(index as u32));
					let named = meta_parameters(&name).filter(|names| names.len() == params.len());
					let parameters = named.unwrap_or_else(|| {
						(0..params.len() as u32)
							.map(|local| {
								names
									.and_then(|names| names.get(&local))
									.cloned()
									.unwrap_or_else(|| format!("${local}"))
							})
							.collect()
					});
					let crosses_nodes = number_types(function_type.params()).is_none()
						|| number_types(function_type.results()).is_none();
					add(
						name,
						params,
						results,
						Role::Function,
						parameters,
						node_result.filter(|_| crosses_nodes),
					);
				}
			}
			ExternalKind::Global => {
				let Some(global) = global_types.get(index) else {
					continue;
				};
				let Some(value_type) = number_types(&[global.content_type]) else {
					continue;
				};
				if global.mutable {
					add(
						format!("{SETTER_PREFIX}{name}"),
						value_type.clone(),
						value_type.clone(),
						Role::Setter,
						vec![name.clone()],
						None,
					);
				}
				add(
					name,
					vec![],
					value_type,
					Role::Global {
						mutable: global.mutable,
					},
					vec![],
					None,
				);
			}
			_ => {}
		}
	}
	with_header_types(path, &mut exports);
	Ok(exports)
}

/// A C library compiled to WebAssembly: the header beside it (`zlib.h` of `zlib.wasm`) says how its functions' parameters
/// and results cross (CParameter, CResult) and names the parameters; the module's own number types win (`size_t` is 32
/// bits in wasm32), pointers to structs stay the module's numbers
pub(crate) fn with_header_types(path: &str, exports: &mut HashMap<String, Export>) {
	use crate::ffi::{parse_header_file, pointer_kind, CPointer};
	let header = Path::new(path).with_extension("h");
	let Some(header) = header.to_str().filter(|header| crate::web::file_exists(header)) else {
		return;
	};
	for declared in parse_header_file(header, path) {
		let Some(export) = exports
			.get_mut(&declared.name)
			.filter(|export| export.role == Role::Function)
		else {
			continue;
		};
		let c_parameters = c_parameters(&declared.param_types);
		let first_out = declared
			.param_types
			.iter()
			.find(|c_type| pointer_kind(c_type) == Some(CPointer::Out));
		let c_result = match pointer_kind(&declared.return_type) {
			Some(CPointer::Text) => CResult::Text,
			None if c_parameters.contains(&CParameter::Buffer) => CResult::Buffer,
			None if first_out.is_some() => CResult::Out {
				text: first_out.is_some_and(|c_type| c_type.matches('*').count() == 2 && c_type.contains("char")),
			},
			None if declared.return_type.contains("unsigned")
				&& export.signature.results == [wasm_encoder::ValType::I32] =>
			{
				CResult::Unsigned
			}
			_ => CResult::Number,
		};
		if c_parameters.len() != export.signature.params.len()
			|| c_result != CResult::Number && export.signature.results.len() != 1
		{
			eprintln!(
				"[wasm] {header}: {} does not match the module's {} parameters",
				declared.raw.trim(),
				export.signature.params.len()
			);
			continue;
		}
		let warp_parameter = |index: &usize| c_parameters[*index].is_passed();
		if export.parameters.iter().all(|name| name.starts_with('$'))
			&& declared.param_names.len() == c_parameters.len()
		{
			export.parameters = declared.param_names.clone();
		}
		export.parameters = (0..c_parameters.len())
			.filter(warp_parameter)
			.map(|index| export.parameters[index].clone())
			.collect();
		export.signature.params = (0..c_parameters.len())
			.filter(warp_parameter)
			.map(|index| export.signature.params[index])
			.collect();
		export.signature.results = match c_result {
			CResult::Text | CResult::Out { text: true } | CResult::Buffer => {
				vec![wasm_encoder::ValType::Ref(wasm_encoder::RefType::ANYREF)]
			} // the text Node the host builds
			CResult::Unsigned => vec![wasm_encoder::ValType::I64],
			CResult::Number | CResult::Out { text: false } => export.signature.results.clone(),
		};
		export.c_parameters = c_parameters;
		export.c_result = c_result;
	}
}

/// How the parameters of these C types cross into a module (CParameter): a writable byte pointer before a pointer to an
/// unsigned count is a buffer with its in/out length, `char *` and const pointers to anything but a struct carry the
/// program's text (with its length when an unsigned count follows), `T **` is an out-pointer, the rest are numbers
pub(crate) fn c_parameters(c_types: &[String]) -> Vec<CParameter> {
	use crate::ffi::{pointer_kind, CPointer};
	let is_const = |c_type: &str| c_type.split_whitespace().any(|word| word == "const");
	let mut parameters: Vec<CParameter> = Vec::with_capacity(c_types.len());
	for (index, c_type) in c_types.iter().enumerate() {
		let after = |kind: CParameter| parameters.last() == Some(&kind);
		let next_counts = c_types.get(index + 1).is_some_and(|next| is_count_pointer(next));
		parameters.push(match pointer_kind(c_type) {
			Some(CPointer::Text | CPointer::Memory) if !is_const(c_type) && next_counts => CParameter::Buffer,
			_ if after(CParameter::Buffer) => CParameter::BufferLength,
			Some(CPointer::Text) => CParameter::Text,
			Some(CPointer::Memory) if is_const(c_type) => CParameter::Text,
			Some(CPointer::Out) => CParameter::Out,
			None if after(CParameter::Text) && is_count(c_type) => CParameter::TextLength,
			_ => CParameter::Number,
		});
	}
	parameters
}

/// An unsigned integer type, as C counts bytes: `size_t`, `unsigned long`, `uint32_t`
pub(crate) fn is_count(c_type: &str) -> bool {
	let words: Vec<&str> = c_type.split_whitespace().filter(|word| *word != "const").collect();
	!c_type.contains('*')
		&& words
			.first()
			.is_some_and(|first| *first == "unsigned" || *first == "size_t" || first.starts_with("uint"))
}

/// A writable pointer to an unsigned count (`uLongf *destLen` spelled `unsigned long *destLen`)
pub(crate) fn is_count_pointer(c_type: &str) -> bool {
	c_type.matches('*').count() == 1 && !c_type.contains("const") && is_count(&c_type.replace('*', " "))
}

/// The result kind of a function a warp.meta `functions` entry describes: its signature's `-> type`, any value else
pub(crate) fn node_result(function: &Node) -> Option<crate::type_kinds::Kind> {
	let Node::Text(signature) = function["signature"].drop_meta() else {
		return None;
	};
	let result = signature
		.rsplit_once("-> ")
		.map(|(_, result)| result.trim())
		.unwrap_or_default();
	Some(crate::analyzer::type_word_kind(result).unwrap_or(crate::type_kinds::Kind::Empty))
}

/// The types of a warp function's parameters or results: its numbers, a value (a Node of the module) as anyref
pub(crate) fn node_types(types: &[wasmparser::ValType]) -> Option<Vec<wasm_encoder::ValType>> {
	types
		.iter()
		.map(|value_type| match value_type {
			wasmparser::ValType::Ref(_) => Some(wasm_encoder::ValType::Ref(wasm_encoder::RefType::ANYREF)),
			number => number_types(std::slice::from_ref(number)).map(|mut types| types.remove(0)),
		})
		.collect()
}

pub(crate) fn number_types(types: &[wasmparser::ValType]) -> Option<Vec<wasm_encoder::ValType>> {
	types
		.iter()
		.map(|value_type| match value_type {
			wasmparser::ValType::I32 => Some(wasm_encoder::ValType::I32),
			wasmparser::ValType::I64 => Some(wasm_encoder::ValType::I64),
			wasmparser::ValType::F32 => Some(wasm_encoder::ValType::F32),
			wasmparser::ValType::F64 => Some(wasm_encoder::ValType::F64),
			_ => None,
		})
		.collect()
}
