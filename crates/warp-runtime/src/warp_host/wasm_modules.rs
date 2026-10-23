//! Other WebAssembly modules a program loads (lib/wasm.warp, card warp-host-wasm-load, notes/warp_host.md):
//! - `load_wasm("x.wasm")` (std_io wasm.load): a core module instantiated in the program's own store with the same
//!   imports as a program, or a component (components.rs) by its header; a handle `{wasm: 1, path: "x.wasm", kind:
//!   "module"}`, the same one for the same file all run long
//! - `wasm_call(m, "f", [args])`: its export called, the values crossing by the export's own types
//! - an import whose module names a .wasm or .wat file, `(import "lib/x.wasm" "f" (func …))`, or a bare name with
//!   such a file (`m` with m.wasm in reach), is linked to that module's exports, instantiated once per run (the old
//!   pipeline's convention, notes/wasm_modules.md: relative to the working folder, then to the importing module's)
use crate::nodes::{self, Builders, Node};
use crate::{cache, components, ffi, Host};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use warp_runtime::gc_values::parts_of;
use wasmtime::{Extern, Instance, Linker, Module, Result, StoreContextMut, Val, ValType};

/// The modules a run loaded: a handle's id is its place + 1
#[derive(Default)]
pub struct Loaded {
	entries: Vec<Entry>,
	by_path: HashMap<PathBuf, usize>,
	/// the files being instantiated now: an import cycle among them is an error, not a deep recursion
	linking: Vec<PathBuf>,
}

struct Entry {
	path: String,
	item: Item,
}

enum Item {
	Module(Instance),
	Component(Box<components::Loaded>),
}

/// The program's words of `use wasm` (std_io module "wasm"): their answer, a value built for the program or one of its
/// own references handed back as it is; a failure is an Error `wasm.call: …`
pub fn answer(store: &mut StoreContextMut<'_, Host>, builders: &Builders, member: &str, raw: Val) -> Result<Val> {
	let parts = parts_of(&raw, store)?;
	let memory = Some(builders.memory());
	let read = |store: &mut StoreContextMut<'_, Host>, part: &Val| nodes::read(part, store, memory);
	let answered = match (member, parts.as_slice()) {
		("load", [module]) => {
			let module = read(store, module)?;
			loaded(store, &module).map(|id| handle(store.data(), id))
		}
		("exports", [module]) => {
			let module = read(store, module)?;
			loaded(store, &module).and_then(|id| exports(store, id))
		}
		("call", [module, name, arguments]) => {
			let (module, name) = (read(store, module)?, read(store, name)?.content());
			let raw_arguments = parts_of(arguments, store)?;
			match loaded(store, &module) {
				Ok(id) => return called(store, builders, id, &name, raw_arguments),
				Err(problem) => Err(problem),
			}
		}
		_ => Err(format!("warp-host has no wasm.{member} of {} arguments", parts.len())),
	};
	let answer = answered.unwrap_or_else(|problem| Node::Error(format!("wasm.{member}: {problem}").into_bytes()));
	builders.build(&answer, store)
}

/// A module's handle id: a handle, a path (loaded once per run), or its bytes (a list of bytes, a fresh instance)
fn loaded(store: &mut StoreContextMut<'_, Host>, module: &Node) -> Result<usize, String> {
	match module {
		Node::List(_, entries) if entries.iter().any(|entry| key_of(entry) == Some("wasm")) => {
			let id = entries.iter().find(|entry| key_of(entry) == Some("wasm")).and_then(value_of);
			match id {
				Some(Node::Int(id)) if *id >= 1 && (*id as usize) <= store.data().wasm.entries.len() => {
					Ok(*id as usize)
				}
				_ => Err(format!("{} is no handle of a loaded module", module.shown())),
			}
		}
		Node::Text(_) | Node::Symbol(_) => {
			let path = PathBuf::from(module.content());
			load_file(store, &path, None).map_err(|problem| format!("{problem:#}"))
		}
		Node::List(..) => {
			let bytes = byte_list(module)?;
			load_bytes(store, &bytes, "<bytes>".into(), None).map_err(|problem| format!("{problem:#}"))
		}
		other => Err(format!("needs a module: a path, its bytes or a handle, got {}", other.shown())),
	}
}

