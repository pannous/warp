use wasmtime::{Config, Engine, Store};

/// The GC heap a run starts with: reserved, committed lazily by the OS; a heap starting empty collects (walking every
/// frame of a deep recursion) at each of its many small growths
const GC_HEAP_INITIAL_BYTES: u64 = 1 << 30;

/// Engine settings every wasp run shares: GC, typed function references, and canonical NaNs so float
/// results are bit-identical on every CPU and engine (a NaN produced by arithmetic is always 0x7ff8000000000000).
/// Machine code compiled with these settings (`warp compile --aot`, `warp build`) loads into any engine made
/// from them, with or without a compiler.
pub fn deterministic_config() -> Config {
	let mut config = Config::new();
	config.wasm_gc(true);
	config.wasm_function_references(true);
	#[cfg(feature = "compiler")]
	config.cranelift_nan_canonicalization(true);
	config.gc_heap_initial_size(GC_HEAP_INITIAL_BYTES);
	// warp runs core modules, no components: the machine code then also loads into a runtime built without the
	// component model (the warp-runtime stub), where it is off anyway
	#[cfg(feature = "compiler")]
	config.wasm_component_model(false);
	config.concurrency_support(false);
	config
}

/// deterministic_config with fuel metering: the settings of warp's gc_engine
pub fn fueled_config() -> Config {
	let mut config = deterministic_config();
	config.consume_fuel(true);
	config
}

/// A store with `fuel` steps to run
pub fn fueled_store<T>(engine: &Engine, data: T, fuel: u64) -> Store<T> {
	let mut store = Store::new(engine, data);
	store.set_fuel(fuel).expect("the engine consumes fuel");
	store
}
