use warp::Node::{Empty, Type};
use warp::*;
use warp::wasm_gc_emitter::eval;

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
fn test_magic_object_roundtrip() {
	// wasm_object! creates wasm_struct! class definition AND instance in one go!!
	// 🎉 Most ergonomic way to create classes and instances, beautiful syntax!
	let alice = wasm_object! { Person { name: String = "Alice", age: i64 = 30 } };
	is!("class Person{name:String age:i64}; Person{name:'Alice' age:30}", alice);
}


#[test]
fn test_debug_format() {
	// Verify the Debug output format shows type name, field names and values
	// Expected: Data(GcObject{Person{name:'Bob' age:42}})
	let result = eval("class Person{name:String age:i64}; Person{name:'Bob' age:42}");
	let debug_str = format!("{:?}", result);
	println!("Debug output: {}", debug_str);
	assert!(debug_str.contains("Person{"), "Should contain type name 'Person{{");
	assert!(debug_str.contains("name:"), "Should contain field name 'name:'");
	assert!(debug_str.contains("age:"), "Should contain field name 'age:'");
	assert!(debug_str.contains("'Bob'"), "Should contain value 'Bob'");
	assert!(debug_str.contains("42"), "Should contain value 42");
}

#[test]
#[should_panic(expected = "'Bob'")] // Now shows actual values in assertion!
fn test_magic_object_mismatch() {
	let alice = wasm_object! { Person4 { name: String = "Alice", age: i64 = 30 } };
	is!("class Person4{name:String age:i64}; Person4{name:'Bob' age:42}", alice);
}

/*
let alice = wasm_object! { Person { name: String = "Alice", age: i64 = 30 } }; is PERFECTLY FINE

  Why full type inference like Person { name= "Alice", age= 30 } isn't possible:
  Rust's declarative macros (macro_rules!) are purely syntactic -
  they can't inspect the type of a literal like 30 at compile time. That requires:
  - Procedural macros (separate crate)
  - Or const generics with unstable features
 */

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
fn test_class_instance_explicit() -> anyhow::Result<()> {
	// if the beautiful test_magic_object_roundtrip fails, debug the result here
	// eval() now returns Node::Data(GcObject) for class instances
	// Verify GcObject fields match expected values
	use warp::gc_traits::GcObject;
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
	use warp::gc_traits::GcObject;
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
	use warp::gc_traits::GcObject;

	let alice = Person::new("Alice", 30);

	// eval() automatically detects class+instance and returns Node::Data(GcObject)
	let result = warp::wasm_gc_emitter::eval("class Person{name:String age:i64}; Person{name:'Alice' age:30}");

	// Extract GcObject and convert to Person struct
	if let warp::Node::Data(dada) = &result {
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
