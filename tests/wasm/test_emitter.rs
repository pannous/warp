use crate::is;
use warp::*;

#[test]
fn test_compact_empty() {
	let mut emitter = WasmGcEmitter::new();
	emitter.emit();
	emitter.emit_node_main(&Node::Empty);
	let bytes = emitter.finish();
	assert!(!bytes.is_empty());
	parses_to!("ø", Empty);
	parses_to!("null", Empty);
	parses_to!("no", Empty);
	parses_to!("none", Empty);
	parses_to!("nix", Empty);
	parses_to!("empty", Empty);
	is!("ø", Empty);
	is!("null", Empty);
}

#[test]
fn test_compact_int() {
	let mut emitter = WasmGcEmitter::new();
	emitter.emit();
	emitter.emit_node_main(&Node::Number(Number::Int(42)));
	let bytes = emitter.finish();
	assert!(!bytes.is_empty());
}

#[test]
fn test_compact_float() {
	let mut emitter = WasmGcEmitter::new();
	emitter.emit();
	emitter.emit_node_main(&Node::Number(Number::Float(3.11)));
	let bytes = emitter.finish();
	assert!(!bytes.is_empty());
	is!("3.11", 3.11);
}
