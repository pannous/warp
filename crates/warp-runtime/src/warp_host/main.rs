//! warp-host: the small native host of pure warp programs (notes/warp_host.md). It compiles a program with the warp
//! compiler written in warp (compiler-self.wasm, or compiler.wasm with `--compiler full`) and runs the module with WASI
//! and what wasm cannot do itself: shell commands (os.command, os.exec, os.run), C calls, the host words.
//!   warp-host [--compiler self|full] [run] <file.warp | code> [args…]   compile and run; the run's value printed
//!   warp-host [--compiler self|full] compile [--aot] <file.warp> [<file.wasm>]   --aot: also its machine code, .cwasm
//!   warp-host [--compiler self|full] build <file.warp | file.wasm> [-o <app>]   a standalone executable (aot.rs)
//!   warp-host <file.wasm | file.wat | file.cwasm> [args…]   a module; its C imports loaded from their libraries (ffi.rs)
//! As warp itself: no command means run, so `warp-host f.warp` and `warp-host '6*7'` are `warp-host run …`
mod aot;
mod cache;
mod components;
mod ffi;
mod nodes;
mod shell;
mod wasm_modules;
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
const USAGE: &str = "usage: warp-host [--compiler self|full] [run] <file.warp | code> [args…]
       warp-host [--compiler self|full] compile [--aot] <file.warp> [<file.wasm>]
       warp-host [--compiler self|full] build <file.warp | file.wasm> [-o <app>]
       warp-host <file.wasm | file.wat | file.cwasm> [args…]
no command means run: warp-host f.warp, warp-host '6*7'
compile --aot also writes the machine code for this machine, <file>.cwasm; build writes an executable carrying it";
/// The words warp-host takes as commands; any other first word is a program (`run` implied), as with warp
const COMMANDS: [&str; 4] = ["run", "compile", "build", "help"];
/// The extensions of warp source, as warp's (src/cli/formatting.rs WARP_EXTENSIONS)
const WARP_EXTENSIONS: [&str; 2] = ["warp", "wasp"];

/// A run's state: WASI's, the C handles of its C calls, and what std_io needs
pub struct Host {
	wasi: WasiP1Ctx,
	c: CHandles,
	/// where texts for the program's constructors go (nodes.rs Builders::text): an address, and the memory's pages then
	scratch: Option<(u32, u64)>,
	args: Vec<String>,
	/// whose kept values os.kept reads: the compiler module and lib/ it compiles with, else the program
	keeper: Option<cache::Keeper>,
	/// the engine of the run, for the modules it runs itself (os.run_wasm) and the code it compiles (os.compile_wasm)
	engine: Engine,
	/// the modules and components it loaded (wasm_modules.rs): by `use wasm`, and those its imports name
	wasm: wasm_modules::Loaded,
}

impl CallsC for Host {
	fn c_handles(&mut self) -> &mut CHandles {
		&mut self.c
	}
}

/// The compiler module of this warp-host's runs: compiler-self.wasm, or compiler.wasm with `--compiler full`
static COMPILER: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();

/// How a run ended
enum Ran {
	Value(Node),
	Exited(i32),
	/// a trap of the machine's own (`wasm trap: …`), or a module that could not run
	Failed(String),
	/// an uncaught Warp error: a raise, or a runtime error such as index out of range (its message)
	Raised(String),
}

fn main() {
	let mut args = std::env::args();
	let name = args.next().unwrap_or_default();
	let args = args.collect();
	// an executable `build` wrote runs the program it carries with all its arguments; else warp-host's commands
	let run = move || match aot::carried() {
		Some(machine_code) => run_carried(&machine_code, [vec![name], args].concat()),
		None => command(args),
	};
	let worker = std::thread::Builder::new().stack_size(THREAD_STACK).spawn(run);
	std::process::exit(worker.map_or(1, |worker| worker.join().unwrap_or(1)))
}

