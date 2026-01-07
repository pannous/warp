use wasp::Node::{Empty, Type};
use wasp::*;
use wasp::wasm_gc_emitter::eval;

// End goal API achieved - unified struct for both Rust and WASM GC
// Single definition creates both Rust struct and WASM GC reader
wasm_struct! {
	Person {
		name: String,
		age: i64,
	}
}

#[test]
fn test_class_instance_magic_roundtrip() {
	let alice = Person { name: "Alice".into(), age: 30 };
	is!("class Person{name:String age:i64}; Person{name:'Alice' age:30}", alice); // IT WORKS!! 🎉
}


#[test]
fn test_object_magic_roundtrip_2() {
	// wasm_object! creates wasm_struct! class definition AND instance in one go!!
	let alice = wasm_object! { Person2 { name: String = "Alice", age: i64 = 30 } };
	is!("class Person2{name:String age:i64}; Person2{name:'Alice' age:30}", alice);
}

fn field(name: &str, type_name: &str) -> Node {
	let typ=Type {
		name: Box::new(symbol(type_name)),
		body: Box::new(Empty),
	};
	key(name, typ)
}

#[test]
fn test_class_definition() {
	is!(
		"class Person{name:String age:i64}",
		Type {
			name: Box::new(symbol("Person")),
			body: Box::new(list(vec![field("name","String"), field("age", "i64")]))
		}
	);
}

#[test]
fn test_class_instance1() -> anyhow::Result<()> {
	// eval() now returns Node::Data(GcObject) for class instances
	// Verify GcObject fields match expected values
	use wasp::gc_traits::GcObject;
	let result = eval("class Person{name:String age:i64}; Person{name:'Alice' age:30}");
	if let Data(dada) = &result {
		let gc_obj = dada.downcast_ref::<GcObject>().expect("should be GcObject");
		// Field access by index (name access requires register_gc_types_from_wasm)
		let name: String = gc_obj.get_string(0)?;
		let age: i64 = gc_obj.get(1)?;
		eq!(name, "Alice");
		eq!(age, 30);
	}
	Ok(())
}




#[test]
fn test_class_instance2() -> anyhow::Result<()> {
	// eval() now returns Node::Data(GcObject) for class instances
	// Use from_gc() to create Person from GcObject
	use wasp::gc_traits::GcObject;
	let result = eval("class Person{name:String age:i64}; Person{name:'Alice' age:30}");
	if let Data(dada) = &result {
		let gc_obj = dada.downcast_ref::<GcObject>().expect("should be GcObject");
		let person = Person::from_gc(gc_obj)?;
		// Generated accessor methods - IDE autocomplete works!
		let name: String = person.name()?;
		let age: i64 = person.age()?;
		eq!(name, "Alice");
		eq!(age, 30);
	} else {
		panic!("expected Node::Data(GcObject), got {:?}", result);
	}
	Ok(())
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
