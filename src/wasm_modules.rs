//! `import fourty_two` of a WebAssembly core module (`fourty_two.wasm`, or its text `fourty_two.wat`): its exported
//! functions are called like C functions, with the parameter and result types the module declares, and an exported
//! global reads as its value. The program imports them from the module's path (import module name), the runner links
//! each import to the module instantiated once per run (notes/wasm_modules.md).
use crate::ffi::FfiSignature;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashMap;
use std::path::Path;

pub const MODULE_EXTENSIONS: [&str; 2] = ["wasm", "wat"];

/// The import that sets a mutable global g of a module: `set g` (P140), a host function of the value, giving it back
const SETTER_PREFIX: &str = "set ";
/// The exports `include m` runs, the first one m has (P139)
const ENTRY_POINTS: [&str; 2] = ["main", "_start"];

/// The custom section describing the program's C calls for a host that cannot see an import's types
/// (web/playground/host.js, which calls libc.wasm for `c` and imported modules): a line `module\tname\tparameters\tresult`
/// per import. Parameter letters: `t` a text (a NUL-terminated copy in the program's memory), `l` the length of the text
/// before it (strcmp's pairs, then not NUL-terminated), `s` that length passed on to the module too, `n` a number, `o`
/// an out-pointer, `b` a buffer of the capacity given, `k` its length slot (`o` and `k` take no value of the program);
/// result: `t` a text Node built by the host, `u` an unsigned number, `ot`/`on` what the first out-pointer received,
/// `b` the bytes the buffer received, `n` a number or none. libc.wasm has the program's number types (web/playground/lib/libc.c)
pub const C_CALLS_SECTION: &str = "warp.c_calls";
/// The C library whose calls the browser host makes through libc.wasm
const LIBC: &str = "c";

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
}

/// How a parameter of a C function compiled to wasm crosses (the header beside the module, notes/wasm_modules.md)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CParameter {
	/// a number, or a pointer the program got from the module (`struct x *`): its own wasm value
	Number,
	/// `char *` or a const pointer to bytes (zlib's `const Bytef *buf`): the program's text copied into the module's malloc
	Text,
	/// the unsigned count after a text (`const void *input, size_t length`): the text's byte length, which the wasp call
	/// may leave out (`XXH32("hello", 0)`), then the text crosses by its bytes, NUL bytes included
	TextLength,
	/// `T **`: left out of the wasp call, the module gets a NULL-initialised slot of its malloc
	Out,
	/// a writable buffer followed by its in/out length (`Bytef *dest, uLongf *destLen`): the wasp call passes its
	/// capacity, the module gets a block of that size
	Buffer,
	/// the length slot of the buffer before it: left out of the wasp call, holds the capacity going in, the length written
	/// coming out
	BufferLength,
}

impl CParameter {
	/// Does the wasp call pass a value for it (the program's import has its parameter)
	fn is_passed(self) -> bool {
		!matches!(self, CParameter::Out | CParameter::BufferLength)
	}
}

/// What the result of a C function compiled to wasm crosses as
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CResult {
	Number,
	/// an unsigned 32-bit C result (`unsigned long` in wasm32): zero-extended into an Int, never negative
	Unsigned,
	/// `char *`: the NUL-terminated text in the module's memory, read back as a wasp text, NULL is ø
	Text,
	/// what the first out-pointer received (as natively, notes/ffi_handles.md): a text for `char **`, else the module's
	/// pointer as a number; NULL there is a loud error naming the C status
	Out { text: bool },
	/// the bytes the first buffer received (its length slot says how many) as a wasp text; a C status other than 0 is a
	/// loud error (zlib's Z_BUF_ERROR when the capacity is too small)
	Buffer,
}

impl Export {
	/// Does a call need the module's memory: a text or an out-pointer crosses, or the result changes type
	pub fn crosses_c_values(&self) -> bool {
		self.c_result != CResult::Number || self.c_parameters.iter().any(|parameter| *parameter != CParameter::Number)
	}
}

