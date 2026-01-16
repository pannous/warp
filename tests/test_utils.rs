use anyhow::Result;
/// Test utilities for fast WASM GC testing
/// Provides shared Engine to avoid expensive per-test Engine creation
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;
use wasmtime::{Config, Engine, Linker, Module, Store};
use warp::GcObject;

/// Shared WASM engine with GC support (created once, reused across all tests)
/// This is thread-safe and significantly speeds up tests by avoiding repeated Engine creation
pub static SHARED_GC_ENGINE: Lazy<Engine> = Lazy::new(|| {
	let mut config = Config::new();
	config.wasm_gc(true);
	config.wasm_function_references(true);
	Engine::new(&config).expect("Failed to create shared GC engine")
});

/// Module cache: maps bytecode hash to compiled Module
/// Avoids expensive recompilation of identical WASM modules across tests
static MODULE_CACHE: Lazy<Mutex<HashMap<u64, Module>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// Cache statistics for performance analysis
static CACHE_STATS: Lazy<Mutex<CacheStats>> = Lazy::new(|| Mutex::new(CacheStats::new()));

struct CacheStats {
	hits: usize,
	misses: usize,
}

impl CacheStats {
	fn new() -> Self {
		CacheStats { hits: 0, misses: 0 }
	}

	fn hit_rate(&self) -> f64 {
		let total = self.hits + self.misses;
		if total == 0 {
			0.0
		} else {
			self.hits as f64 / total as f64
		}
	}
}

/// Get cache statistics (useful for debugging/profiling)
pub fn cache_stats() -> (usize, usize, f64) {
	let stats = CACHE_STATS.lock().unwrap();
	(stats.hits, stats.misses, stats.hit_rate())
}

/// Create a new Store from the shared engine (fast, per-test isolation)
pub fn new_store() -> Store<()> {
	Store::new(&SHARED_GC_ENGINE, ())
}

/// Simple hash function for byte slices
fn hash_bytes(bytes: &[u8]) -> u64 {
	use std::collections::hash_map::DefaultHasher;
	use std::hash::{Hash, Hasher};
	let mut hasher = DefaultHasher::new();
	bytes.hash(&mut hasher);
	hasher.finish()
}

/// Fast path: compile module using shared engine with caching
pub fn compile_module(bytes: &[u8]) -> Result<Module> {
	let hash = hash_bytes(bytes);

	// Try to get from cache first
	{
		let cache = MODULE_CACHE.lock().unwrap();
		if let Some(module) = cache.get(&hash) {
			CACHE_STATS.lock().unwrap().hits += 1;
			return Ok(module.clone());
		}
	}

	// Cache miss - compile and store
	CACHE_STATS.lock().unwrap().misses += 1;
	let module = Module::new(&SHARED_GC_ENGINE, bytes)?;

	{
		let mut cache = MODULE_CACHE.lock().unwrap();
		cache.insert(hash, module.clone());
	}

	Ok(module)
}

/// Fast ergonomic reader using shared engine
pub fn read_bytes_fast(bytes: &[u8]) -> Result<GcObject> {
	let module = compile_module(bytes)?;
	let store = new_store();
	let store_rc = Rc::new(RefCell::new(store));

	let linker = Linker::new(&SHARED_GC_ENGINE);
	let instance = {
		let mut s = store_rc.borrow_mut();
		linker.instantiate(&mut *s, &module)?
	};

	// Call main() to get the root node
	let main = {
		let mut s = store_rc.borrow_mut();
		instance
			.get_func(&mut *s, "main")
			.ok_or_else(|| anyhow::anyhow!("No main function"))?
	};

	let mut results = vec![wasmtime::Val::I32(0)];
	{
		let mut s = store_rc.borrow_mut();
		main.call(&mut *s, &[], &mut results)?;
	}

	Ok(GcObject::new(results[0], store_rc, instance))
}

/// Fast path for reading WASM file using shared engine
pub fn read_wasm_fast(path: &str) -> Result<GcObject> {
	let bytes = std::fs::read(path)?;
	read_bytes_fast(&bytes)
}
