use warp::Node::*;
use warp::{Bracket, Node, Op};
use warp::run::wasmtime_runner::run;
use warp::wasm_gc_emitter::{eval, WasmGcEmitter};
use warp::StringExtensions;
use warp::{eq, is, write_wasm};

fn normalize_blocks(node: &Node) -> Node {
	let node = node.drop_meta();
	match node {
		List(items, Bracket::Curly, _) if items.len() == 1 => normalize_blocks(&items[0]),
		List(items, _, _) if items.len() == 1 => normalize_blocks(&items[0]),
		// Preserve Op since WASM roundtrip now preserves it
		Key(k, op, v) => Key(Box::new(normalize_blocks(k)), op.clone(), Box::new(normalize_blocks(v))),
		_ => node.clone(),
	}
}
use warp::type_kinds::NodeKind;
use warp::Number::Int;

#[test]
fn test_wasm_roundtrip() {
	// same as eval() but shows explicit parsing
	use warp::wasp_parser::WaspParser;

	// Parse WASP input
	let input = "html{test=1}";
	let node = WaspParser::parse(input);
	println!("Parsed node: {:?}", node);

	let mut emitter = WasmGcEmitter::new();
	emitter.emit();
	emitter.emit_node_main(&node); // Emit a main() function that returns the node

	let path = "out/test_wasm_roundtrip.wasm";
	let bytes = emitter.finish();
	assert!(write_wasm(path, &bytes), "Failed to write WASM file");
	println!("✓ Generated {} ({} bytes)", path, bytes.len());

	// let root : GcObject = run_wasm_gc_object(path).expect("Failed to read back WASM");
	let root = run(path); // reconstruct Node from WASM via GcObject
	println!("✓ Read back root node from WASM: {:?}", root);
	// Normalize original: unwrap single-item blocks like WASM does
	let normalized = normalize_blocks(&node);
	eq!(root, normalized);
}

#[test]
fn test_wasm_roundtrip_via_is() {
	// Parser treats html{test=1} as Key("html", {test=1})
	// WASM roundtrip now preserves Op: outer uses Colon, inner uses Assign
	let x = Key(Box::new(Symbol("test".s())), Op::Assign, Box::new(Number(Int(1))));
	let _ok: Node = eval("html{test=1}");
	// After single-item block unwrapping, body becomes just the Key
	is!(
		"html{test=1}",
		Key(Box::new(Symbol("html".s())), Op::Colon, Box::new(x))
	);
}

#[test]
fn test_emit_gc_types() {
	// let mut emitter = WasmGcEmitter::new();
	// emitter.emit();
	// Verify unified type indices are valid (can be 0 for first type)
	// private
	// eq!(emitter.node_base_type, 0); // First type
	// eq!(emitter.node_array_type, 1); // Second type
	// assert!(emitter.next_type_idx > 1); // We defined at least 2 types
}

#[test]
fn test_generate_wasm() {
	let mut emitter = WasmGcEmitter::new();
	emitter.emit();
	let bytes = emitter.finish();

	// Should have WASM magic number
	eq!(&bytes[0..4], &[0x00, 0x61, 0x73, 0x6d]);
	// Should have version 1
	eq!(&bytes[4..8], &[0x01, 0x00, 0x00, 0x00]);
}

#[test]
fn test_node_kind_enum_abi() {
	// ensure enum values match expected ABI (Kind)
	eq!(NodeKind::Empty as u32, 0);
	eq!(NodeKind::Int as u32, 1);
	eq!(NodeKind::Float as u32, 2);
	eq!(NodeKind::Text as u32, 3);
	eq!(NodeKind::Codepoint as u32, 4);
	eq!(NodeKind::Symbol as u32, 5);
	eq!(NodeKind::Key as u32, 6);
	eq!(NodeKind::Block as u32, 7);
	eq!(NodeKind::List as u32, 8);
	eq!(NodeKind::Data as u32, 9);
	eq!(NodeKind::Meta as u32, 10);
	eq!(NodeKind::Error as u32, 11);
}

#[test]
fn test_function_usage_tracking() {
	use warp::wasp_parser::WaspParser;

	// Parse a simple char - should only use new_codepoint
	let node = WaspParser::parse("'🦀'");
	let mut emitter = WasmGcEmitter::new();
	emitter.emit();
	emitter.emit_node_main(&node);

	let used = emitter.get_used_functions();
	let unused = emitter.get_unused_functions();

	println!("Used functions: {:?}", used);
	println!("Unused functions: {:?}", unused);

	// new_codepoint should be used
	assert!(used.contains(&"new_codepoint"), "new_codepoint should be used for char");

	// Many functions should be unused for a simple char
	assert!(unused.iter().any(|s| s == "new_text"), "new_text should be unused for char");
	assert!(unused.iter().any(|s| s == "new_symbol"), "new_symbol should be unused for char");
	// new_pair was removed (Pair variant removed from Node)

	// Verify unused count is significant (for tree-shaking validation)
	assert!(unused.len() >= 5, "Should have at least 5 unused functions, got {}", unused.len());
}

#[test]
fn test_emit_for_node_tree_shaking() {
	use warp::wasp_parser::WaspParser;

	// Without tree-shaking
	let node = WaspParser::parse("'🦀'");
	let mut emitter_full = WasmGcEmitter::new();
	emitter_full.emit();
	emitter_full.emit_node_main(&node);
	let bytes_full = emitter_full.finish();

	// With tree-shaking
	let mut emitter_slim = WasmGcEmitter::new();
	emitter_slim.emit_for_node(&node);
	let bytes_slim = emitter_slim.finish();

	println!(
		"Tree-shaking at emit time: {} -> {} bytes ({}% reduction)",
		bytes_full.len(),
		bytes_slim.len(),
		(bytes_full.len() - bytes_slim.len()) * 100 / bytes_full.len()
	);

	// Tree-shaking should reduce size (we skip unused constructor functions)
	// Note: external wasm-opt/wasm-metadce achieves ~50% more reduction by also
	// removing getters, names section entries, etc.
	assert!(
		bytes_slim.len() < bytes_full.len() * 85 / 100,
		"Tree-shaking should reduce size by at least 15%: {} vs {}",
		bytes_slim.len(),
		bytes_full.len()
	);
}
