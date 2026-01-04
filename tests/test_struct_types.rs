use wasp::extensions::numbers::Number;
use wasp::Node;
use wasp::Node::Symbol;
use wasp::Op;
use wasp::{Bracket, Separator};
use wasp::wasm_gc_emitter::WasmGcEmitter;
use wasp::{eq, write_wasm};
use wasp::Node::Empty;
use wasp::wasm_gc_reader::read_bytes;

/// Test ergonomic reading patterns from rasm
#[test]
fn test_ergonomic_node_reading() {
	println!("=== Testing Ergonomic WASM GC Reading ===\n");

	// Generate a WASM module with a simple node
	let mut emitter = WasmGcEmitter::new();
	emitter.emit();

	let node = Node::Number(Number::Int(42));
	emitter.emit_node_main(&node);

	let bytes = emitter.finish();

	// Read it back ergonomically using fast shared engine
	let root = read_bytes(&bytes).expect("Failed to read WASM");

	println!("✓ Loaded WASM module and got root node");

	// Test field access
	let kind: i32 = root.get("tag").expect("Failed to get tag");
	println!("  Root kind (tag): {}", kind);
	eq!(kind, 1); // NodeKind::Number

	let int_value: i64 = root.get("int_value").expect("Failed to get int_value");
	println!("  Int value: {}", int_value);
	eq!(int_value, 42);

	// Test convenience method
	let kind2 = root.kind().expect("Failed to get kind");
	println!("  Using .kind(): {}", kind2);
	eq!(kind2, 1);

	println!("\n✓ Ergonomic field access works!");
}

/// Test reading text nodes with memory access
#[test]
fn test_read_text_node() {
	println!("=== Testing Text Node Reading ===\n");

	let mut emitter = WasmGcEmitter::new();
	emitter.emit();

	let node = Node::Text("hello".to_string());
	emitter.emit_node_main(&node);

	let bytes = emitter.finish();
	let root = read_bytes(&bytes).expect("Failed to read WASM");

	println!("✓ Loaded text node");

	let kind = root.kind().expect("Failed to get kind");
	eq!(kind, 2); // NodeKind::Text

	// Read text from linear memory
	let text = root.text().expect("Failed to read text");
	println!("  Text: '{}'", text);
	eq!(text, "hello");

	println!("\n✓ Text reading from linear memory works!");
}

/// Test reading symbol nodes
#[test]
fn test_read_symbol_node() {
	println!("=== Testing Symbol Node Reading ===\n");

	let mut emitter = WasmGcEmitter::new();
	emitter.emit();

	let node = Node::Symbol("my_var".to_string());
	emitter.emit_node_main(&node);

	let bytes = emitter.finish();
	let root = read_bytes(&bytes).expect("Failed to read WASM");

	let kind = root.kind().expect("Failed to get kind");
	eq!(kind, 4); // NodeKind::Symbol

	let text = root.text().expect("Failed to read symbol");
	println!("  Symbol: '{}'", text);
	eq!(text, "my_var");

	println!("\n✓ Symbol reading works!");
}

/// Test the complete ergonomic pattern similar to: root = read("test.wasm"); is!(root.name, "html")
#[test]
fn test_ergonomic_pattern() {
	println!("=== Testing Complete Ergonomic Pattern ===\n");

	// Generate WASM with a Key node that has a name
	let mut emitter = WasmGcEmitter::new();
	emitter.emit();

	let node = Node::Key(
		Box::new(Symbol("html".to_string())),
		Op::Colon,
		Box::new(Node::List(
			vec![
				Node::keys(".param", "test"),
				Node::keys("body", "ok"),
			],
			Bracket::Curly,
			Separator::None,
		)),
	);
	emitter.emit_node_main(&node);

	let bytes = emitter.finish();
	let filename = "out/test_ergonomic_pattern.wasm";
	write_wasm(filename, &bytes);
}

#[test]
#[ignore]
fn what_was_that(){
	let root = Empty; // eval(filename).expect("Failed to read WASM file");

	println!("✓ Read WASM file");

	// Access name field
	let name = root.name();
	println!("  Name: '{}'", name);

	// The pattern: is!(root.name, "html")
	eq!(name, "html");
	println!("\n✓ Pattern works: root.name() == \"html\"");

	// Verify kind
	let _kind = root.kind();
	// println!("  Kind: {}", kind);
	// eq!(kind, 7); // NodeKind::Tag
}

/// Test field existence checking
#[test]
fn test_field_existence() {
	println!("=== Testing Field Existence ===\n");

	let mut emitter = WasmGcEmitter::new();
	emitter.emit();

	let node = Node::Number(Number::Int(123));
	emitter.emit_node_main(&node);

	let bytes = emitter.finish();
	let root = read_bytes(&bytes).expect("Failed to read WASM");

	// Test has() method
	assert!(root.has("tag").unwrap());
	assert!(root.has("int_value").unwrap());
	assert!(root.has("kind").unwrap()); // alias
	assert!(!root.has("nonexistent").unwrap());

	println!("✓ Field existence checking works");
}

/// Test empty node
#[test]
fn test_empty_node() {
	println!("=== Testing Empty Node ===\n");

	let mut emitter = WasmGcEmitter::new();
	emitter.emit();

	emitter.emit_node_main(&Node::Empty);

	let bytes = emitter.finish();
	let root = read_bytes(&bytes).expect("Failed to read WASM");

	let kind = root.kind().expect("Failed to get kind");
	eq!(kind, 0); // NodeKind::Empty

	println!("✓ Empty node works");
}

/// Test codepoint node
#[test]
fn test_codepoint_node() {
	println!("=== Testing Char Node ===\n");

	let mut emitter = WasmGcEmitter::new();
	emitter.emit();

	let node = Node::Char('🦀');
	emitter.emit_node_main(&node);

	let bytes = emitter.finish();
	let root = read_bytes(&bytes).expect("Failed to read WASM");

	let kind = root.kind().expect("Failed to get kind");
	eq!(kind, 3); // NodeKind::Char

	let codepoint: i64 = root.get("int_value").expect("Failed to get codepoint");
	println!(
		"  Char value: {} ({})",
		codepoint,
		char::from_u32(codepoint as u32).unwrap_or('?')
	);
	eq!(codepoint, '🦀' as i64);

	println!("✓ Char node works");
}

/// Test float number node
#[test]
fn test_float_node() {
	println!("=== Testing Float Node ===\n");

	let mut emitter = WasmGcEmitter::new();
	emitter.emit();

	let node = Node::Number(Number::Float(1.23));
	emitter.emit_node_main(&node);

	let bytes = emitter.finish();
	let root = read_bytes(&bytes).expect("Failed to read WASM");

	let kind = root.kind().expect("Failed to get kind");
	eq!(kind, 1); // NodeKind::Number

	let float_value: f64 = root.get("float_value").expect("Failed to get float");
	println!("  Float value: {}", float_value);
	eq!(float_value, 1.23);

	println!("✓ Float node works");
}
