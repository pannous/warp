pub use wasmtime_c_api::*;

/// wasm_config_t is `#[repr(C)] struct { config: Config }` with that field crate-private: the Config at its address
fn config_of(config: &mut wasm_config_t) -> &mut wasmtime::Config {
	unsafe { &mut *(config as *mut wasm_config_t as *mut wasmtime::Config) }
}

/// The GC heap a store starts with (Config::gc_heap_initial_size): a heap starting small collects over and over
#[unsafe(no_mangle)]
pub extern "C" fn warp_config_gc_heap_initial_size_set(config: &mut wasm_config_t, bytes: u64) {
	config_of(config).gc_heap_initial_size(bytes);
}

/// The collector: 1 deferred reference counting, 2 copying, anything else auto (copying, as this build has it)
#[unsafe(no_mangle)]
pub extern "C" fn warp_config_collector_set(config: &mut wasm_config_t, collector: i64) {
	let chosen = match collector {
		1 => wasmtime::Collector::DeferredReferenceCounting,
		2 => wasmtime::Collector::Copying,
		_ => wasmtime::Collector::Auto,
	};
	config_of(config).collector(chosen);
}
