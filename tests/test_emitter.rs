use wasp::*;
use wasp::wasm_optimizer::{ExportMode, OptimizationMode, WasmOptimizer};


#[test]
fn test_compact_empty() {
	let mut emitter = WasmGcEmitter::new();
	emitter.emit();
	emitter.emit_node_main(&Node::Empty);
	let bytes = emitter.finish();
	assert!(!bytes.is_empty());
	peq!("ø",Empty);
	peq!("null",Empty);
	peq!("no",Empty);
	peq!("none",Empty);
	peq!("nix",Empty);
	peq!("empty",Empty);
	is!("ø",Empty);
	is!("null",Empty);
}

#[test]
fn test_compact_int() {
	let mut emitter = WasmGcEmitter::new();
	emitter.emit();
	emitter.emit_node_main(&Node::Number(Number::Int(42)));
	let bytes = emitter.finish();
	assert!(!bytes.is_empty());
	is!("42",42);
}

#[test]
fn test_compact_float() {
	let mut emitter = WasmGcEmitter::new();
	emitter.emit();
	emitter.emit_node_main(&Node::Number(Number::Float(3.11)));
	let bytes = emitter.finish();
	assert!(!bytes.is_empty());
	is!("3.11",3.11);
}


#[test]
fn test_tools_available() {
	assert!(WasmOptimizer::tools_available(), "wasm-opt not found");
	assert!(WasmOptimizer::tree_shaking_available(), "wasm-metadce not found");
}

#[test]
fn test_optimizer_graph_generation() {
	let optimizer = WasmOptimizer::executable(
		OptimizationMode::Standard,
		vec!["main".to_string(), "init".to_string()],
	);

	if let ExportMode::Executable { ref entry_points } = optimizer.export_mode {
		let graph = optimizer.build_roots_graph(entry_points);
		assert!(graph.contains("\"export\": \"main\""));
		assert!(graph.contains("\"export\": \"init\""));
		assert!(graph.contains("\"root\": true"));
	}
}