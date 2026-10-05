use std::fs;
use std::path::PathBuf;
use warp::gc_engine;
use warp::run::module_cache::{compiled_module_in, Origin};
use wasmtime::{Linker, Module};

const ANSWER_MODULE: &str = r#"(module (func (export "main") (result i64) i64.const 42))"#;

const OUTPUT_DIRECTORY: &str = env!("CARGO_TARGET_TMPDIR");

/// An empty cache directory of its own
fn fresh_cache(name: &str) -> PathBuf {
	let directory = PathBuf::from(OUTPUT_DIRECTORY).join("compiled_module_cache").join(name);
	let _ = fs::remove_dir_all(&directory);
	directory
}

fn main_result(engine: &wasmtime::Engine, module: &Module) -> i64 {
	let mut store = warp::util::fueled_store(engine, ());
	let instance = Linker::new(engine).instantiate(&mut store, module).unwrap();
	instance.get_typed_func::<(), i64>(&mut store, "main").unwrap().call(&mut store, ()).unwrap()
}

#[test]
fn test_compiled_module_cache_compiles_once_then_maps_the_compiled_module() {
	let cache = fresh_cache("compiles_once");
	let engine = gc_engine();
	let (compiled, first) = compiled_module_in(&cache, &engine, ANSWER_MODULE.as_bytes()).unwrap();
	assert_eq!(first, Origin::Compiled);
	let (cached, second) = compiled_module_in(&cache, &engine, ANSWER_MODULE.as_bytes()).unwrap();
	assert_eq!(second, Origin::Cached);
	assert_eq!(main_result(&engine, &compiled), 42);
	// another engine with the same settings reuses the module compiled for the first one
	let mut config = warp::util::deterministic_config();
	config.consume_fuel(true);
	let other_engine = wasmtime::Engine::new(&config).unwrap();
	let (reused, origin) = compiled_module_in(&cache, &other_engine, ANSWER_MODULE.as_bytes()).unwrap();
	assert_eq!(origin, Origin::Cached);
	assert_eq!(main_result(&other_engine, &reused), 42);
	assert_eq!(main_result(&engine, &cached), 42);
}

#[test]
fn test_compiled_module_cache_keys_on_the_engine_settings() {
	let cache = fresh_cache("engine_settings");
	compiled_module_in(&cache, &gc_engine(), ANSWER_MODULE.as_bytes()).unwrap();
	// the task engine adds epoch interruption: other machine code, so another cache entry
	let (_, origin) = compiled_module_in(&cache, &warp::util::task_engine(), ANSWER_MODULE.as_bytes()).unwrap();
	assert_eq!(origin, Origin::Compiled);
}

#[test]
fn test_compiled_module_cache_recompiles_a_damaged_entry() {
	let cache = fresh_cache("damaged");
	let engine = gc_engine();
	compiled_module_in(&cache, &engine, ANSWER_MODULE.as_bytes()).unwrap();
	for entry in fs::read_dir(&cache).unwrap() {
		fs::write(entry.unwrap().path(), b"not a compiled module").unwrap();
	}
	let (module, origin) = compiled_module_in(&cache, &engine, ANSWER_MODULE.as_bytes()).unwrap();
	assert_eq!(origin, Origin::Compiled);
	assert_eq!(main_result(&engine, &module), 42);
	let (_, origin) = compiled_module_in(&cache, &engine, ANSWER_MODULE.as_bytes()).unwrap();
	assert_eq!(origin, Origin::Cached);
}

#[test]
fn test_compiled_module_cache_holds_compiled_programs() {
	let cache = fresh_cache("programs");
	let engine = gc_engine();
	let program = warp::wasm_emitter::compile("fib := it < 2 ? it : fib(it - 1) + fib(it - 2); fib(10)").unwrap();
	let (_, first) = compiled_module_in(&cache, &engine, &program.bytes).unwrap();
	let (_, second) = compiled_module_in(&cache, &engine, &program.bytes).unwrap();
	assert_eq!((first, second), (Origin::Compiled, Origin::Cached));
}

/// `warp compile --aot` writes the machine code next to the .wasm; `warp <file>.cwasm` runs it with its imports
#[test]
fn test_compile_aot_writes_machine_code_that_runs() {
	let source = PathBuf::from(OUTPUT_DIRECTORY).join("compile_aot_print.warp");
	fs::write(&source, "print \"hello\"; 6*7").unwrap();
	let machine_code = source.with_extension("cwasm");
	let _ = fs::remove_file(&machine_code);
	let compiled = crate::common::warp_command().args(["compile", "--aot"]).arg(&source).output().unwrap();
	assert!(compiled.status.success(), "{}", String::from_utf8_lossy(&compiled.stderr));
	assert!(source.with_extension("wasm").exists());
	let run = crate::common::warp_command().arg(&machine_code).output().unwrap();
	let printed = String::from_utf8_lossy(&run.stdout);
	assert!(printed.contains("hello") && printed.contains("42"), "{printed}{}", String::from_utf8_lossy(&run.stderr));
}

/// every run shares one gc_engine, so a module compiled once in a process runs again without compiling
#[test]
fn test_gc_engine_is_shared() {
	assert!(wasmtime::Engine::same(&gc_engine(), &gc_engine()));
	assert!(!wasmtime::Engine::same(&warp::util::task_engine(), &warp::util::task_engine()));
}