fn command(given: Vec<String>) -> i32 {
	// options before the file: --compiler, and the launcher's (--no-hints, --fuel n …), which warp-host ignores
	let (mut full, mut aot, mut args, mut given) = (false, false, Vec::<String>::new(), given.into_iter());
	while let Some(word) = given.next() {
		let before_file = args.is_empty()
			|| matches!(args.as_slice(), [command] if command == "run" || command == "compile" || command == "build");
		match word.as_str() {
			"--help" | "-h" if before_file => args.push("help".into()),
			"--aot" if before_file => aot = true,
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
	let _ = COMPILER.set(compiler);
	match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
		["run", source, ..] => run_source(&engine, compiler, source, &args[2..]),
		["compile", source] if aot => {
			compile_ahead(&engine, compiler, source, &Path::new(source).with_extension("wasm"))
		}
		["compile", source, wasm] if aot => compile_ahead(&engine, compiler, source, Path::new(wasm)),
		["compile", source] => {
			compile_file(&engine, compiler, Path::new(source), &Path::new(source).with_extension("wasm"))
		}
		["compile", source, wasm] => compile_file(&engine, compiler, Path::new(source), Path::new(wasm)),
		["build", source] => build(&engine, compiler, source, &executable_name(source)),
		["build", source, "-o" | "--output", output] | ["build", "-o" | "--output", output, source] => {
			build(&engine, compiler, source, Path::new(output))
		}
		["help", ..] => {
			println!("{USAGE}");
			0
		}
		[file, ..] if aot::is_machine_code(file) => match std::fs::read(file) {
			Ok(machine_code) => match aot::load(&engine, &machine_code, file) {
				Ok((module, functions)) => shown(run_module(
					&engine,
					&module,
					&ffi::CImports { described: functions, ..Default::default() },
					args.clone(),
					None,
				)),
				Err(problem) => failed(&problem),
			},
			Err(problem) => failed(&format!("{file}: {problem}")),
		},
		[file, ..] if is_module(file) => match module_bytes(file) {
			Ok(bytes) => shown(run_wasm(&engine, &bytes, args.clone(), None, false)),
			Err(problem) => failed(&format!("{file}: {problem}")),
		},
		// no command: run, as `warp f.warp` and `warp '6*7'` run
		[source, ..] if !COMMANDS.contains(source) => run_source(&engine, compiler, source, &args[1..]),
		_ => {
			eprintln!("{USAGE}");
			2
		}
	}
}

/// A module to run: a .wasm or .wat file by its name, else a file starting with wasm's magic bytes (`\0asm`)
fn is_module(file: &str) -> bool {
	use std::io::Read;
	let mut magic = [0u8; 4];
	let starts_as_wasm = Path::new(file).is_file()
		&& std::fs::File::open(file).and_then(|mut opened| opened.read_exact(&mut magic)).is_ok()
		&& &magic == b"\0asm";
	file.ends_with(".wasm") || file.ends_with(".wat") || starts_as_wasm
}

/// A module file's bytes; WAT text only in a build with the feature wat
fn module_bytes(file: &str) -> Result<Vec<u8>, String> {
	let bytes = std::fs::read(file).map_err(|problem| problem.to_string())?;
	if !file.ends_with(".wat") {
		return Ok(bytes);
	}
	#[cfg(feature = "wat")]
	return wat::parse_bytes(&bytes).map(|bytes| bytes.to_vec()).map_err(|problem| problem.to_string());
	#[cfg(not(feature = "wat"))]
	Err("this warp-host reads no WAT text: build it with --features wat, or assemble the module (wat2wasm)".into())
}

/// warp's engine settings (GC, typed function references; exceptions come with GC), the deep stack the compiler needs
fn engine() -> wasmtime::Result<Engine> {
	let mut config = warp_runtime::engine::deterministic_config();
	config.async_stack_size(WASM_STACK).max_wasm_stack(WASM_STACK);
	Engine::new(&config)
}

/// `run f.warp`, `run <code>` (a program of the current folder): compiled to a module of its own, then run. As warp
/// decides: a name with a warp extension is a file, and a missing one an error, not code; any other existing file (a
/// `#!` script) is a file too, else the word is code
fn run_source(engine: &Engine, compiler: &str, source: &str, args: &[String]) -> i32 {
	let named_warp =
		Path::new(source).extension().is_some_and(|extension| WARP_EXTENSIONS.iter().any(|warp| extension == *warp));
	if named_warp && !Path::new(source).is_file() {
		return failed(&format!("could not read file {source}"));
	}
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
		Ok(bytes) => shown(run_wasm(engine, &bytes, [vec![source.to_string()], args.to_vec()].concat(), None, false)),
		Err(problem) => refused(&problem),
	}
}

/// The machine code of a program or module, as wasm compiled it, for this warp-host and machine: a program's module
/// written to `wasm` as compile writes it
fn machine_code(engine: &Engine, compiler: &str, source: &str, wasm: &Path) -> Result<Vec<u8>, String> {
	let bytes = if is_module(source) {
		module_bytes(source).map_err(|problem| format!("{source}: {problem}"))?
	} else if !Path::new(source).is_file() {
		return Err(format!("could not read file {source}"));
	} else {
		compile(engine, compiler, Path::new(source), wasm, None)?;
		std::fs::read(wasm).map_err(|problem| format!("{}: {problem}", wasm.display()))?
	};
	aot::machine_code(engine, &bytes)
}

