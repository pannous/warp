//! warp-host: the small native host of pure warp programs (notes/warp_host.md). It compiles a program with the warp
//! compiler written in warp (compiler-self.wasm, or compiler.wasm with `--compiler full`) and runs the module with WASI
//! and what wasm cannot do itself: shell commands (os.command, os.exec, os.run), C calls, the host words.
//!   warp-host [--compiler self|full] run <file.warp | code> [args…]   compile and run; the run's value printed
//!   warp-host [--compiler self|full] compile <file.warp> [<file.wasm>]
//!   warp-host <file.wasm> [args…]
mod cache;
mod nodes;
mod words;

use nodes::Node;
use std::path::{Path, PathBuf};
use warp_runtime::c_calls::{CHandles, CallsC};
use warp_runtime::system_signals;
use wasmtime::{Engine, Instance, Linker, Module, Store, Val};
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::{FsPerms, WasiCtxBuilder};

/// The stack wasm runs on: the compiler recurses deeply (warp's self-hosted stage workers take 64 MB)
const WASM_STACK: usize = 240 << 20;
const THREAD_STACK: usize = WASM_STACK + (16 << 20);
const USAGE: &str = "usage: warp-host [--compiler self|full] run <file.warp | code> [args…]
       warp-host [--compiler self|full] compile <file.warp> [<file.wasm>]
       warp-host <file.wasm> [args…]";

/// A run's state: WASI's, the C handles of its C calls, and what std_io needs
pub struct Host {
	wasi: WasiP1Ctx,
	c: CHandles,
	/// where texts for the program's constructors go (nodes.rs Builders::text): an address, and the memory's pages then
	scratch: Option<(u32, u64)>,
	args: Vec<String>,
	/// whose kept values os.kept reads: the compiler module and lib/ it compiles with, else the program
	keeper: Option<String>,
}

impl CallsC for Host {
	fn c_handles(&mut self) -> &mut CHandles {
		&mut self.c
	}
}

/// How a run ended
enum Ran {
	Value(Node),
	Exited(i32),
	Failed(String),
}

fn main() {
	let args = std::env::args().skip(1).collect();
	let worker = std::thread::Builder::new().stack_size(THREAD_STACK).spawn(move || command(args));
	std::process::exit(worker.map_or(1, |worker| worker.join().unwrap_or(1)))
}

fn command(given: Vec<String>) -> i32 {
	// options before the file: --compiler, and the launcher's (--no-hints, --fuel n …), which warp-host ignores
	let (mut full, mut args, mut given) = (false, Vec::<String>::new(), given.into_iter());
	while let Some(word) = given.next() {
		let before_file =
			args.is_empty() || matches!(args.as_slice(), [command] if command == "run" || command == "compile");
		match word.as_str() {
			"--compiler" | "--fuel" if before_file => {
				full |= word == "--compiler" && given.next().as_deref() == Some("full")
			}
			option if before_file && option.starts_with("--") => {}
			_ => args.push(word),
		}
	}
	let engine = match engine() {
		Ok(engine) => engine,
		Err(problem) => return failed(&format!("{problem:#}")),
	};
	let compiler = if full { "compiler.wasm" } else { "compiler-self.wasm" };
	match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
		["run", source, ..] => run_source(&engine, compiler, source, &args[2..]),
		["compile", source] => {
			compile_file(&engine, compiler, Path::new(source), &Path::new(source).with_extension("wasm"))
		}
		["compile", source, wasm] => compile_file(&engine, compiler, Path::new(source), Path::new(wasm)),
		[file, ..] if file.ends_with(".wasm") => match std::fs::read(file) {
			Ok(bytes) => shown(run_wasm(&engine, &bytes, args.clone(), None)),
			Err(problem) => failed(&format!("{file}: {problem}")),
		},
		[source, ..] if source.ends_with(".warp") => run_source(&engine, compiler, source, &args[1..]),
		_ => {
			eprintln!("{USAGE}");
			2
		}
	}
}

/// warp's engine settings (GC, typed function references; exceptions come with GC), the deep stack the compiler needs
fn engine() -> wasmtime::Result<Engine> {
	let mut config = warp_runtime::engine::deterministic_config();
	config.async_stack_size(WASM_STACK).max_wasm_stack(WASM_STACK);
	Engine::new(&config)
}

