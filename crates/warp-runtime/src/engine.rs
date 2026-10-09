use wasmtime::{Config, Engine, Store};

/// The GC heap a run starts with: reserved, committed lazily by the OS; a heap starting empty collects (walking every
/// frame of a deep recursion) at each of its many small growths
const GC_HEAP_INITIAL_BYTES: u64 = 1 << 30;

/// Engine settings every warp run shares: GC and typed function references. NaNs are canonicalized where they are
/// observed (floats::canonical_nan, P119), not after every float operation, which doubled the cost of float loops.
/// Machine code compiled with these settings (`warp compile --aot`, `warp build`) loads into any engine made
/// from them, with or without a compiler.
pub fn deterministic_config() -> Config {
	let mut config = Config::new();
	config.wasm_gc(true);
	config.wasm_function_references(true);
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

/// A store with `fuel` steps to run, its memories and GC heap capped (`cap_memory`)
pub fn fueled_store<T>(engine: &Engine, data: T, fuel: u64) -> Store<T> {
	let mut store = Store::new(engine, data);
	store.set_fuel(fuel).expect("the engine consumes fuel");
	cap_memory(&mut store);
	store
}

/// The size any one memory or GC heap of a run may grow to: a runaway program stops with an error instead of
/// swapping the machine to death (a 154 GB process took the Mac down, 2026-10-09)
pub const DEFAULT_MEMORY_CAP_MB: usize = 2048;
/// Environment variable that overrides `DEFAULT_MEMORY_CAP_MB`
pub const MEMORY_CAP_VARIABLE: &str = "WARP_MEMORY_CAP_MB";
const BYTES_PER_MB: usize = 1 << 20;

/// The cap in bytes: `WARP_MEMORY_CAP_MB` if it is set to a number, else the default
pub fn memory_cap_bytes() -> usize {
	static CAP: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
	*CAP.get_or_init(|| {
		let megabytes = std::env::var(MEMORY_CAP_VARIABLE).ok().and_then(|value| value.trim().parse().ok());
		megabytes.unwrap_or(DEFAULT_MEMORY_CAP_MB).saturating_mul(BYTES_PER_MB)
	})
}

/// Every growth of the store's linear memories and its GC heap (wasmtime grows the GC heap's memory through the
/// same limiter) past `memory_cap_bytes` fails the run with an error naming the cap
pub fn cap_memory<T>(store: &mut Store<T>) {
	// a zero-sized limiter: leaking its box allocates nothing
	store.limiter(|_| Box::leak(Box::new(MemoryCap)));
}

struct MemoryCap;

impl wasmtime::ResourceLimiter for MemoryCap {
	fn memory_growing(&mut self, _current: usize, desired: usize, maximum: Option<usize>) -> wasmtime::Result<bool> {
		let cap = memory_cap_bytes();
		if desired > cap {
			let refusal = format!("memory cap: growing a memory to {} MB passes the cap of {} MB ({MEMORY_CAP_VARIABLE})", desired / BYTES_PER_MB, cap / BYTES_PER_MB);
			// also on stderr: wasmtime drops this error for the GC heap and traps with "GC heap out of memory"
			eprintln!("{refusal}");
			wasmtime::bail!(refusal);
		}
		Ok(maximum.is_none_or(|maximum| desired <= maximum))
	}

	fn table_growing(&mut self, _current: usize, desired: usize, maximum: Option<usize>) -> wasmtime::Result<bool> {
		Ok(maximum.is_none_or(|maximum| desired <= maximum))
	}
}
