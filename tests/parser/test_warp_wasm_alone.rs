//! `wasmtime run --default-values-unknown-imports compiler-self.wasm hello.warp hello.wasm <folder> <lib>`: the
//! self-compiled warp compiler runs under plain wasmtime with WASI preview 1, no warp host, and the module it writes
//! runs there too (card standalone, decision wasm-standalone)
use std::path::{Path, PathBuf};
use wasmtime::{Config, Engine, Linker, Module, Store};
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;
use wasmtime_wasi::{p1, FsPerms, I32Exit, WasiCtxBuilder};

const HELLO: &str = "print(\"hello world\")\n";
const OUTPUT_CAPACITY: usize = 1 << 20;

struct WasiRun {
	status: i32,
	stdout: String,
	stderr: String,
}

/// The command module's `_start` under wasmtime with WASI p1 and no warp host: these arguments, `/` and `folder` (as
/// `.`) preopened
fn run_wasi(module: &Path, arguments: &[&str], folder: &Path) -> WasiRun {
	let mut config = Config::new();
	config.wasm_gc(true).wasm_function_references(true).wasm_exceptions(true);
	let engine = Engine::new(&config).unwrap();
	let module = Module::from_file(&engine, module).unwrap();
	let (stdout, stderr) = (MemoryOutputPipe::new(OUTPUT_CAPACITY), MemoryOutputPipe::new(OUTPUT_CAPACITY));
	let mut context = WasiCtxBuilder::new();
	context.args(arguments).stdout(stdout.clone()).stderr(stderr.clone());
	context.preopened_dir("/", "/", FsPerms::ReadWrite).unwrap();
	context.preopened_dir(folder, ".", FsPerms::ReadWrite).unwrap();
	let mut store = Store::new(&engine, context.build_p1());
	let mut linker: Linker<p1::WasiP1Ctx> = Linker::new(&engine);
	p1::add_to_linker_sync(&mut linker, |context| context).unwrap();
	// the host words it may import (kept, keep, local_offset) answer with defaults, as `wasmtime run
	// --default-values-unknown-imports` does: nothing kept, every run cold (notes/standalone.md)
	linker.define_unknown_imports_as_default_values(&mut store, &module).unwrap();
	let instance = linker.instantiate(&mut store, &module).unwrap_or_else(|failure| panic!("instantiate: {failure:?}"));
	let start = instance.get_typed_func::<(), ()>(&mut store, "_start").unwrap();
	let status = match start.call(&mut store, ()) {
		Ok(()) => 0,
		Err(trap) => trap.downcast_ref::<I32Exit>().map(|exit| exit.0).unwrap_or_else(|| panic!("trap: {trap:?}")),
	};
	drop(store);
	let text = |pipe: &MemoryOutputPipe| String::from_utf8_lossy(&pipe.contents()).into_owned();
	WasiRun { status, stdout: text(&stdout), stderr: text(&stderr) }
}

/// `warp rebuild self` for this checkout's lib/: the path of compiler-self.wasm
fn self_compiled_compiler(library: &Path) -> PathBuf {
	let run = crate::common::warp_command()
		.args(["--no-ask", "rebuild", "self"])
		.env("WARP_LIB", library)
		.env("WARP_TIME_LIMIT", super::test_warp_pure::SELF_BUILD_TIME_LIMIT_SECONDS)
		.env("WARP_NO_WINDOW", "1")
		.env("WARP_HINTS", "0")
		.output()
		.expect("warp runs");
	let printed = String::from_utf8_lossy(&run.stdout);
	let built = printed.lines().find_map(|line| line.strip_prefix("built "));
	PathBuf::from(built.unwrap_or_else(|| panic!("{printed}\n{}", String::from_utf8_lossy(&run.stderr))))
}

#[test]
#[ignore = "slow: builds compiler-self.wasm like test_warp_pure; run with --ignored"]
fn compiler_wasm_runs_under_plain_wasmtime() {
	let folder = crate::common::scratch_directory("warp_wasm_alone");
	std::fs::create_dir_all(&folder).unwrap();
	let program = folder.join("hello.warp");
	let output = folder.join("hello.wasm");
	std::fs::write(&program, HELLO).unwrap();
	let _ = std::fs::remove_file(&output);
	let library = Path::new(env!("CARGO_MANIFEST_DIR")).join("lib");
	let compiler = self_compiled_compiler(&library);
	let paths = [&compiler, &program, &output, &folder, &library].map(|path| path.display().to_string());
	let compiled = run_wasi(&compiler, &paths.each_ref().map(String::as_str), &folder);
	assert!(output.exists(), "status {}\n{}\n{}", compiled.status, compiled.stdout, compiled.stderr);
	let hello = run_wasi(&output, &["hello.wasm"], &folder);
	assert_eq!(hello.stdout.lines().next(), Some("hello world"), "{}", hello.stderr);
}
