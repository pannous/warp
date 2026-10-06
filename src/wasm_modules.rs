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

/// The custom section naming the C texts the program's imports from modules cross, for a host that cannot see an
/// import's types (web/playground/host.js): a line `module\tname\tparameters\tresult` per import, `t` a text, `n` not
pub const TEXT_CROSSINGS_SECTION: &str = "warp.module_texts";

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
	/// which parameters are C texts (`char *` in the header beside the module): copied into the module's memory
	pub text_parameters: Vec<bool>,
	/// a `char *` result: the NUL-terminated text in the module's memory, read back as a wasp text
	pub text_result: bool,
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
		exports.insert(name, Export { signature: FfiSignature::new(import_name, library, params, results), role, parameters, text_parameters: vec![], text_result: false });
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

/// The TEXT_CROSSINGS_SECTION of these imports: the exports of modules that take or give a C text
pub fn text_crossings<'a>(imports: impl Iterator<Item = &'a FfiSignature>) -> String {
	let letter = |text: bool| if text { 't' } else { 'n' };
	let lines: std::collections::BTreeSet<String> = imports.filter(|import| is_module_path(import.library))
		.filter_map(|import| exports(import.library).get(import.name).map(|export| (import, export)))
		.filter(|(_, export)| export.text_result || export.text_parameters.contains(&true))
		.map(|(import, export)| {
			let parameters: String = export.text_parameters.iter().map(|text| letter(*text)).collect();
			format!("{}\t{}\t{parameters}\t{}\n", import.library, import.name, letter(export.text_result))
		}).collect();
	lines.into_iter().collect()
}

/// A C library compiled to WebAssembly: the header beside it (`shout.h` of `shout.wasm`) gives its functions C types,
/// `char *` parameters and results cross as texts; other pointers stay numbers
fn with_header_types(path: &str, exports: &mut HashMap<String, Export>) {
	use crate::ffi::{header_sig_to_ffi_sig, parse_header_file, pointer_kind, pointer_result, CPointer};
	let header = Path::new(path).with_extension("h");
	let Some(header) = header.to_str().filter(|_| header.is_file()) else { return };
	for declared in parse_header_file(header, path) {
		let Some(export) = exports.get_mut(&declared.name).filter(|export| export.role == Role::Function) else { continue };
		let text_parameters: Vec<bool> = declared.param_types.iter().map(|c_type| pointer_kind(c_type) == Some(CPointer::Text)).collect();
		let text_result = pointer_result(&declared) == Some(CPointer::Text);
		let Some(signature) = header_sig_to_ffi_sig(&declared).filter(|signature| signature.params.len() == export.signature.params.len()) else {
			eprintln!("[wasm] {header}: {} does not match the module's {} parameters", declared.raw.trim(), export.signature.params.len());
			continue;
		};
		if export.parameters.iter().all(|name| name.starts_with('$')) && declared.param_names.len() == text_parameters.len() {
			export.parameters = declared.param_names.clone();
		}
		export.signature = FfiSignature { name: export.signature.name, library: export.signature.library, ..signature };
		export.text_parameters = text_parameters;
		export.text_result = text_result;
	}
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
	let Some(Export { role: Role::Function, parameters, .. }) = exports(path).get(export) else { return Ok(arguments) };
	let signature = format!("{}({})", qualified(path, export), parameters.join(", "));
	let wrong_count = || crate::node::error(&format!("{signature} takes {} arguments, got {}", parameters.len(), arguments.len()));
	let mut slots: Vec<Option<Node>> = vec![None; parameters.len()];
	for argument in &arguments {
		let (slot, value) = match crate::named_arguments::named_argument(argument) {
			Some((name, value)) => {
				let slot = parameters.iter().position(|parameter| *parameter == name)
					.ok_or_else(|| crate::node::error(&format!("{signature} has no parameter {name}")))?;
				(slot, value)
			}
			None => (slots.iter().position(Option::is_none).ok_or_else(wrong_count)?, argument),
		};
		slots[slot] = Some(value.clone());
	}
	slots.into_iter().collect::<Option<Vec<_>>>().ok_or_else(wrong_count)
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
					match exports(&path).get(&name).filter(|export| export.text_result || export.text_parameters.contains(&true)) {
						Some(export) => call_with_texts(&mut caller, instance, function, export, arguments, results)?,
						None => function.call(&mut caller, arguments, results)?,
					}
				}
			}
			Ok(())
		})?;
	}
	Ok(())
}

/// A call of a C function of a module whose header says it takes or gives texts: each text argument (a NUL-terminated
/// copy in the program's memory) is copied into a block of the module's `malloc`, a `char *` result read back as a
/// text of the program (NULL is ø)
#[cfg(feature = "native")]
fn call_with_texts(caller: &mut wasmtime::Caller<'_, crate::host::HostState>, instance: wasmtime::Instance, function: wasmtime::Func,
	export: &Export, arguments: &[wasmtime::Val], results: &mut [wasmtime::Val]) -> wasmtime::Result<()> {
	use wasmtime::Val;
	let name = export.signature.name;
	let program_memory = caller.get_export("memory").and_then(|memory| memory.into_memory())
		.ok_or_else(|| wasmtime::format_err!("{name}: the program has no memory for its text"))?;
	let module_memory = instance.get_memory(&mut *caller, "memory").ok_or_else(|| wasmtime::format_err!("{name}: its module exports no memory for texts"))?;
	let mut module_arguments = Vec::with_capacity(arguments.len());
	for (argument, is_text) in arguments.iter().zip(export.text_parameters.iter().chain(std::iter::repeat(&false))) {
		if !is_text {
			module_arguments.push(*argument);
			continue;
		}
		let mut text = c_text(program_memory.data(&*caller), argument.unwrap_i32() as usize).to_vec();
		text.push(0);
		let malloc = instance.get_typed_func::<i32, i32>(&mut *caller, "malloc")
			.map_err(|_| wasmtime::format_err!("{name}: its module exports no malloc(size) to copy a text argument into"))?;
		let block = malloc.call(&mut *caller, text.len() as i32)?;
		module_memory.write(&mut *caller, block as usize, &text)?;
		module_arguments.push(Val::I32(block));
	}
	if !export.text_result {
		return function.call(&mut *caller, &module_arguments, results);
	}
	let mut pointer = [Val::I32(0)];
	function.call(&mut *caller, &module_arguments, &mut pointer)?;
	let address = pointer[0].unwrap_i32() as usize;
	let text = (address != 0).then(|| c_text(module_memory.data(&*caller), address).to_vec());
	results[0] = crate::ffi::text_node(caller, text.as_deref()).map_err(|failure| wasmtime::format_err!("{name}: {failure}"))?;
	Ok(())
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
