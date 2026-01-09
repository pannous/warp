use serde_json::json;
use wasmtime::{Config, Engine};

pub fn fetch(p0: &str) -> String {
	ureq::get(p0)
		.call()
		.unwrap()
		.body_mut()
		.read_to_string()
		.unwrap()
}

/// Create a WASM engine with GC and function references enabled.
/// This is the standard configuration for all wasp WASM operations.
pub fn gc_engine() -> Engine {
	let mut config = Config::new();
	config.wasm_gc(true);
	config.wasm_function_references(true);
	Engine::new(&config).expect("Failed to create WASM engine")
}


pub fn show_type_name<T>(_: &T) {
	use std::any::{type_name, type_name_of_val};
	// println!("{}", type_name_of_val(*json!({"name": "Alice"})));
	println!("{}", type_name::<T>());
}
