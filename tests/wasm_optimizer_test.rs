use wasp::wasm_optimizer::{ExportMode, OptimizationMode, WasmOptimizer};
use std::path::Path;

#[test]
fn test_tools_availability() {
	assert!(WasmOptimizer::tools_available(), "wasm-opt not found - install binaryen");
	assert!(WasmOptimizer::tree_shaking_available(), "wasm-metadce not found");
}

#[test]
fn test_library_optimization() {
	let input_path = Path::new("out/kitchensink_char.wasm");
	if !input_path.exists() {
		eprintln!("Skipping: out/kitchensink_char.wasm not found");
		return;
	}

	let wasm_bytes = std::fs::read(input_path).unwrap();
	let original_size = wasm_bytes.len();

	// Library mode: optimize but keep all exports
	let optimizer = WasmOptimizer::library(OptimizationMode::Speed);
	let optimized = optimizer.optimize(&wasm_bytes).unwrap();

	println!(
		"Library optimization: {} -> {} bytes ({}% reduction)",
		original_size,
		optimized.len(),
		(original_size - optimized.len()) * 100 / original_size
	);

	assert!(optimized.len() <= original_size, "Optimization should not increase size");
}

#[test]
fn test_executable_tree_shaking() {
	let input_path = Path::new("out/kitchensink_char.wasm");
	if !input_path.exists() {
		eprintln!("Skipping: out/kitchensink_char.wasm not found");
		return;
	}

	let wasm_bytes = std::fs::read(input_path).unwrap();
	let original_size = wasm_bytes.len();

	// Executable mode: tree-shake to main only
	let optimizer = WasmOptimizer::executable(
		OptimizationMode::Speed,
		vec!["main".to_string()],
	);
	let optimized = optimizer.optimize(&wasm_bytes).unwrap();

	println!(
		"Executable tree-shaking: {} -> {} bytes ({}% reduction)",
		original_size,
		optimized.len(),
		(original_size - optimized.len()) * 100 / original_size
	);

	// Tree-shaking should provide significant reduction
	assert!(
		optimized.len() < original_size / 2,
		"Tree-shaking should reduce size by at least 50%"
	);
}

#[test]
fn test_no_optimization() {
	let input_path = Path::new("out/kitchensink_char.wasm");
	if !input_path.exists() {
		eprintln!("Skipping: out/kitchensink_char.wasm not found");
		return;
	}

	let wasm_bytes = std::fs::read(input_path).unwrap();

	let optimizer = WasmOptimizer {
		optimization: OptimizationMode::None,
		export_mode: ExportMode::Library,
	};
	let result = optimizer.optimize(&wasm_bytes).unwrap();

	assert_eq!(result.len(), wasm_bytes.len(), "None mode should return unchanged bytes");
}