fn key_of(entry: &Node) -> Option<&str> {
	match entry {
		Node::Key(_, key, _) => match key.as_ref() {
			Node::Symbol(name) | Node::Text(name) => std::str::from_utf8(name).ok(),
			_ => None,
		},
		_ => None,
	}
}

fn value_of(entry: &Node) -> Option<&Node> {
	match entry {
		Node::Key(_, _, value) => Some(value),
		_ => None,
	}
}

fn byte_list(bytes: &Node) -> Result<Vec<u8>, String> {
	bytes
		.clone()
		.items()
		.into_iter()
		.map(|byte| match byte {
			Node::Int(byte) => u8::try_from(byte).map_err(|_| format!("{byte} is no byte (0 to 255)")),
			other => Err(format!("{} is no byte (0 to 255)", other.shown())),
		})
		.collect()
}

fn handle(host: &Host, id: usize) -> Node {
	let entry = &host.wasm.entries[id - 1];
	let kind = match entry.item {
		Item::Module(_) => "module",
		Item::Component(_) => "component",
	};
	Node::map(vec![("wasm", Node::Int(id as i64)), ("path", Node::text(&entry.path)), ("kind", Node::text(kind))])
}

/// The file loaded once per run: its handle id
pub fn load_file(store: &mut StoreContextMut<'_, Host>, path: &Path, folder: Option<&Path>) -> Result<usize> {
	let found = found_file(path, folder).ok_or_else(|| {
		wasmtime::Error::msg(format!(
			"no module {} (looked in the working folder{})",
			path.display(),
			folder.map(|folder| format!(" and {}", folder.display())).unwrap_or_default()
		))
	})?;
	let key = found.canonicalize().unwrap_or_else(|_| found.clone());
	if let Some(id) = store.data().wasm.by_path.get(&key) {
		return Ok(*id);
	}
	let linking = &store.data().wasm.linking;
	if let Some(at) = linking.iter().position(|file| file == &key) {
		let names = |file: &PathBuf| file.file_name().map_or(String::new(), |name| name.to_string_lossy().into_owned());
		let cycle = linking[at..].iter().chain([&key]).map(names).collect::<Vec<_>>().join(" → ");
		wasmtime::bail!("the modules import each other in a cycle: {cycle}");
	}
	let bytes = file_bytes(&found)?;
	store.data_mut().wasm.linking.push(key.clone());
	let id = load_bytes(store, &bytes, path.display().to_string(), key.parent());
	store.data_mut().wasm.linking.retain(|linking| linking != &key);
	let id = id?;
	store.data_mut().wasm.by_path.insert(key, id);
	Ok(id)
}

/// A module's or component's bytes instantiated: its handle id
fn load_bytes(
	store: &mut StoreContextMut<'_, Host>,
	bytes: &[u8],
	path: String,
	folder: Option<&Path>,
) -> Result<usize> {
	let item = if components::is_component(bytes) {
		Item::Component(Box::new(components::load(bytes)?))
	} else {
		let engine = store.data().engine.clone();
		let module = cache::module(&engine, bytes)?;
		Item::Module(instantiate(store, &module, bytes, folder)?)
	};
	let entries = &mut store.data_mut().wasm.entries;
	entries.push(Entry { path, item });
	Ok(entries.len())
}