/// `compile --aot f.warp [f.wasm]`: the module and its machine code beside it, f.cwasm (of a module: only that)
fn compile_ahead(engine: &Engine, compiler: &str, source: &str, wasm: &Path) -> i32 {
	let cwasm = wasm.with_extension(aot::EXTENSION);
	let written = machine_code(engine, compiler, source, wasm).and_then(|code| {
		std::fs::write(&cwasm, &code).map(|()| code.len()).map_err(|e| format!("{}: {e}", cwasm.display()))
	});
	match written {
		Ok(size) => {
			println!("compiled {size} bytes of machine code to {}", cwasm.display());
			0
		}
		Err(problem) => refused(&problem),
	}
}

/// `build f.warp [-o app]`: the standalone executable, f beside f.warp (f.exe on Windows) unless -o names it
fn build(engine: &Engine, compiler: &str, source: &str, output: &Path) -> i32 {
	if output == Path::new(source) {
		return refused(&format!("build would write the executable over {source}: name another with -o"));
	}
	let scratch = std::env::temp_dir().join(format!("warp-host-{}.wasm", std::process::id()));
	let code = machine_code(engine, compiler, source, &scratch);
	let _ = std::fs::remove_file(&scratch);
	match code.and_then(|code| aot::write_executable(&code, output)) {
		Ok(size) => {
			println!("wrote {} ({size} bytes)", output.display());
			0
		}
		Err(problem) => refused(&problem),
	}
}

fn executable_name(source: &str) -> PathBuf {
	Path::new(source).with_extension(std::env::consts::EXE_EXTENSION)
}

/// The program an executable carries, run with the executable's arguments as a program's (its name first); its
/// value printed, as `warp-host f.warp` prints it
fn run_carried(machine_code: &[u8], args: Vec<String>) -> i32 {
	let engine = match engine() {
		Ok(engine) => engine,
		Err(problem) => return failed(&format!("{problem:#}")),
	};
	match aot::load(&engine, machine_code, &args[0]) {
		Ok((module, functions)) => shown(run_module(
			&engine,
			&module,
			&ffi::CImports { described: functions, ..Default::default() },
			args,
			None,
		)),
		Err(problem) => failed(&problem),
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
	if std::env::var_os("WARP_HOST_DEBUG").is_some() {
		eprintln!("warp-host: {} with lib/ {}, kept values {keeper}", compiler.display(), lib.display());
	}
	let argv = [compiler.as_path(), &source, &absolute(wasm), &folder, &lib].map(|path| path.display().to_string());
	let keeper = cache::Keeper { key: keeper, library: argv[4].clone() };
	match run_wasm(engine, &bytes, argv.to_vec(), Some(keeper), true) {
		Ran::Value(Node::Error(message)) => Err(String::from_utf8_lossy(&message).into_owned()),
		Ran::Failed(message) | Ran::Raised(message) => Err(message),
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
/// The compiler beside warp-host comes first: the one a harness staged there or `warp rebuild` wrote into target/, not
/// a checkout's data/ one, which may be older than its lib/ (a data/compiler-self.wasm without C imports refused every
/// C call as "outside the core"); then lib/../data, then <prefix>/share/warp
fn compiler_file(lib: &Path, name: &str) -> Option<PathBuf> {
	let beside = installed_folders().pop().into_iter().collect::<Vec<_>>();
	let folders = [beside, vec![lib.join("../data")], installed_folders()].concat();
	folders.into_iter().map(|folder| folder.join(name)).find(|file| file.is_file())
}

fn installed_folders() -> Vec<PathBuf> {
	let beside = std::env::current_exe().ok().and_then(|exe| exe.canonicalize().ok()?.parent().map(Path::to_path_buf));
	beside.map(|folder| vec![folder.join("../share/warp"), folder]).unwrap_or_default()
}

/// `run_wasm(bytes)` (os.run_wasm): a module run in this process with the same engine and imports as a program, its
/// machine code cached by its bytes, in a store of its own on a thread of its own (its own deep stack); its main's
/// value, else the trap's or the exit status's reason; an uncaught Warp error is that Error, passed through as the
/// module's value, a trap a failure of its own
pub fn ran_module(engine: &Engine, bytes: Vec<u8>, args: Vec<String>) -> Result<Node, String> {
	let engine = engine.clone();
	match on_own_stack(move || run_wasm(&engine, &bytes, args, None, true))? {
		Ran::Value(value) => Ok(value),
		Ran::Exited(0) => Ok(Node::Empty),
		Ran::Exited(status) => Err(format!("the module exited with status {status}")),
		Ran::Raised(message) => Ok(Node::Error(message.into_bytes())),
		Ran::Failed(message) => Err(message),
	}
}

/// `compile_wasm(code)` (os.compile_wasm): the code, a program of the current folder, compiled by this warp-host's
/// compiler in this process; the module's bytes, as read_bytes gives them, or the compiler's refusal
pub fn compiled_code(engine: &Engine, code: String) -> Result<Vec<u8>, String> {
	static COMPILES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
	let number = COMPILES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
	let scratch = std::env::temp_dir().join(format!("warp-host-{}-{number}", std::process::id()));
	let (source, wasm) = (scratch.with_extension("warp"), scratch.with_extension("wasm"));
	std::fs::write(&source, code).map_err(|problem| format!("{}: {problem}", source.display()))?;
	let engine = engine.clone();
	let compiler = COMPILER.get().copied().unwrap_or("compiler-self.wasm");
	let (from, to) = (source.clone(), wasm.clone());
	let compiled = on_own_stack(move || compile(&engine, compiler, &from, &to, std::env::current_dir().ok()));
	let bytes = compiled.and_then(|compiled| compiled).and_then(|()| std::fs::read(&wasm).map_err(|e| e.to_string()));
	let _ = std::fs::remove_file(&source);
	let _ = std::fs::remove_file(&wasm);
	bytes
}

/// `work` on a thread with the stack a run needs: a module run from inside a host word starts its wasm stack afresh
fn on_own_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
	let worker = std::thread::Builder::new().stack_size(THREAD_STACK).spawn(work).map_err(|e| e.to_string())?;
	worker.join().map_err(|_| "the run's thread failed".to_string())
}

/// Run a module's main (else _start) with WASI (stdio, environment, the file system at / and .) and the host's words;
/// `cached`: its machine code from the cache (cache::module)
fn run_wasm(engine: &Engine, bytes: &[u8], args: Vec<String>, keeper: Option<cache::Keeper>, cached: bool) -> Ran {
	let loaded = || -> wasmtime::Result<(Module, ffi::CImports)> {
		let module = if cached { cache::module(engine, bytes)? } else { Module::new(engine, bytes)? };
		Ok((module, ffi::CImports::of(bytes)?))
	};
	match loaded() {
		Ok((module, c)) => run_module(engine, &module, &c, args, keeper),
		Err(problem) => Ran::Failed(format!("{problem:#}")),
	}
}

/// Run a compiled module (run_wasm), the C functions `c` describes linked with its other C imports
fn run_module(
	engine: &Engine,
	module: &Module,
	c: &ffi::CImports,
	args: Vec<String>,
	keeper: Option<cache::Keeper>,
) -> Ran {
	let ran = || -> wasmtime::Result<Ran> {
		let mut wasi = WasiCtxBuilder::new();
		wasi.inherit_stdio().inherit_env().args(&args);
		wasi.preopened_dir("/", "/", FsPerms::ReadWrite)?;
		wasi.preopened_dir(".", ".", FsPerms::ReadWrite)?;
		let host = Host {
			wasi: wasi.build_p1(),
			c: CHandles::default(),
			scratch: None,
			args: args.clone(),
			keeper,
			engine: engine.clone(),
			wasm: Default::default(),
		};
		let mut store = Store::new(engine, host);
		warp_runtime::engine::cap_memory(&mut store);
		let instance = wasm_modules::instantiate(&mut store.as_context_mut(), module, c, None)?;
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
				None => failure(&error, &mut store, &instance),
			},
		})
	};
	ran().unwrap_or_else(|problem| Ran::Failed(format!("{problem:#}")))
}

