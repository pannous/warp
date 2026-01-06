use wasp::extensions::numbers::Number;
use wasp::node::node;
use wasp::wasm_gc_emitter::WasmGcEmitter;
use wasp::wasm_gc_reader::read_bytes;
use wasp::Kind::Codepoint;
use wasp::Node::{Empty, Type};
use wasp::*;

fn TypeRef(name: &str) -> Node {
	Type {
		name: Box::new(symbol(name)),
		body: Box::new((Empty)),
	}
}

#[test]
fn test_class_definition() {
	is!(
		"class Person{name:String age:i64}",
		Type {
			name: Box::new(symbol("Person")),
			body: Box::new(list(vec![key("name", TypeRef("String")), key("age", TypeRef("i64")),]))
		}
	);
}

#[test]
fn test_class_instance() {
	is!(
		"class Person{name:String age:i64}; Person{name:'Alice' age:30}",
		key("Person",list(vec![key("name", text("Alice")),key("age", int(30)),]))
	);
}


#[test]
fn test_text() {
	is!("'test'", "test");
}

#[test]
fn test_symbol() {
	is!("test", symbol("test"));
}

#[test]
fn test_codepoint() {
	is!("'𖠋'", "𖠋");
	is!("'𖠋'", '𖠋');
}