/// `run f.warp`, `run <code>` (a program of the current folder): compiled to a module of its own, then run
fn run_source(engine: &Engine, compiler: &str, source: &str, args: &[String]) -> i32 {
	let scratch = std::env::temp_dir().join(format!("warp-host-{}", std::process::id()));
	let code = scratch.with_extension("warp");
	let (file, folder) = if Path::new(source).is_file() {
		(PathBuf::from(source), None)
	} else if let Err(problem) = std::fs::write(&code, source) {
		return failed(&format!("{}: {problem}", code.display()));
	} else {
		(code.clone(), std::env::current_dir().ok())
	};
	let wasm = scratch.with_extension("wasm");
	let compiled = compile(engine, compiler, &file, &wasm, folder);
	let _ = std::fs::remove_file(&code);
	let module = compiled.and_then(|()| std::fs::read(&wasm).map_err(|problem| problem.to_string()));
	let _ = std::fs::remove_file(&wasm);
	match module {
		Ok(bytes) => shown(run_wasm(engine, &bytes, [vec![source.to_string()], args.to_vec()].concat(), None)),
		Err(problem) => refused(&problem),
	}
}

fn compile_file(engine: &Engine, compiler: &str, source: &Path, wasm: &Path) -> i32 {
	match compile(engine, compiler, source, wasm, None) {
		Ok(()) => 0,
		Err(problem) => refused(&problem),
	}
}

/// The module of `source` written to `wasm` by the compiler, the uses of `folder` (else the source's) in reach; the
/// compile's error, a refusal of a form beyond the emitter's core among them ("outside the core: …")
fn compile(engine: &Engine, compiler: &str, source: &Path, wasm: &Path, folder: Option<PathBuf>) -> Result<(), String> {
	let absolute = |path: &Path| std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
	let source = absolute(source);
	let folder = folder.or_else(|| source.parent().map(Path::to_path_buf)).unwrap_or_default();
	let lib = library_for(&source).ok_or("no lib/ with compiler/ found: WARP_LIB names one")?;
	let compiler = compiler_file(&lib, compiler)
		.ok_or(format!("no {compiler} beside warp-host nor in {}/../data", lib.display()))?;
	let bytes = std::fs::read(&compiler).map_err(|problem| format!("{}: {problem}", compiler.display()))?;
	let keeper = cache::fingerprint((&bytes, cache::folder_fingerprint(&lib)));
	let argv = [compiler.as_path(), &source, &absolute(wasm), &folder, &lib].map(|path| path.display().to_string());
	match run_wasm(engine, &bytes, argv.to_vec(), Some(keeper)) {
		Ran::Value(Node::Error(message)) => Err(String::from_utf8_lossy(&message).into_owned()),
		Ran::Failed(message) => Err(message),
		Ran::Exited(status) if status != 0 => Err(format!("the compiler exited with status {status}")),
		_ => Ok(()),
	}
}

/// lib/ for compiler.warp, as warp-rust finds it: WARP_LIB, else the nearest lib/ holding compiler/ above the source or
/// the current folder, else the one installed beside warp-host or in <prefix>/share/warp
fn library_for(source: &Path) -> Option<PathBuf> {
	if let Some(lib) = std::env::var_os("WARP_LIB") {
		return Some(PathBuf::from(lib));
	}
	let above =
		|start: PathBuf| start.ancestors().map(|folder| folder.join("lib")).find(|lib| lib.join("compiler").is_dir());
	let current = std::env::current_dir().ok();
	let installed =
		installed_folders().into_iter().map(|folder| folder.join("lib")).find(|lib| lib.join("compiler").is_dir());
	source.parent().map(Path::to_path_buf).and_then(above).or_else(|| current.and_then(above)).or(installed)
}

/// The compiler module: built from that lib/ (<lib>/../data/, a checkout's), else installed beside warp-host
fn compiler_file(lib: &Path, name: &str) -> Option<PathBuf> {
	let folders = [vec![lib.join("../data")], installed_folders()].concat();
	folders.into_iter().map(|folder| folder.join(name)).find(|file| file.is_file())
}

fn installed_folders() -> Vec<PathBuf> {
	let beside = std::env::current_exe().ok().and_then(|exe| exe.canonicalize().ok()?.parent().map(Path::to_path_buf));
	beside.map(|folder| vec![folder.join("../share/warp"), folder]).unwrap_or_default()
}

