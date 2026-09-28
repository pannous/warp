use serde_json::json;
use std::cell::Cell;
use wasmtime::{Config, Engine, Store};

/// Fuel for one run: wasmtime burns about one unit per executed WASM instruction, so this is roughly
/// ten billion steps (several seconds of compute) before a run stops with `out of fuel after N steps`
/// instead of hanging. Raise it with the environment variable `WARP_FUEL=<steps>` or `warp --fuel <steps>`.
pub const DEFAULT_FUEL: u64 = 10_000_000_000;
/// Environment variable that overrides `DEFAULT_FUEL`
pub const FUEL_VARIABLE: &str = "WARP_FUEL";

thread_local! {
	static FUEL_OVERRIDE: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Fuel budget of the next runs on this thread: `with_fuel`, else `WARP_FUEL`, else `DEFAULT_FUEL`
pub fn fuel_budget() -> u64 {
	FUEL_OVERRIDE.with(Cell::get)
		.or_else(|| std::env::var(FUEL_VARIABLE).ok().and_then(|steps| steps.trim().replace('_', "").parse().ok()))
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

/// Engine settings every wasp run shares: GC, typed function references, and canonical NaNs so float
/// results are bit-identical on every CPU and engine (a NaN produced by arithmetic is always 0x7ff8000000000000).
pub fn deterministic_config() -> Config {
	let mut config = Config::new();
	config.wasm_gc(true);
	config.wasm_function_references(true);
	config.cranelift_nan_canonicalization(true);
	config
}

/// A store for a `gc_engine` with the current fuel budget
pub fn fueled_store<T>(engine: &Engine, data: T) -> Store<T> {
	let mut store = Store::new(engine, data);
	store.set_fuel(fuel_budget()).expect("gc_engine consumes fuel");
	store
}

pub fn fetch(p0: &str) -> String {
	ureq::get(p0)
		.call()
		.unwrap()
		.body_mut()
		.read_to_string()
		.unwrap()
}

/// Create a WASM engine with GC, function references, canonical NaNs and fuel metering.
/// This is the standard configuration for all wasp WASM operations; create its stores with `fueled_store`.
pub fn gc_engine() -> Engine {
	let mut config = deterministic_config();
	config.consume_fuel(true);
	Engine::new(&config).expect("Failed to create WASM engine")
}


pub fn show_type_name<T>(_: &T) {
	use std::any::{type_name, type_name_of_val};
	// println!("{}", type_name_of_val(*json!({"name": "Alice"})));
	println!("{}", type_name::<T>());
}
