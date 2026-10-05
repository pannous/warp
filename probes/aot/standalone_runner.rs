// A standalone executable of one warp program: wasmtime's runtime without a compiler (no Cranelift) and the
// program's machine code (`warp compile --aot`, a .cwasm) embedded. Built by probes/aot/standalone.sh.
// The engine settings must match the engine that compiled the .cwasm (util::deterministic_config + fuel).
use std::time::Instant;
use wasmtime::{Config, Engine, Linker, Module, Store, Val};

const MACHINE_CODE: &[u8] = include_bytes!(env!("WARP_CWASM"));
const FUEL: u64 = 10_000_000_000;
const GC_HEAP_INITIAL_BYTES: u64 = 1 << 30;

fn main() -> wasmtime::Result<()> {
	let start = Instant::now();
	let mut config = Config::new();
	config.wasm_gc(true);
	config.wasm_function_references(true);
	config.gc_heap_initial_size(GC_HEAP_INITIAL_BYTES);
	config.consume_fuel(true);
	let engine = Engine::new(&config)?;
	// SAFETY: the embedded machine code is warp's own output for this wasmtime version
	let module = unsafe { Module::deserialize(&engine, MACHINE_CODE)? };
	let loaded = start.elapsed();
	let mut store = Store::new(&engine, ());
	store.set_fuel(FUEL)?;
	let instance = Linker::new(&engine).instantiate(&mut store, &module)?;
	let main = instance.get_func(&mut store, "main").expect("a main function");
	let mut results = vec![Val::AnyRef(None); main.ty(&store).results().len()];
	main.call(&mut store, &[], &mut results)?;
	let result = match results.first() {
		Some(Val::I64(number)) => number.to_string(),
		Some(Val::I32(number)) => number.to_string(),
		Some(Val::AnyRef(Some(_))) => "<node>".to_string(),
		_ => "<none>".to_string(),
	};
	eprintln!("result {result}, loaded in {loaded:?}, total {:?}", start.elapsed());
	Ok(())
}
