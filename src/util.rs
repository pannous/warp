use std::cell::Cell;
#[cfg(feature = "native")]
use wasmtime::{Engine, Store};

pub use warp_runtime::fuel::{DEFAULT_FUEL, FUEL_VARIABLE};
#[cfg(feature = "native")]
pub use warp_runtime::engine::deterministic_config;

thread_local! {
	static FUEL_OVERRIDE: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Fuel budget of the next runs on this thread: `with_fuel`, else `WARP_FUEL`, else `DEFAULT_FUEL`
pub fn fuel_budget() -> u64 {
	FUEL_OVERRIDE.with(Cell::get)
		.or_else(warp_runtime::fuel::fuel_from_environment)
		.unwrap_or(DEFAULT_FUEL)
}

/// Set the fuel budget for every later run on this thread (the CLI's `--fuel`)
pub fn set_fuel_budget(steps: u64) {
	FUEL_OVERRIDE.with(|budget| budget.set(Some(steps)));
}

/// Run `body` with a different fuel budget on this thread, then restore the previous one
pub fn with_fuel<R>(steps: u64, body: impl FnOnce() -> R) -> R {
	let previous = FUEL_OVERRIDE.with(|budget| budget.replace(Some(steps)));
	let result = body();
	FUEL_OVERRIDE.with(|budget| budget.set(previous));
	result
}

/// A store for a `gc_engine` with the current fuel budget
#[cfg(feature = "native")]
pub fn fueled_store<T>(engine: &Engine, data: T) -> Store<T> {
	warp_runtime::engine::fueled_store(engine, data, fuel_budget())
}

/// The body at `url`, empty (and reported on stderr) when the download fails
#[cfg(feature = "native")]
pub fn fetch(url: &str) -> String {
	crate::extensions::utils::download(url)
}

/// The engine of a program that starts tasks (tasks.rs): gc_engine's settings plus epoch interruption, the check
/// points where a task stops or pauses
#[cfg(feature = "native")]
pub fn task_engine() -> Engine {
	let mut config = warp_runtime::engine::fueled_config();
	config.epoch_interruption(true);
	Engine::new(&config).expect("Failed to create WASM engine")
}

/// The WASM engine with GC, function references, canonical NaNs and fuel metering, one per process: every run shares
/// it, so a module compiled once runs again without compiling (run/module_cache.rs); stores and their fuel stay per
/// run (`fueled_store`). Programs that start tasks get an engine of their own (task_engine): their epoch ticks would
/// reach every store of a shared engine.
#[cfg(feature = "native")]
pub fn gc_engine() -> Engine {
	static SHARED: std::sync::OnceLock<Engine> = std::sync::OnceLock::new();
	SHARED.get_or_init(|| Engine::new(&warp_runtime::engine::fueled_config()).expect("Failed to create WASM engine")).clone()
}


pub fn show_type_name<T>(_: &T) {
	use std::any::type_name;
	// println!("{}", type_name_of_val(*json!({"name": "Alice"})));
	println!("{}", type_name::<T>());
}