/// A file's module bytes; WAT text only in a build with the feature wat
fn file_bytes(file: &Path) -> Result<Vec<u8>> {
	let bytes =
		std::fs::read(file).map_err(|problem| wasmtime::Error::msg(format!("{}: {problem}", file.display())))?;
	if file.extension().is_some_and(|extension| extension == "wat") && !bytes.starts_with(b"\0asm") {
		#[cfg(feature = "wat")]
		return Ok(wat::parse_bytes(&bytes)?.to_vec());
		#[cfg(not(feature = "wat"))]
		wasmtime::bail!(
			"{}: this warp-host reads no WAT text: build it with --features wat, or assemble it",
			file.display()
		);
	}
	Ok(bytes)
}

/// The module file a path names: as given (relative to the working folder), else in the importing module's folder
fn found_file(path: &Path, folder: Option<&Path>) -> Option<PathBuf> {
	let beside = folder.filter(|_| path.is_relative()).map(|folder| folder.join(path));
	std::iter::once(path.to_path_buf()).chain(beside).find(|file| file.is_file())
}

/// The module file an import's module names: a .wasm or .wat path, or a bare name with name.wasm (name.wat) in reach;
/// None for host, WASI and C libraries
fn import_file(name: &str, folder: Option<&Path>) -> Option<PathBuf> {
	if name == warp_runtime::host_words::HOST_LIBRARY || name.starts_with("wasi") || name.is_empty() {
		return None;
	}
	if name.ends_with(".wasm") || name.ends_with(".wat") {
		return Some(PathBuf::from(name));
	}
	let named = |extension: &str| found_file(&PathBuf::from(format!("{name}.{extension}")), folder);
	named("wasm").or_else(|| named("wat"))
}

/// A core module instantiated in the store with a program's imports: WASI, the host words, std_io, other modules by
/// path (each once per run), C libraries; any other import traps when called
pub fn instantiate(
	store: &mut StoreContextMut<'_, Host>,
	module: &Module,
	bytes: &[u8],
	folder: Option<&Path>,
) -> Result<Instance> {
	let mut linker = crate::program_linker(module.engine())?;
	link_module_imports(&mut linker, store, module, folder)?;
	ffi::link_c_imports(&mut linker, store, module, bytes)?;
	linker.define_unknown_imports_as_traps(module)?;
	linker.instantiate(store, module)
}

/// Each import whose module names a module file: that module's exports, the module loaded once per run
fn link_module_imports(
	linker: &mut Linker<Host>,
	store: &mut StoreContextMut<'_, Host>,
	module: &Module,
	folder: Option<&Path>,
) -> Result<()> {
	let mut names: Vec<&str> = module.imports().map(|import| import.module()).collect();
	names.dedup();
	for name in names {
		let Some(file) = import_file(name, folder) else { continue };
		let id = load_file(store, &file, folder).map_err(|problem| {
			let problem = format!("{problem:#}");
			// the innermost import names the problem: a chain of them says nothing more
			match problem.starts_with("the import module ") {
				true => wasmtime::Error::msg(problem),
				false => wasmtime::Error::msg(format!("the import module {name}: {problem}")),
			}
		})?;
		match store.data().wasm.entries[id - 1].item {
			Item::Module(instance) => {
				linker.allow_shadowing(true);
				linker.instance(&mut *store, name, instance)?;
			}
			Item::Component(_) => {
				wasmtime::bail!("the import module {name} is a component: a core module imports only core modules")
			}
		}
	}
	Ok(())
}

