// ⚠️ TEST WITH  cargo test --features optimizer
// A `try` compiles to wasm exception handling (try_table, throw, tag section): wasm-opt must accept and keep it
#![cfg(feature = "optimizer")]

use warp::wasm_optimizer::{OptimizationMode, WasmOptimizer};
use warp::{wasm_emitter::compile, wasm_reader::read_bytes, Node, Number};

#[test]
fn optimized_try_still_catches() {
	if !WasmOptimizer::tools_available() {
		eprintln!("Skipping: wasm-opt not found - install binaryen");
		return;
	}
	let module = compile("try 1 + [1 2]#5 else 7").unwrap_or_else(|error| panic!("{error:?}"));
	let optimized = WasmOptimizer::library(OptimizationMode::Speed).optimize(&module.bytes).unwrap();
	assert_eq!(read_bytes(&optimized).unwrap(), Node::Number(Number::Int(7)));
}