use wasmtime::AsContextMut;

/// The imports every module of a run gets: WASI p1, warp-runtime's host words, libm, libc, std_io
pub fn program_linker(engine: &Engine) -> wasmtime::Result<Linker<Host>> {
	let mut linker = Linker::new(engine);
	wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |host: &mut Host| &mut host.wasi)?;
	warp_runtime::host_words::link_host_words(&mut linker)?;
	warp_runtime::libm::link_libm(&mut linker)?;
	warp_runtime::c_calls::link_libc_functions(&mut linker)?;
	words::link(&mut linker)?;
	Ok(linker)
}

/// A failed run's message: a runtime error fails inside a function of its name (index_out_of_range, …), the global
/// trap_detail holding the value it names; a raise's message is that value (lib/compiler/runtime/errors.warp)
fn failure(error: &wasmtime::Error, store: &mut Store<Host>, instance: &Instance) -> Ran {
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
		(Some("returned error") | None, Some(detail)) => Ran::Raised(detail.content()),
		(Some(name), Some(detail)) => Ran::Raised(format!("{name}: {}", detail.shown())),
		(Some(name), None) if !name.starts_with("wasm") => Ran::Raised(name.to_string()),
		// a trap of the machine's own, without the backtrace: `wasm trap: wasm `unreachable` instruction executed`
		_ => Ran::Failed(match error.downcast_ref::<wasmtime::Trap>() {
			Some(trap) => trap.to_string(),
			None => format!("{:#}", error),
		}),
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
		Ran::Failed(message) | Ran::Raised(message) => failed(&message),
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