/// Does an import name a WebAssembly module file (`lib/x.wasm`, `x.wat`) rather than a C library
pub fn is_module_path(name: &str) -> bool {
	Path::new(name).extension().and_then(|extension| extension.to_str()).is_some_and(|extension| MODULE_EXTENSIONS.contains(&extension))
}

/// The binary of a module file; a `.wat` file is its text form
pub fn module_bytes(path: &str) -> Result<Vec<u8>, String> {
	let bytes = std::fs::read(path).map_err(|failure| format!("cannot read the module {path}: {failure}"))?;
	if !path.ends_with(".wat") {
		return Ok(bytes);
	}
	#[cfg(feature = "native")]
	return wat::parse_bytes(&bytes).map(|binary| binary.into_owned()).map_err(|failure| format!("{path}: {failure}"));
	#[cfg(not(feature = "native"))]
	Err(format!("{path}: a module in WAT text needs the native build"))
}

/// The exports of the module at `path` the program can use, by name; read once per path and process
pub fn exports(path: &str) -> &'static HashMap<String, Export> {
	static READ: std::sync::Mutex<Vec<(String, &'static HashMap<String, Export>)>> = std::sync::Mutex::new(Vec::new());
	let mut read = READ.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	if let Some((_, exports)) = read.iter().find(|(read_path, _)| read_path == path) {
		return exports;
	}
	let found = module_bytes(path).and_then(|bytes| read_exports(path, &bytes)).unwrap_or_else(|failure| {
		eprintln!("[wasm] {failure}");
		HashMap::new()
	});
	let exports: &'static HashMap<String, Export> = Box::leak(Box::new(found));
	read.push((path.to_string(), exports));
	exports
}

/// The functions and globals a module exports with number types (i32, i64, f32, f64); others are not callable from warp
fn read_exports(path: &str, bytes: &[u8]) -> Result<HashMap<String, Export>, String> {
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
					types.extend(group.map_err(failure)?.into_types().map(|sub_type| sub_type.unwrap_func().clone()));
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
						let wasmparser::Name::Local(functions) = entry.map_err(failure)? else { continue };
						for function in functions {
							let function = function.map_err(failure)?;
							let names = function.names.into_iter().filter_map(Result::ok).map(|naming| (naming.index, naming.name.to_string()));
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
	let library: &'static str = Box::leak(path.to_string().into_boxed_str());
	let mut exports = HashMap::new();
	let mut add = |name: String, params: Vec<wasm_encoder::ValType>, results: Vec<wasm_encoder::ValType>, role: Role, parameters: Vec<String>| {
		let import_name: &'static str = Box::leak(name.clone().into_boxed_str());
		exports.insert(name, Export { signature: FfiSignature::new(import_name, library, params, results), role, parameters, c_parameters: vec![], c_result: CResult::Number });
	};
	for (name, kind, index) in exported {
		match kind {
			ExternalKind::Func => {
				let Some(function_type) = function_types.get(index).and_then(|type_index| types.get(*type_index as usize)) else { continue };
				if let (Some(params), Some(results)) = (number_types(function_type.params()), number_types(function_type.results())) {
					let names = local_names.get(&(index as u32));
					let parameters = (0..params.len() as u32).map(|local| names.and_then(|names| names.get(&local)).cloned().unwrap_or_else(|| format!("${local}"))).collect();
					add(name, params, results, Role::Function, parameters);
				}
			}
			ExternalKind::Global => {
				let Some(global) = global_types.get(index) else { continue };
				let Some(value_type) = number_types(&[global.content_type]) else { continue };
				if global.mutable {
					add(format!("{SETTER_PREFIX}{name}"), value_type.clone(), value_type.clone(), Role::Setter, vec![name.clone()]);
				}
				add(name, vec![], value_type, Role::Global { mutable: global.mutable }, vec![]);
			}
			_ => {}
		}
	}
	with_header_types(path, &mut exports);
	Ok(exports)
}

/// The C_CALLS_SECTION of these imports: libc's, and the exports of modules that take or give C texts, out-pointers or
/// unsigned results
pub fn c_calls<'a>(imports: impl Iterator<Item = &'a FfiSignature>) -> String {
	let line = |import: &FfiSignature, parameters: String, result: &str| format!("{}\t{}\t{parameters}\t{result}\n", import.library, import.name);
	let libc_line = |import: &FfiSignature| {
		let texts = crate::ffi::header_text_parameters(LIBC).get(import.name).cloned().unwrap_or_default();
		let pairs = crate::ffi::string_pair_count(import.name);
		let parameters = (0..import.params.len()).map(|index| match index {
			index if index < 2 * pairs => if index % 2 == 0 { 't' } else { 'l' },
			index if texts.get(index - pairs).copied().unwrap_or(false) => 't',
			_ => 'n',
		}).collect();
		line(import, parameters, if matches!(import.results.first(), Some(wasm_encoder::ValType::Ref(_))) { "t" } else { "n" })
	};
	let module_line = |import: &FfiSignature, export: &Export| {
		let parameters = export.c_parameters.iter().map(|parameter| match parameter {
			CParameter::Number => 'n',
			CParameter::Text => 't',
			CParameter::TextLength => 's',
			CParameter::Out => 'o',
			CParameter::Buffer => 'b',
			CParameter::BufferLength => 'k',
		}).collect();
		line(import, parameters, match export.c_result {
			CResult::Number => "n",
			CResult::Unsigned => "u",
			CResult::Text => "t",
			CResult::Out { text: true } => "ot",
			CResult::Out { text: false } => "on",
			CResult::Buffer => "b",
		})
	};
	let lines: std::collections::BTreeSet<String> = imports.filter_map(|import| match import.library {
		LIBC => Some(libc_line(import)),
		library if is_module_path(library) => exports(library).get(import.name)
			.filter(|export| export.crosses_c_values())
			.map(|export| module_line(import, export)),
		_ => None,
	}).collect();
	lines.into_iter().collect()
}

