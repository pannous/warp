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

// End goal API - unified struct for both Rust and WASM GC
use wasp::wasm_struct;

// Single definition creates both Rust struct and WASM GC reader
wasm_struct! {
	Person {
		name: String,
		age: i64,
	}
}

#[test]
fn test_class_instance_raw() {
	use wasp::gc_traits::GcObject;

	let alice = Person::new("Alice", 30);

	// eval() automatically detects class+instance and returns Node::Data(GcObject)
	let result = wasp::wasm_gc_emitter::eval("class Person{name:String age:i64}; Person{name:'Alice' age:30}");

	// Extract GcObject and convert to Person struct
	if let wasp::Node::Data(dada) = &result {
		let gc_obj = dada.downcast_ref::<GcObject>().expect("should be GcObject");

		// Read fields directly (this approach works)
		let name: String = gc_obj.get_string(0).unwrap();
		let age: i64 = gc_obj.get(1).unwrap();
		let person = Person { name, age };

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