/// `wasm_call(m, "f", [args])`: the export called with the arguments as its types want them
fn called(
	store: &mut StoreContextMut<'_, Host>,
	builders: &Builders,
	id: usize,
	name: &str,
	arguments: Vec<Val>,
) -> Result<Val> {
	let failed = |store: &mut StoreContextMut<'_, Host>, problem: String| {
		builders.build(&Node::Error(format!("wasm.call: {problem}").into_bytes()), store)
	};
	let memory = Some(builders.memory());
	let instance = match &store.data().wasm.entries[id - 1].item {
		Item::Module(instance) => *instance,
		Item::Component(_) => {
			let mut given = Vec::with_capacity(arguments.len());
			for argument in &arguments {
				given.push(nodes::read(argument, store, memory)?);
			}
			let Item::Component(component) = &mut store.data_mut().wasm.entries[id - 1].item else {
				unreachable!("a component's entry")
			};
			let answer = component.call(name, &given).unwrap_or_else(|problem| Node::Error(problem.into_bytes()));
			return match answer {
				Node::Error(problem) => failed(store, String::from_utf8_lossy(&problem).into_owned()),
				answer => builders.build(&answer, store),
			};
		}
	};
	let path = store.data().wasm.entries[id - 1].path.clone();
	let Some(function) = instance.get_func(&mut *store, name) else {
		let names = function_names(store, instance).join(", ");
		return failed(store, format!("{path} has no function {name} (it has {names})"));
	};
	let ty = function.ty(&*store);
	if ty.params().len() != arguments.len() {
		let problem = format!("{path}'s {name} takes {} arguments, got {}", ty.params().len(), arguments.len());
		return failed(store, problem);
	}
	// a warp module's nodes cross by copy through its own constructors: their texts may live in its linear memory
	let callee = Builders::of(|export| instance.get_export(&mut *store, export));
	let mut given = Vec::new();
	for (argument, wanted) in arguments.iter().zip(ty.params()) {
		match crossed_in(store, argument, &wanted, memory, callee.as_ref()) {
			Ok(value) => given.push(value),
			Err(problem) => return failed(store, format!("{path}'s {name}: {problem}")),
		}
	}
	let mut results = vec![Val::I32(0); ty.results().len()];
	if let Err(problem) = function.call(&mut *store, &given, &mut results) {
		return failed(store, format!("{path}'s {name} failed: {}", trap_text(&problem)));
	}
	let callee_memory = instance.get_memory(&mut *store, "memory");
	let mut out = Vec::new();
	for result in &results {
		match crossed_out(store, result, callee_memory) {
			Ok(node) => out.push(node),
			Err(problem) => return failed(store, format!("{path}'s {name}: {problem}")),
		}
	}
	let answer = match out.len() {
		0 => Node::Empty,
		1 => out.pop().unwrap_or(Node::Empty),
		_ => Node::List(nodes::SQUARE_LIST, out),
	};
	builders.build(&answer, store)
}

fn trap_text(problem: &wasmtime::Error) -> String {
	match problem.downcast_ref::<wasmtime::Trap>() {
		Some(trap) => trap.to_string(),
		None => format!("{problem:#}"),
	}
}

/// An argument as the parameter's type wants it: numbers from Ints, Floats and Bools, a reference the program's own
/// value (copied into a warp module through its constructors)
fn crossed_in(
	store: &mut StoreContextMut<'_, Host>,
	argument: &Val,
	wanted: &ValType,
	memory: Option<wasmtime::Memory>,
	callee: Option<&Builders>,
) -> Result<Val, String> {
	if let ValType::Ref(_) = wanted {
		return match callee {
			Some(callee) => {
				let node = nodes::read(argument, store, memory).map_err(|problem| problem.to_string())?;
				callee.build(&node, store).map_err(|problem| problem.to_string())
			}
			None => Ok(*argument),
		};
	}
	let node = nodes::read(argument, store, memory).map_err(|problem| problem.to_string())?;
	let integer = |node: &Node| match node {
		Node::Int(n) => Ok(*n),
		Node::Bool(truth) => Ok(i64::from(*truth)),
		other => Err(format!("{} is no integer", other.shown())),
	};
	let number = |node: &Node| match node {
		Node::Float(x) => Ok(*x),
		Node::Int(n) => Ok(*n as f64),
		other => Err(format!("{} is no number", other.shown())),
	};
	Ok(match wanted {
		ValType::I32 => {
			let n = integer(&node)?;
			// an i32 takes the signed and the unsigned range: 4294967295 is -1
			Val::I32(
				i32::try_from(n)
					.or_else(|_| u32::try_from(n).map(|n| n as i32))
					.map_err(|_| format!("{n} does not fit an i32"))?,
			)
		}
		ValType::I64 => Val::I64(integer(&node)?),
		ValType::F32 => Val::F32((number(&node)? as f32).to_bits()),
		ValType::F64 => Val::F64(number(&node)?.to_bits()),
		other => return Err(format!("parameters of the type {other} are not supported yet")),
	})
}

