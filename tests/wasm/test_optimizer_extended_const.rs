// ⚠️ TEST WITH  cargo test --features optimizer
// Extended constant expressions (Wasm 3.0) are on by default: a global initialised by arithmetic passes wasm-opt
#![cfg(feature = "optimizer")]

use warp::wasm_optimizer::{OptimizationMode, WasmOptimizer};

const GLOBAL_BY_ARITHMETIC: &str = r#"(module (global $g i64 (i64.add (i64.const 40) (i64.const 2)))
	(func (export "main") (result i64) global.get $g))"#;

#[test]
fn optimizer_keeps_extended_constant_expressions() {
	if !WasmOptimizer::tools_available() {
		eprintln!("Skipping: wasm-opt not found - install binaryen");
		return;
	}
	let module = wat::parse_str(GLOBAL_BY_ARITHMETIC).unwrap();
	let optimized = WasmOptimizer::library(OptimizationMode::Speed).optimize(&module).unwrap();
	wasmparser::Validator::new().validate_all(&optimized).unwrap();
}
