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

/// An export of a module: a function, or a global read through a getter function of no parameters
#[derive(Clone, Debug)]
pub struct Export {
	pub signature: FfiSignature,
	pub is_global: bool,
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
						TypeRef::Global(global) => global_types.push(global.content_type),
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
					global_types.push(global.map_err(failure)?.ty.content_type);
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
	for (name, kind, index) in exported {
		let (params, results, is_global) = match kind {
			ExternalKind::Func => {
				let Some(function_type) = function_types.get(index).and_then(|type_index| types.get(*type_index as usize)) else { continue };
				(number_types(function_type.params()), number_types(function_type.results()), false)
			}
			ExternalKind::Global => (Some(vec![]), global_types.get(index).and_then(|value_type| number_types(&[*value_type])), true),
			_ => continue,
		};
		let (Some(params), Some(results)) = (params, results) else { continue };
		let export_name: &'static str = Box::leak(name.clone().into_boxed_str());
		exports.insert(name, Export { signature: FfiSignature::new(export_name, library, params, results), is_global });
	}
	Ok(exports)
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

/// The program's uses of the imported modules' exports: `m.f(x)` is the call of the qualified import, `m.g` and a bare
/// `g` of an exported global read its value (the getter call `g()`); assigning such a global is an error
pub fn rewrite_uses(program: Node, modules: &[String]) -> Node {
	if modules.is_empty() {
		return program;
	}
	let globals: Vec<&String> = modules.iter().flat_map(|path| exports(path).iter().filter(|(_, export)| export.is_global).map(|(name, _)| name)).collect();
	let mut bound = std::collections::HashSet::new();
	crate::library_words::collect_assigned_names(&program, &mut bound);
	// like `pi = 4` (P130): an imported global is no variable of the program
	if let Some(assigned) = globals.iter().find(|global| bound.contains(**global)) {
		return crate::node::error(&format!("{assigned} is a global of an imported module; fix: another name"));
	}
	let calls: HashMap<String, Node> = globals.iter().map(|global| (global.to_string(), call(global, vec![]))).collect();
	qualify(crate::law::substitute(&program, &calls), modules)
}

fn call(name: &str, arguments: Vec<Node>) -> Node {
	Node::List([vec![Node::Symbol(name.to_string())], arguments].concat(), Bracket::Round, Separator::None)
}

/// `m.f(x)`, `m.g` of an imported module m: the call of its qualified import
fn qualify(node: Node, modules: &[String]) -> Node {
	let module_of = |alias: &str, export: &str| modules.iter().find(|path| module_alias(path) == alias && exports(path).contains_key(export));
	if let Node::Key(receiver, Op::Dot, member) = node.drop_meta() {
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
				let arguments = arguments.into_iter().map(|argument| qualify(argument, modules)).collect();
				return call(&qualified(path, &export), arguments);
			}
		}
	}
	node.map_children(|child| qualify(child, modules))
}

/// Link the program's imports from WebAssembly modules: each is a host function that calls the export of the module,
/// instantiated in the run's store at its first call
#[cfg(feature = "native")]
pub fn link(linker: &mut wasmtime::Linker<crate::host::HostState>, module: &wasmtime::Module) -> wasmtime::Result<()> {
	for import in module.imports().filter(|import| is_module_path(import.module())) {
		let wasmtime::ExternType::Func(function_type) = import.ty() else { continue };
		let (path, name) = (import.module().to_string(), import.name().to_string());
		let is_global = exports(&path).get(&name).is_some_and(|export| export.is_global);
		linker.func_new(import.module(), import.name(), function_type, move |mut caller, arguments, results| {
			let instance = instance_of(&mut caller, &path)?;
			if is_global {
				let global = instance.get_global(&mut caller, &name).ok_or_else(|| wasmtime::format_err!("{path} exports no global {name}"))?;
				results[0] = global.get(&mut caller);
				return Ok(());
			}
			let function = instance.get_func(&mut caller, &name).ok_or_else(|| wasmtime::format_err!("{path} exports no function {name}"))?;
			function.call(&mut caller, arguments, results)
		})?;
	}
	Ok(())
}

/// The run's instance of the module at `path`; a module that imports anything is not linked yet
#[cfg(feature = "native")]
fn instance_of(caller: &mut wasmtime::Caller<'_, crate::host::HostState>, path: &str) -> wasmtime::Result<wasmtime::Instance> {
	if let Some(instance) = caller.data().wasm_modules.get(path) {
		return Ok(*instance);
	}
	let bytes = module_bytes(path).map_err(wasmtime::Error::msg)?;
	let module = wasmtime::Module::new(caller.engine(), &bytes)?;
	if let Some(import) = module.imports().next() {
		wasmtime::bail!("{path} imports {}.{}: only modules without imports can be imported yet", import.module(), import.name());
	}
	let instance = wasmtime::Instance::new(&mut *caller, &module, &[])?;
	caller.data_mut().wasm_modules.insert(path.to_string(), instance);
	Ok(instance)
}
