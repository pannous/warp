use wasp::Node::{Empty, Type};
use wasp::*;

fn type_ref(name: &str) -> Node {
	Type {
		name: Box::new(symbol(name)),
		body: Box::new(Empty),
	}
}

#[test]
fn test_class_definition() {
	is!(
		"class Person{name:String age:i64}",
		Type {
			name: Box::new(symbol("Person")),
			body: Box::new(list(vec![key("name", type_ref("String")), key("age", type_ref("i64")),]))
		}
	);
}

#[test]
fn test_class_instance() {
	// eval() now returns Node::Data(GcObject) for class instances
	// Verify GcObject fields match expected values
	use wasp::gc_traits::GcObject;
	let result = wasp::wasm_gc_emitter::eval("class Person{name:String age:i64}; Person{name:'Alice' age:30}");
	if let wasp::Node::Data(dada) = &result {
		let gc_obj = dada.downcast_ref::<GcObject>().expect("should be GcObject");
		assert_eq!(gc_obj.get_string(0).unwrap(), "Alice");
		let age: i64 = gc_obj.get(1).unwrap();
		assert_eq!(age, 30);
	} else {
		panic!("expected Node::Data(GcObject), got {:?}", result);
	}
}

// End goal API - raw GC struct roundtrip via Person type
use wasp::gc_struct;

/// Rust Person struct for direct comparison
#[derive(Debug, Clone, PartialEq)]
struct RustPerson {
	name: String,
	age: i64,
}

impl RustPerson {
	fn new(name: &str, age: i64) -> Self {
		Self { name: name.to_string(), age }
	}
}

// gc_struct! wrapper for reading WASM GC Person struct
gc_struct! {
	WasmPerson {
		name: 0 => String,
		age: 1 => i64,
	}
}

#[test]
fn test_class_instance_raw() {
	use wasp::gc_traits::GcObject;

	let alice = RustPerson::new("Alice", 30);

	// eval() automatically detects class+instance and returns Node::Data(GcObject)
	let result = wasp::wasm_gc_emitter::eval("class Person{name:String age:i64}; Person{name:'Alice' age:30}");

	// Extract GcObject from Node::Data and read field values
	if let wasp::Node::Data(dada) = &result {
		let gc_obj = dada.downcast_ref::<GcObject>().expect("should be GcObject");
		let name: String = gc_obj.get_string(0).unwrap();
		let age: i64 = gc_obj.get(1).unwrap();

		let person = RustPerson { name, age };
		assert_eq!(person, alice);
		println!("eval() returns Node::Data(GcObject): {:?}", person);
	} else {
		panic!("expected Node::Data, got {:?}", result);
	}
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