/// Run a module's main (else _start) with WASI (stdio, environment, the file system at / and .) and the host's words
fn run_wasm(engine: &Engine, bytes: &[u8], args: Vec<String>, keeper: Option<String>) -> Ran {
	let ran = || -> wasmtime::Result<Ran> {
		let module = if keeper.is_some() { cache::module(engine, bytes)? } else { Module::new(engine, bytes)? };
		let mut wasi = WasiCtxBuilder::new();
		wasi.inherit_stdio().inherit_env().args(&args);
		wasi.preopened_dir("/", "/", FsPerms::ReadWrite)?;
		wasi.preopened_dir(".", ".", FsPerms::ReadWrite)?;
		let host = Host {
			wasi: wasi.build_p1(),
			c: CHandles::default(),
			scratch: None,
			args: args.clone(),
			keeper: keeper.clone(),
		};
		let mut linker = Linker::new(engine);
		wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |host: &mut Host| &mut host.wasi)?;
		warp_runtime::host_words::link_host_words(&mut linker)?;
		warp_runtime::libm::link_libm(&mut linker)?;
		warp_runtime::c_calls::link_libc_functions(&mut linker)?;
		words::link(&mut linker)?;
		linker.define_unknown_imports_as_traps(&module)?;
		let mut store = Store::new(engine, host);
		warp_runtime::engine::cap_memory(&mut store);
		let instance = linker.instantiate(&mut store, &module)?;
		let main = warp_runtime::standalone::ENTRY_POINTS.iter().find_map(|name| instance.get_func(&mut store, name));
		let main = main.ok_or_else(|| wasmtime::Error::msg("the module has no main nor _start"))?;
		let mut results = vec![Val::AnyRef(None); main.ty(&store).results().len()];
		system_signals::allow_staying();
		let outcome = main
			.call(&mut store, &[], &mut results)
			.and_then(|()| system_signals::stay_while_listening(&mut store, &instance));
		let outcome =
			system_signals::with_exit_handler(outcome, &mut store, |store, name| instance.get_func(store, name));
		Ok(match outcome {
			Ok(Some(())) => match results.first() {
				Some(result) => {
					let memory = instance.get_memory(&mut store, "memory");
					Ran::Value(nodes::read(result, &mut store.as_context_mut(), memory)?)
				}
				None => Ran::Value(Node::Empty),
			},
			Ok(None) => Ran::Exited(system_signals::take_exit_code().unwrap_or(0)),
			Err(error) => match error.downcast_ref::<wasmtime_wasi::I32Exit>() {
				Some(exit) => Ran::Exited(exit.0),
				None => Ran::Failed(failure(&error, &mut store, &instance)),
			},
		})
	};
	ran().unwrap_or_else(|problem| Ran::Failed(format!("{problem:#}")))
}

use wasmtime::AsContextMut;

/// A failed run's message: a runtime error fails inside a function of its name (index_out_of_range, …), the global
/// trap_detail holding the value it names; a raise's message is that value (lib/compiler/runtime/errors.warp)
fn failure(error: &wasmtime::Error, store: &mut Store<Host>, instance: &Instance) -> String {
	// WARP_HOST_DEBUG=1: the failure with its wasm backtrace too
	if std::env::var_os("WARP_HOST_DEBUG").is_some() {
		eprintln!("{error:?}");
	}
	let detail = instance.get_global(&mut *store, "trap_detail").map(|global| global.get(&mut *store));
	let memory = instance.get_memory(&mut *store, "memory");
	let detail = detail.filter(|value| !matches!(value, Val::AnyRef(None)));
	let detail = detail.and_then(|value| nodes::read(&value, &mut store.as_context_mut(), memory).ok());
	let frame = error.downcast_ref::<wasmtime::WasmBacktrace>().and_then(|trace| {
		trace
			.frames()
			.iter()
			.find_map(|frame| frame.func_name().map(|name| name.split('·').next().unwrap_or(name).replace('_', " ")))
	});
	match (frame.as_deref(), detail) {
		(Some("returned error") | None, Some(detail)) => detail.content(),
		(Some(name), Some(detail)) => format!("{name}: {}", detail.shown()),
		(Some(name), None) if !name.starts_with("wasm") => name.to_string(),
		_ => format!("{:#}", error),
	}
}

/// The run's value printed (ø prints nothing), its status: an Int's, an Error's 1, else 0 (as warp-rust's display.rs)
fn shown(ran: Ran) -> i32 {
	match ran {
		Ran::Value(Node::Empty) => 0,
		Ran::Value(value) => {
			println!("{}", value.shown());
			match value {
				Node::Int(n) => n as i32,
				Node::Error(_) => 1,
				_ => 0,
			}
		}
		Ran::Exited(status) => status,
		Ran::Failed(message) => failed(&message),
	}
}

/// A compile that failed: its Error the run's only output (the launcher's `refused` reads it there), status 1
fn refused(message: &str) -> i32 {
	println!("{}", Node::Error(message.as_bytes().to_vec()).shown());
	1
}

fn failed(message: &str) -> i32 {
	eprintln!("warp-host: {message}");
	1
}
