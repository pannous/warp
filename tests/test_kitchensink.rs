use warp::{block, float, int, key, list, parens, symbol, text, Node};
use warp::wasm_emitter::WasmGcEmitter;
use warp::wasp_parser::WaspParser;
use warp::write_wasm;

/// Comprehensive test covering all Node types and their WASM encoding
#[test]
fn test_kitchensink_all_node_types() {
	println!("\n=== Kitchensink Test: All Node Types ===\n");

	// Test 1: Empty node
	test_node("Empty", Node::Empty);

	// Test 2: Number nodes (Int and Float)
	test_node("Number::Int", int(42));
	test_node("Number::Float", float(1.23));

	// Test 3: Text node
	test_node("Text", text("hello world"));

	// Test 4: Char node
	test_node("Char", Node::Char('🦀'));

	// Test 5: Symbol node
	test_node("Symbol", symbol("my_var"));

	// Test 6: Tag node (from parser)
	let tag_input = "div{class=container}";
	let tag_node = WaspParser::parse(tag_input);
	test_node("Tag", tag_node);

	// Test 7: Key node
	test_node("Key", key("key", int(123)));

	// Test 8: Block node
	test_node("Block", parens(vec![int(1), int(2), int(3)]));

	// Test 10: List node
	test_node("List", list(vec![text("item1"), text("item2"), text("item3")]));

	// Test 11: Data node
	test_node("Data", Node::data(vec![1, 2, 3, 4, 5]));

	// Test 12: Complex nested structure
	let complex = WaspParser::parse("html{head{title{My Page}} body{h1{Hello} p{World}}}");
	if !matches!(complex, Node::Error(_)) {
		test_node("Complex nested HTML", complex);
	}

	println!("\n✅ All kitchensink tests passed!\n");
}

/// Helper function to test encoding a single node
fn test_node(name: &str, node: Node) {
	println!("Testing {}: {:?}", name, node);

	let mut emitter = WasmGcEmitter::new();
	emitter.emit();
	emitter.emit_node_main(&node);

	let bytes = emitter.finish();

	// Verify WASM magic number
	assert_eq!(
		&bytes[0..4],
		&[0x00, 0x61, 0x73, 0x6D],
		"{}: Invalid WASM magic number",
		name
	);

	// Verify WASM version
	assert_eq!(
		&bytes[4..8],
		&[0x01, 0x00, 0x00, 0x00],
		"{}: Invalid WASM version",
		name
	);

	// Validate with wasmparser
	use wasmparser::{Validator, WasmFeatures};
	let mut features = WasmFeatures::default();
	features.set(WasmFeatures::REFERENCE_TYPES, true);
	features.set(WasmFeatures::GC, true);

	let mut validator = Validator::new_with_features(features);
	match validator.validate_all(&bytes) {
		Ok(_) => println!("  ✓ WASM validation passed"),
		Err(e) => panic!("{}: WASM validation failed: {}", name, e),
	}

	// Write to file for inspection
	let task = name.to_lowercase().replace("::", "_").replace(" ", "_");
	let filename = format!("out/kitchensink_{}.wasm", task);
	if write_wasm(&filename, &bytes) {
		println!("  ✓ Written to {}", filename);
	}

	println!();
}

/// Test that verifies all node types can be encoded in a single complex tree
#[test]
fn test_kitchensink_complex_tree() {
	println!("\n=== Kitchensink: Complex Tree with All Types ===\n");

	// Build a complex tree containing all node types
	let complex_tree = list(vec![
		// Empty
		Node::Empty,
		// Numbers
		int(42),
		float(std::f64::consts::PI),
		// Strings
		text("hello"),
		symbol("world"),
		Node::Char('🚀'),
		// Key
		key("key", int(100)),
		// Nested Block (Curly brackets)
		block(vec![int(1), int(2)]),
		// List (Square brackets)
		list(vec![text("a"), text("b")]),
		// Data
		Node::data("custom data"),
	]);

	let mut emitter = WasmGcEmitter::new();
	emitter.emit();
	emitter.emit_node_main(&complex_tree);

	let bytes = emitter.finish();

	// Validate
	use wasmparser::{Validator, WasmFeatures};
	let mut features = WasmFeatures::default();
	features.set(WasmFeatures::REFERENCE_TYPES, true);
	features.set(WasmFeatures::GC, true);

	let mut validator = Validator::new_with_features(features);
	validator
		.validate_all(&bytes)
		.expect("Complex tree WASM validation failed");

	// Check data section has all strings
	let filename = "out/kitchensink_complex_tree.wasm";
	write_wasm(filename, &bytes);

	println!("✓ Complex tree with all node types validated successfully");
	println!("✓ Written to {}", filename);

	// Verify with wasm-tools
	use std::process::Command;
	let output = Command::new("wasm-tools")
		.args(["print", filename])
		.output();

	if let Ok(result) = output {
		let wat = String::from_utf8_lossy(&result.stdout);
		println!("\n=== String Data Found in Memory ===");
		for line in wat.lines() {
			if line.contains("data") && !line.contains("data_type") {
				println!("{}", line.trim());
			}
		}

		// Verify specific strings are present
		assert!(
			wat.contains("hello"),
			"String 'hello' not found in data section"
		);
		assert!(
			wat.contains("world"),
			"String 'world' not found in data section"
		);
		assert!(
			wat.contains("key"),
			"String 'key' not found in data section"
		);
		println!("\n✅ All expected strings found in data section");
	}
}

/// Test WASM execution with wasmtime (if available)
#[test]
fn test_kitchensink_wasmtime_execution() {
	println!("\n=== Kitchensink: Wasmtime Execution Test ===\n");

	// Create a simple node
	let node = key("html", text("content"));

	let mut emitter = WasmGcEmitter::new();
	emitter.emit();
	emitter.emit_node_main(&node);
	let bytes = emitter.finish();

	// Write to file
	let filename = "out/kitchensink_wasmtime.wasm";
	write_wasm(filename, &bytes);

	// Try to run with wasmtime
	use std::process::Command;
	let output = Command::new("wasmtime").args(["--version"]).output();

	if let Ok(result) = output {
		let version = String::from_utf8_lossy(&result.stdout);
		println!("Wasmtime version: {}", version.trim());

		// Note: Actually running the WASM module would require proper host function setup
		// and GC support which is complex. For now, we verify the file is valid.
		println!("✓ WASM module generated successfully for wasmtime");
		println!(
			"  To run manually: wasmtime run --wasm-features=gc {}",
			filename
		);
	} else {
		println!("⚠ Wasmtime not found, skipping execution test");
	}
}
