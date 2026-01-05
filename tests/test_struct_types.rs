use wasp::{float, int, symbol, text, Bracket, Node, Op, Separator};
use wasp::wasm_gc_emitter::WasmGcEmitter;
use wasp::{eq, write_wasm};
use wasp::Node::Empty;
use wasp::wasm_gc_reader::read_bytes;
use wasp::extensions::numbers::Number;

/// Test reading integer nodes via compact 3-field layout
#[test]
fn test_read_int_node() {
	println!("=== Testing Int Node Reading ===\n");

	let mut emitter = WasmGcEmitter::new();
	emitter.emit();
	emitter.emit_node_main(&int(42));
	let bytes = emitter.finish();

	let root = read_bytes(&bytes).expect("Failed to read WASM");
	println!("  Got node: {:?}", root);

	match root {
		Node::Number(Number::Int(val)) => {
			eq!(val, 42);
			println!("✓ Int value is 42");
		}
		other => panic!("Expected Number(Int(42)), got {:?}", other),
	}
}

/// Test reading float nodes
#[test]
fn test_read_float_node() {
	println!("=== Testing Float Node Reading ===\n");

	let mut emitter = WasmGcEmitter::new();
	emitter.emit();
	emitter.emit_node_main(&float(3.14));
	let bytes = emitter.finish();

	let root = read_bytes(&bytes).expect("Failed to read WASM");
	println!("  Got node: {:?}", root);

	match root {
		Node::Number(Number::Float(val)) => {
			assert!((val - 3.14).abs() < 0.001);
			println!("✓ Float value is 3.14");
		}
		other => panic!("Expected Number(Float(3.14)), got {:?}", other),
	}
}

/// Test reading text nodes with memory access
#[test]
fn test_read_text_node() {
	println!("=== Testing Text Node Reading ===\n");

	let mut emitter = WasmGcEmitter::new();
	emitter.emit();
	emitter.emit_node_main(&text("Hello, WASM!"));
	let bytes = emitter.finish();

	let root = read_bytes(&bytes).expect("Failed to read WASM");
	println!("  Got node: {:?}", root);

	match root {
		Node::Text(s) => {
			eq!(s, "Hello, WASM!".to_string());
			println!("✓ Text value is 'Hello, WASM!'");
		}
		other => panic!("Expected Text, got {:?}", other),
	}
}

/// Test reading symbol nodes
#[test]
fn test_read_symbol_node() {
	println!("=== Testing Symbol Node Reading ===\n");

	let mut emitter = WasmGcEmitter::new();
	emitter.emit();
	emitter.emit_node_main(&symbol("my_symbol"));
	let bytes = emitter.finish();

	let root = read_bytes(&bytes).expect("Failed to read WASM");
	println!("  Got node: {:?}", root);

	match root {
		Node::Symbol(s) => {
			eq!(s, "my_symbol".to_string());
			println!("✓ Symbol value is 'my_symbol'");
		}
		other => panic!("Expected Symbol, got {:?}", other),
	}
}

/// Test reading codepoint nodes
#[test]
fn test_read_codepoint_node() {
	println!("=== Testing Codepoint Node Reading ===\n");

	let mut emitter = WasmGcEmitter::new();
	emitter.emit();
	emitter.emit_node_main(&Node::Char('🚀'));
	let bytes = emitter.finish();

	let root = read_bytes(&bytes).expect("Failed to read WASM");
	println!("  Got node: {:?}", root);

	match root {
		Node::Char(c) => {
			eq!(c, '🚀');
			println!("✓ Codepoint is 🚀");
		}
		other => panic!("Expected Char, got {:?}", other),
	}
}

/// Test reading key nodes (compact: key in data, value in value field)
#[test]
fn test_read_key_node() {
	println!("=== Testing Key Node Reading ===\n");

	let key_node = Node::Key(
		Box::new(symbol("name")),
		Op::None,
		Box::new(text("value"))
	);

	let mut emitter = WasmGcEmitter::new();
	emitter.emit();
	emitter.emit_node_main(&key_node);
	let bytes = emitter.finish();

	let root = read_bytes(&bytes).expect("Failed to read WASM");
	println!("  Got node: {:?}", root);

	match root {
		Node::Key(key, _, value) => {
			match *key {
				Node::Symbol(s) => eq!(s, "name".to_string()),
				other => panic!("Expected Symbol key, got {:?}", other),
			}
			match *value {
				Node::Text(s) => eq!(s, "value".to_string()),
				other => panic!("Expected Text value, got {:?}", other),
			}
			println!("✓ Key node correctly decoded");
		}
		other => panic!("Expected Key, got {:?}", other),
	}
}

/// Test empty node
#[test]
fn test_read_empty_node() {
	println!("=== Testing Empty Node Reading ===\n");

	let mut emitter = WasmGcEmitter::new();
	emitter.emit();
	emitter.emit_node_main(&Empty);
	let bytes = emitter.finish();

	let root = read_bytes(&bytes).expect("Failed to read WASM");
	println!("  Got node: {:?}", root);
	eq!(root, Empty);
	println!("✓ Empty node correctly decoded");
}