/// A result as a value of the program: numbers as Ints and Floats, a warp module's node read out of its store
fn crossed_out(
	store: &mut StoreContextMut<'_, Host>,
	result: &Val,
	memory: Option<wasmtime::Memory>,
) -> Result<Node, String> {
	Ok(match result {
		Val::I32(n) => Node::Int(i64::from(*n)),
		Val::I64(n) => Node::Int(*n),
		Val::F32(bits) => Node::Float(f64::from(f32::from_bits(*bits))),
		Val::F64(bits) => Node::Float(f64::from_bits(*bits)),
		Val::AnyRef(None) => Node::Empty,
		Val::AnyRef(Some(_)) => match nodes::read(result, store, memory).map_err(|problem| problem.to_string())? {
			Node::Empty if !is_node(store, result) => {
				match result.anyref().flatten().and_then(|r| r.as_i31(&*store).ok().flatten()) {
					Some(small) => Node::Int(i64::from(small.get_i32())),
					None => return Err("its result is a reference that is no warp value".into()),
				}
			}
			node => node,
		},
		other => return Err(format!("results of the type {:?} are not supported yet", other)),
	})
}

/// A reference that looks like a warp node: a struct whose first field is the i64 kind
fn is_node(store: &mut StoreContextMut<'_, Host>, value: &Val) -> bool {
	let Some(node) = value.anyref().flatten().and_then(|r| r.unwrap_struct(&*store).ok()) else { return false };
	node.field(&mut *store, 0).ok().and_then(|kind| kind.i64()).is_some()
}

fn function_names(store: &mut StoreContextMut<'_, Host>, instance: Instance) -> Vec<String> {
	let exports: Vec<(String, Extern)> =
		instance.exports(&mut *store).map(|export| (export.name().to_string(), export.into_extern())).collect();
	exports.into_iter().filter(|(_, export)| matches!(export, Extern::Func(_))).map(|(name, _)| name).collect()
}

/// `wasm_exports(m)`: each export's name and type, `{twice: "func(i64) -> i64", memory: "memory"}`
fn exports(store: &mut StoreContextMut<'_, Host>, id: usize) -> Result<Node, String> {
	let instance = match &store.data().wasm.entries[id - 1].item {
		Item::Module(instance) => *instance,
		Item::Component(component) => return Ok(component.exports()),
	};
	let exports: Vec<(String, Extern)> =
		instance.exports(&mut *store).map(|export| (export.name().to_string(), export.into_extern())).collect();
	let entries = exports
		.into_iter()
		.map(|(name, export)| {
			let kind = match export {
				Extern::Func(function) => {
					let ty = function.ty(&*store);
					let list = |types: Vec<String>| types.join(", ");
					let params = list(ty.params().map(|t| t.to_string()).collect());
					let results = list(ty.results().map(|t| t.to_string()).collect());
					let count = ty.results().len();
					match count {
						0 => format!("func({params})"),
						1 => format!("func({params}) -> {results}"),
						_ => format!("func({params}) -> ({results})"),
					}
				}
				Extern::Global(global) => format!("global {}", global.ty(&*store).content()),
				Extern::Memory(_) => "memory".into(),
				Extern::Table(_) => "table".into(),
				_ => "other".into(),
			};
			(name, Node::text(kind))
		})
		.collect::<Vec<_>>();
	Ok(Node::named_map(entries))
}
