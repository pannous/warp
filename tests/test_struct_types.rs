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

// End goal API - raw GC struct roundtrip via Person type
use wasp::gc_struct;
use wasp::gc_traits::GcObject;

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

/// gc_struct! wrapper for reading WASM GC Person struct
gc_struct! {
	WasmPerson {
		name: 0 => String,
		age: 1 => i64,
	}
}

#[test]
fn test_class_instance_raw() {
	use wasp::{WasmGcEmitter, TypeDef, extract_instance_values};
	use wasmtime::*;

	// The expected result as a Rust struct
	let alice = RustPerson::new("Alice", 30);

	// Parse full code: class definition + instance
	let code = "class Person{name:String age:i64}; Person{name:'Alice' age:30}";
	let parsed = wasp::parse(code);

	// Find class definition and instance in the parsed tree
	// For "class X; instance" the result is a List containing both
	let (class_node, instance_node) = match parsed.drop_meta() {
		wasp::Node::List(items, _, _) if items.len() >= 2 => {
			(items[0].clone(), items[1].clone())
		}
		_ => panic!("expected list with class and instance"),
	};

	// Extract TypeDef from class definition
	let person_typedef = TypeDef::from_node(&class_node).expect("valid class definition");
	assert_eq!(person_typedef.name, "Person");
	assert_eq!(person_typedef.fields.len(), 2);

	// Extract field values from instance automatically
	let (type_name, field_values) = extract_instance_values(&instance_node).expect("valid instance");
	assert_eq!(type_name, "Person");
	assert_eq!(field_values.len(), 2);

	// Emit raw GC struct WASM
	let wasm_bytes = WasmGcEmitter::emit_raw_struct(&person_typedef, &field_values);

	// Run WASM and get result
	let mut config = Config::new();
	config.wasm_gc(true);
	config.wasm_function_references(true);
	let engine = Engine::new(&config).unwrap();
	let mut store = Store::new(&engine, ());
	let module = Module::new(&engine, &wasm_bytes).unwrap();
	let linker = Linker::new(&engine);
	let instance = linker.instantiate(&mut store, &module).unwrap();
	let main = instance.get_func(&mut store, "main").unwrap();
	let mut results = vec![Val::I32(0)];
	main.call(&mut store, &[], &mut results).unwrap();

	// Wrap GcObject in Node::Data - this is how eval() would return it
	use wasp::gc_traits::GcObject as ErgonomicGcObject;
	let gc_obj = ErgonomicGcObject::new(results[0].clone(), store, Some(instance)).unwrap();
	let node_data = wasp::data(gc_obj);

	// Extract from Node::Data and read values
	if let wasp::Node::Data(dada) = &node_data {
		let extracted = dada.downcast_ref::<ErgonomicGcObject>().expect("should be GcObject");
		let name: String = extracted.get_string(0).unwrap();
		let age: i64 = extracted.get(1).unwrap();

		let result = RustPerson { name, age };
		assert_eq!(result, alice);
		println!("Node::Data(GcObject) roundtrip: {:?} == {:?}", result, alice);
	} else {
		panic!("expected Node::Data");
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
