//! Probe (g-qV-s first slice): two Modules + one Store via Linker share a tiny runtime.
//! Import module name is exactly `warp_runtime` (no component model / WIT).
//! Proves the Linker path; production INT_RUNTIME / emit split is still Later (notes/aot.md Rec §4).
//! Built and run by probes/aot/shared_runtime.sh.
use std::path::PathBuf;
use wasmtime::{Config, Engine, Linker, Module, Store};

/// Tiny shared runtime: a few plain i32 helpers (stand-in for warp's INT_RUNTIME).
const RUNTIME_WAT: &str = r#"
(module
  (func (export "add_i32") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add)
  (func (export "mul_i32") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.mul)
)
"#;

/// Program that imports from `warp_runtime` instead of baking helpers in.
const PROGRAM_WAT: &str = r#"
(module
  (import "warp_runtime" "add_i32" (func $add (param i32 i32) (result i32)))
  (import "warp_runtime" "mul_i32" (func $mul (param i32 i32) (result i32)))
  (func (export "main") (result i32)
    i32.const 2
    i32.const 3
    call $add
    i32.const 7
    call $mul)
)
"#;

/// Same helpers inlined (monolithic) for an optional .cwasm size contrast.
const MONOLITHIC_WAT: &str = r#"
(module
  (func $add (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add)
  (func $mul (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.mul)
  (func (export "main") (result i32)
    i32.const 2
    i32.const 3
    call $add
    i32.const 7
    call $mul)
)
"#;

fn engine() -> wasmtime::Result<Engine> {
	// Match crates/warp-runtime: GC + fuel knobs ready; this probe uses plain i32 only.
	let mut config = Config::new();
	config.wasm_gc(true);
	config.wasm_function_references(true);
	config.consume_fuel(true);
	config.concurrency_support(false);
	Engine::new(&config)
}

fn main() -> wasmtime::Result<()> {
	let engine = engine()?;
	let mut store = Store::new(&engine, ());
	store.set_fuel(1_000_000)?;

	let runtime = Module::new(&engine, RUNTIME_WAT)?;
	let program = Module::new(&engine, PROGRAM_WAT)?;

	let mut linker = Linker::new(&engine);
	// wasmtime 49: Linker::module (Issues said define_module; same role)
	linker.module(&mut store, "warp_runtime", &runtime)?;
	let instance = linker.instantiate(&mut store, &program)?;
	let main = instance.get_typed_func::<(), i32>(&mut store, "main")?;
	let result = main.call(&mut store, ())?;
	assert_eq!(result, 35, "expected (2+3)*7 via shared warp_runtime");
	println!("shared_runtime_linker: main -> {result}");

	// Optional: serialize each Module and contrast vs a monolithic .cwasm
	let out = scratch_dir();
	std::fs::create_dir_all(&out)?;
	let runtime_cwasm = engine.precompile_module(RUNTIME_WAT.as_bytes())?;
	let program_cwasm = engine.precompile_module(PROGRAM_WAT.as_bytes())?;
	let mono_cwasm = engine.precompile_module(MONOLITHIC_WAT.as_bytes())?;
	let runtime_path = out.join("runtime.cwasm");
	let program_path = out.join("program.cwasm");
	let mono_path = out.join("monolithic.cwasm");
	std::fs::write(&runtime_path, &runtime_cwasm)?;
	std::fs::write(&program_path, &program_cwasm)?;
	std::fs::write(&mono_path, &mono_cwasm)?;
	let shared_sum = runtime_cwasm.len() + program_cwasm.len();
	println!(
		"cwasm sizes: runtime {} B, program {} B (sum {}), monolithic {} B",
		runtime_cwasm.len(),
		program_cwasm.len(),
		shared_sum,
		mono_cwasm.len()
	);
	println!(
		"ok shared_runtime_linker (Linker path; production emit still Later)"
	);
	Ok(())
}

fn scratch_dir() -> PathBuf {
	if let Ok(root) = std::env::var("WARP_SCRATCH") {
		return PathBuf::from(root).join("aot/shared_runtime");
	}
	PathBuf::from("scratch/aot/shared_runtime")
}