/// A C library compiled to WebAssembly: the header beside it (`zlib.h` of `zlib.wasm`) says how its functions' parameters
/// and results cross (CParameter, CResult) and names the parameters; the module's own number types win (`size_t` is 32
/// bits in wasm32), pointers to structs stay the module's numbers
fn with_header_types(path: &str, exports: &mut HashMap<String, Export>) {
	use crate::ffi::{parse_header_file, pointer_kind, CPointer};
	let header = Path::new(path).with_extension("h");
	let Some(header) = header.to_str().filter(|_| header.is_file()) else { return };
	for declared in parse_header_file(header, path) {
		let Some(export) = exports.get_mut(&declared.name).filter(|export| export.role == Role::Function) else { continue };
		let c_parameters = c_parameters(&declared.param_types);
		let first_out = declared.param_types.iter().find(|c_type| pointer_kind(c_type) == Some(CPointer::Out));
		let c_result = match pointer_kind(&declared.return_type) {
			Some(CPointer::Text) => CResult::Text,
			None if c_parameters.contains(&CParameter::Buffer) => CResult::Buffer,
			None if first_out.is_some() => CResult::Out { text: first_out.is_some_and(|c_type| c_type.matches('*').count() == 2 && c_type.contains("char")) },
			None if declared.return_type.contains("unsigned") && export.signature.results == [wasm_encoder::ValType::I32] => CResult::Unsigned,
			_ => CResult::Number,
		};
		if c_parameters.len() != export.signature.params.len() || c_result != CResult::Number && export.signature.results.len() != 1 {
			eprintln!("[wasm] {header}: {} does not match the module's {} parameters", declared.raw.trim(), export.signature.params.len());
			continue;
		}
		let wasp_parameter = |index: &usize| c_parameters[*index].is_passed();
		if export.parameters.iter().all(|name| name.starts_with('$')) && declared.param_names.len() == c_parameters.len() {
			export.parameters = declared.param_names.clone();
		}
		export.parameters = (0..c_parameters.len()).filter(wasp_parameter).map(|index| export.parameters[index].clone()).collect();
		export.signature.params = (0..c_parameters.len()).filter(wasp_parameter).map(|index| export.signature.params[index]).collect();
		export.signature.results = match c_result {
			CResult::Text | CResult::Out { text: true } | CResult::Buffer => vec![wasm_encoder::ValType::Ref(wasm_encoder::RefType::ANYREF)], // the text Node the host builds
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
fn c_parameters(c_types: &[String]) -> Vec<CParameter> {
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
fn is_count(c_type: &str) -> bool {
	let words: Vec<&str> = c_type.split_whitespace().filter(|word| *word != "const").collect();
	!c_type.contains('*') && words.first().is_some_and(|first| *first == "unsigned" || *first == "size_t" || first.starts_with("uint"))
}

/// A writable pointer to an unsigned count (`uLongf *destLen` spelled `unsigned long *destLen`)
fn is_count_pointer(c_type: &str) -> bool {
	c_type.matches('*').count() == 1 && !c_type.contains("const") && is_count(&c_type.replace('*', " "))
}

/// The parameters of a module function's import (the program's view) whose text crosses with its length in the next one
pub fn texts_with_lengths(signature: &FfiSignature) -> Vec<usize> {
	if !is_module_path(signature.library) {
		return (0..crate::ffi::string_pair_count(signature.name)).map(|pair| 2 * pair).collect();
	}
	let Some(export) = exports(signature.library).get(signature.name) else { return vec![] };
	let passed: Vec<CParameter> = export.c_parameters.iter().copied().filter(|parameter| parameter.is_passed()).collect();
	(0..passed.len()).filter(|index| passed[*index] == CParameter::Text && passed.get(index + 1) == Some(&CParameter::TextLength)).collect()
}

fn number_types(types: &[wasmparser::ValType]) -> Option<Vec<wasm_encoder::ValType>> {
	types.iter().map(|value_type| match value_type {
		wasmparser::ValType::I32 => Some(wasm_encoder::ValType::I32),
		wasmparser::ValType::I64 => Some(wasm_encoder::ValType::I64),
		wasmparser::ValType::F32 => Some(wasm_encoder::ValType::F32),
		wasmparser::ValType::F64 => Some(wasm_encoder::ValType::F64),
		_ => None,
	}).collect()
}

/// The name a module's exports are qualified with: its file stem (`fourty_two.twice(21)`)
pub fn module_alias(path: &str) -> String {
	Path::new(path).file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default()
}

/// The import of `export` qualified by its module's alias: `fourty_two.twice`, which no builtin of the same name hides
pub fn qualified(path: &str, export: &str) -> String {
	format!("{}.{export}", module_alias(path))
}

/// `include m` (P139): the call of m's entry point (main, _start), whose value the include is
pub fn entry_call(path: &str) -> Option<Node> {
	let exports = exports(path);
	ENTRY_POINTS.iter().find(|entry| exports.get(**entry).is_some_and(|export| export.role == Role::Function))
		.map(|entry| call(&qualified(path, entry), vec![]))
}

/// The program's uses of the imported modules' exports: `m.f(x)` is the call of the qualified import, `m.g` and a bare
/// `g` of an exported global read its value (the getter call `g()`), `g = v` sets a mutable global (P140) and is an
/// error for an immutable one (P130); a bare call of an export that a builtin would take (`double(21)`) is ambiguous (P141)
pub fn rewrite_uses(program: Node, modules: &[String]) -> Node {
	if modules.is_empty() {
		return program;
	}
	let globals: HashMap<&String, bool> = modules.iter().flat_map(|path| exports(path).iter())
		.filter_map(|(name, export)| match export.role {
			Role::Global { mutable } => Some((name, mutable)),
			_ => None,
		}).collect();
	let rewritten = set_globals(program, &globals, modules).and_then(|program| {
		let reads: HashMap<String, Node> = globals.keys().map(|global| (global.to_string(), call(global, vec![]))).collect();
		qualify(crate::law::substitute(&program, &reads), modules)
	});
	rewritten.unwrap_or_else(|failure| failure)
}

fn call(name: &str, arguments: Vec<Node>) -> Node {
	Node::List([vec![Node::Symbol(name.to_string())], arguments].concat(), Bracket::Round, Separator::None)
}

/// An assignment target naming an exported global, `g` or `m.g`: as written, whether it is mutable, its setter import
fn assigned_global(target: &Node, globals: &HashMap<&String, bool>, modules: &[String]) -> Option<(String, bool, String)> {
	match target.drop_meta() {
		Node::Symbol(global) => globals.get(global).map(|mutable| (global.clone(), *mutable, format!("{SETTER_PREFIX}{global}"))),
		Node::Key(receiver, Op::Dot, member) => {
			let (Node::Symbol(alias), Node::Symbol(global)) = (receiver.drop_meta(), member.drop_meta()) else { return None };
			modules.iter().filter(|path| module_alias(path) == *alias).find_map(|path| match exports(path).get(global)?.role {
				Role::Global { mutable } => Some((format!("{alias}.{global}"), mutable, qualified(path, &format!("{SETTER_PREFIX}{global}")))),
				_ => None,
			})
		}
		_ => None,
	}
}

/// `g = v` and `g += v` of an exported global (also qualified, `m.g = v`): the setter call `set g(v)`, `set g(g + v)` of
/// a mutable one; an immutable one is no variable of the program, like `pi = 4` (P130)
fn set_globals(node: Node, globals: &HashMap<&String, bool>, modules: &[String]) -> Result<Node, Node> {
	if let Node::Key(target, op, value) = node.drop_meta() {
		let assigns = *op == Op::Assign || op.is_compound_assign();
		match assigned_global(target, globals, modules) {
			Some((global, false, _)) if assigns => return Err(crate::node::error(&format!("{global} is an immutable global of an imported module; fix: another name"))),
			Some((_, true, setter)) if assigns => {
				let value = set_globals(value.as_ref().clone(), globals, modules)?;
				let value = match op.is_compound_assign() {
					true => Node::Key(target.clone(), op.base_op(), Box::new(value)),
					false => value,
				};
				return Ok(call(&setter, vec![value]));
			}
			_ => {}
		}
	}
	let mut failure = None;
	let node = node.map_children(|child| set_globals(child, globals, modules).unwrap_or_else(|error| {
		failure.get_or_insert(error);
		Node::Empty
	}));
	failure.map_or(Ok(node), Err)
}

/// `m.f(x)`, `m.g` of an imported module m: the call of its qualified import; a bare `f(x)` of an export a builtin takes
/// (a cast like `double(21)`) is the ambiguity error naming both (P141)
fn qualify(node: Node, modules: &[String]) -> Result<Node, Node> {
	let module_of = |alias: &str, export: &str| modules.iter().find(|path| module_alias(path) == alias && exports(path).contains_key(export));
	match node.drop_meta() {
		Node::Key(receiver, Op::Dot, member) => {
			if let Node::Symbol(alias) = receiver.drop_meta() {
				let (export, arguments) = match member.drop_meta() {
					Node::List(items, _, _) => match items.split_first() {
						Some((Node::Symbol(export), arguments)) => (export.clone(), arguments.to_vec()),
						_ => (String::new(), vec![]),
					},
					// `m.g` read after the global pass: `m.(g)`
					Node::Symbol(export) => (export.clone(), vec![]),
					_ => (String::new(), vec![]),
				};
				if let Some(path) = module_of(alias, &export) {
					let arguments = arguments.into_iter().map(|argument| qualify(argument, modules)).collect::<Result<_, _>>()?;
					return Ok(call(&qualified(path, &export), ordered_arguments(path, &export, arguments)?));
				}
			}
		}
		Node::List(items, _, _) => {
			if let Some((Node::Symbol(name), arguments)) = items.split_first() {
				let cast = crate::analyzer::type_word_kind(&name.to_lowercase());
				let exporting = modules.iter().find(|path| exports(path).get(name).is_some_and(|export| export.role == Role::Function));
				if let (Some(kind), Some(path)) = (cast, exporting) {
					let written: Vec<String> = arguments.iter().map(|argument| argument.serialize().trim().to_string()).collect();
					let written = written.join(", ");
					let message = format!("{name} is ambiguous: {}({written}) for the export, {written} as {kind} for the cast", qualified(path, name));
					return Err(crate::node::error(&message));
				}
				// `minus(amount: 2, from: 10)`: an export called by its parameter names
				let named = arguments.iter().any(|argument| crate::named_arguments::named_argument(argument).is_some());
				if let (true, Some(path)) = (named, exporting) {
					let arguments = arguments.iter().cloned().map(|argument| qualify(argument, modules)).collect::<Result<_, _>>()?;
					return Ok(call(&qualified(path, name), ordered_arguments(path, name, arguments)?));
				}
			}
		}
		_ => {}
	}
	let mut failure = None;
	let node = node.map_children(|child| qualify(child, modules).unwrap_or_else(|error| {
		failure.get_or_insert(error);
		Node::Empty
	}));
	failure.map_or(Ok(node), Err)
}

/// The arguments of a call of the function `export` in its parameter order, a named one (`amount: 2`, `amount=2`) at its
/// parameter of the module's name section; an unknown name or a wrong count is an error showing the parameters
fn ordered_arguments(path: &str, export: &str, arguments: Vec<Node>) -> Result<Vec<Node>, Node> {
	let Some(found @ Export { role: Role::Function, parameters, .. }) = exports(path).get(export) else { return Ok(arguments) };
	let signature = format!("{}({})", qualified(path, export), parameters.join(", "));
	let wrong_count = || crate::node::error(&format!("{signature} takes {} arguments, got {}", parameters.len(), arguments.len()));
	// the lengths of texts may be left out (CParameter::TextLength): the call passes the texts' own lengths then
	let passed = found.c_parameters.iter().filter(|parameter| parameter.is_passed());
	let lengths: Vec<usize> = passed.enumerate().filter(|(_, parameter)| **parameter == CParameter::TextLength).map(|(index, _)| index).collect();
	let left_out = |index: usize| arguments.len() + lengths.len() == parameters.len() && lengths.contains(&index);
	let mut slots: Vec<Option<Node>> = vec![None; parameters.len()];
	for argument in &arguments {
		let (slot, value) = match crate::named_arguments::named_argument(argument) {
			Some((name, value)) => {
				let slot = parameters.iter().position(|parameter| *parameter == name)
					.ok_or_else(|| crate::node::error(&format!("{signature} has no parameter {name}")))?;
				(slot, value)
			}
			None => ((0..slots.len()).find(|index| slots[*index].is_none() && !left_out(*index)).ok_or_else(wrong_count)?, argument),
		};
		slots[slot] = Some(value.clone());
	}
	let given = slots.into_iter().enumerate().filter(|(index, slot)| slot.is_some() || !left_out(*index));
	given.map(|(_, slot)| slot).collect::<Option<Vec<_>>>().ok_or_else(wrong_count)
}

/// Link the program's imports from WebAssembly modules: each is a host function that calls the export of the module,
/// instantiated in the run's store at its first call
#[cfg(feature = "native")]
pub fn link(linker: &mut wasmtime::Linker<crate::host::HostState>, module: &wasmtime::Module) -> wasmtime::Result<()> {
	for import in module.imports().filter(|import| is_module_path(import.module())) {
		let wasmtime::ExternType::Func(function_type) = import.ty() else { continue };
		let (path, name) = (import.module().to_string(), import.name().to_string());
		let role = exports(&path).get(&name).map_or(Role::Function, |export| export.role);
		linker.func_new(import.module(), import.name(), function_type, move |mut caller, arguments, results| {
			let instance = instance_of(&mut caller, &path)?;
			let global_name = name.strip_prefix(SETTER_PREFIX).unwrap_or(&name);
			let global = |caller: &mut wasmtime::Caller<'_, crate::host::HostState>| instance.get_global(caller, global_name)
				.ok_or_else(|| wasmtime::format_err!("{path} exports no global {global_name}"));
			match role {
				Role::Global { .. } => results[0] = global(&mut caller)?.get(&mut caller),
				Role::Setter => {
					global(&mut caller)?.set(&mut caller, arguments[0])?;
					results[0] = arguments[0];
				}
				Role::Function => {
					let function = instance.get_func(&mut caller, &name).ok_or_else(|| wasmtime::format_err!("{path} exports no function {name}"))?;
					match exports(&path).get(&name).filter(|export| export.crosses_c_values()) {
						Some(export) => call_c(&mut caller, instance, function, export, arguments, results)?,
						None => function.call(&mut caller, arguments, results)?,
					}
				}
			}
			Ok(())
		})?;
	}
	Ok(())
}

/// A call of a C function of a module whose header says how its values cross (CParameter, CResult): each text argument
/// (a NUL-terminated copy in the program's memory, or as many bytes as its length says) is copied into a block of the
/// module's `malloc`, each out-pointer gets a NULL slot there, each buffer a block of its capacity and a length slot; the
/// result is read back (a text, an unsigned number, what the first out-pointer or buffer received) before the blocks
/// are freed
#[cfg(feature = "native")]
fn call_c(caller: &mut wasmtime::Caller<'_, crate::host::HostState>, instance: wasmtime::Instance, function: wasmtime::Func,
	export: &Export, arguments: &[wasmtime::Val], results: &mut [wasmtime::Val]) -> wasmtime::Result<()> {
	use wasmtime::Val;
	let name = export.signature.name;
	let program_memory = caller.get_export("memory").and_then(|memory| memory.into_memory())
		.ok_or_else(|| wasmtime::format_err!("{name}: the program has no memory for its text"))?;
	let module_memory = instance.get_memory(&mut *caller, "memory").ok_or_else(|| wasmtime::format_err!("{name}: its module exports no memory"))?;
	let malloc = |caller: &mut wasmtime::Caller<'_, crate::host::HostState>, bytes: &[u8]| -> wasmtime::Result<i32> {
		let malloc = instance.get_typed_func::<i32, i32>(&mut *caller, "malloc")
			.map_err(|_| wasmtime::format_err!("{name}: its module exports no malloc(size) to pass a text or out-pointer"))?;
		let block = malloc.call(&mut *caller, bytes.len() as i32)?;
		module_memory.write(&mut *caller, block as usize, bytes)?;
		Ok(block)
	};
	let too_few = || wasmtime::format_err!("{name}: too few arguments");
	let mut program_arguments = arguments.iter().peekable();
	let (mut module_arguments, mut blocks, mut out_slots, mut buffers) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
	let (mut capacity, mut buffer) = (0, 0);
	for (index, parameter) in export.c_parameters.iter().enumerate() {
		let argument = match parameter {
			CParameter::Number | CParameter::TextLength => *program_arguments.next().ok_or_else(too_few)?,
			CParameter::Text => {
				let address = program_arguments.next().ok_or_else(too_few)?.unwrap_i32() as usize;
				let memory = program_memory.data(&*caller);
				let mut text = match export.c_parameters.get(index + 1) {
					Some(CParameter::TextLength) => memory.get(address..address + count_of(program_arguments.peek().ok_or_else(too_few)?)).unwrap_or_default(),
					_ => c_text(memory, address),
				}.to_vec();
				text.push(0);
				let block = malloc(caller, &text)?;
				blocks.push(block);
				Val::I32(block)
			}
			CParameter::Out => {
				let slot = malloc(caller, &[0; 4])?;
				blocks.push(slot);
				out_slots.push(slot);
				Val::I32(slot)
			}
			CParameter::Buffer => {
				capacity = count_of(program_arguments.next().ok_or_else(too_few)?);
				buffer = malloc(caller, &vec![0; capacity])?;
				blocks.push(buffer);
				Val::I32(buffer)
			}
			CParameter::BufferLength => {
				let slot = malloc(caller, &(capacity as u32).to_le_bytes())?;
				blocks.push(slot);
				buffers.push((buffer, slot));
				Val::I32(slot)
			}
		};
		module_arguments.push(argument);
	}
	let mut returned = [Val::I32(0)];
	let result_count = function.ty(&*caller).results().len();
	function.call(&mut *caller, &module_arguments, &mut returned[..result_count])?;
	let read_u32 = |caller: &wasmtime::Caller<'_, crate::host::HostState>, address: usize| -> u32 {
		module_memory.data(caller).get(address..address + 4).map_or(0, |bytes| u32::from_le_bytes(bytes.try_into().unwrap_or_default()))
	};
	let text_at = |caller: &wasmtime::Caller<'_, crate::host::HostState>, address: u32| (address != 0).then(|| c_text(module_memory.data(caller), address as usize).to_vec());
	let result = match export.c_result {
		CResult::Number => results.first().map(|_| returned[0]),
		CResult::Unsigned => Some(Val::I64(returned[0].unwrap_i32() as u32 as i64)),
		CResult::Text => {
			let text = text_at(caller, returned[0].unwrap_i32() as u32);
			Some(crate::ffi::text_node(caller, text.as_deref()).map_err(|failure| wasmtime::format_err!("{name}: {failure}"))?)
		}
		CResult::Out { text } => {
			let received = read_u32(caller, out_slots[0] as usize);
			if received == 0 {
				return Err(wasmtime::format_err!("{name} gave nothing through its out-pointer (C status {})", returned[0].i32().unwrap_or(0)));
			}
			match text {
				true => Some(crate::ffi::text_node(caller, text_at(caller, received).as_deref()).map_err(|failure| wasmtime::format_err!("{name}: {failure}"))?),
				false => Some(Val::I32(received as i32)),
			}
		}
		CResult::Buffer => {
			let status = returned[0].i32().unwrap_or(0);
			if status != 0 {
				return Err(wasmtime::format_err!("{name} failed (C status {status})"));
			}
			let (block, length_slot) = buffers[0];
			let length = read_u32(caller, length_slot as usize) as usize;
			let bytes = module_memory.data(&*caller).get(block as usize..block as usize + length).unwrap_or_default().to_vec();
			Some(crate::ffi::text_node(caller, Some(&bytes)).map_err(|failure| wasmtime::format_err!("{name}: {failure}"))?)
		}
	};
	if let (Some(result), Some(slot)) = (result, results.first_mut()) {
		*slot = result;
	}
	if let Ok(free) = instance.get_typed_func::<i32, ()>(&mut *caller, "free") {
		for block in blocks {
			free.call(&mut *caller, block)?;
		}
	}
	Ok(())
}

/// A count the program passed (a length, a capacity) as a size
#[cfg(feature = "native")]
fn count_of(value: &wasmtime::Val) -> usize {
	value.i32().map(|count| count as u32 as usize).or_else(|| value.i64().map(|count| count as usize)).unwrap_or(0)
}

/// The NUL-terminated bytes at `address` of a linear memory
#[cfg(feature = "native")]
fn c_text(memory: &[u8], address: usize) -> &[u8] {
	let rest = memory.get(address..).unwrap_or_default();
	&rest[..rest.iter().position(|byte| *byte == 0).unwrap_or(rest.len())]
}

/// The run's instance of the module at `path`, one per file however it is named; its own imports (WASI, warp's host
/// words, C libraries, other modules) are linked like a program's
#[cfg(feature = "native")]
fn instance_of(caller: &mut wasmtime::Caller<'_, crate::host::HostState>, path: &str) -> wasmtime::Result<wasmtime::Instance> {
	let file = std::fs::canonicalize(path).map_or_else(|_| path.to_string(), |file| file.to_string_lossy().into_owned());
	if let Some(instance) = caller.data().wasm_modules.get(&file) {
		return Ok(*instance);
	}
	let bytes = module_bytes(path).map_err(wasmtime::Error::msg)?;
	let engine = caller.engine().clone();
	let module = wasmtime::Module::new(&engine, &bytes)?;
	let mut linker = wasmtime::Linker::new(&engine);
	crate::wasm_reader::link_imports(&mut linker, &engine, &module, crate::wasm_reader::Imports::EVERY)
		.map_err(|failure| wasmtime::format_err!("{path}: {failure:#}"))?;
	let instance = linker.instantiate(&mut *caller, &module)?;
	caller.data_mut().wasm_modules.insert(file, instance);
	Ok(instance)
}
