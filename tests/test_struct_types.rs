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
	use wasp::{WasmGcEmitter, RawFieldValue, TypeDef, FieldDef};
	use wasmtime::*;

	// The expected result as a Rust struct
	let alice = RustPerson::new("Alice", 30);

	// Create TypeDef for Person (matches class Person{name:String age:i64})
	let person_typedef = TypeDef {
		name: "Person".to_string(),
		tag: 100,
		fields: vec![
			FieldDef { name: "name".to_string(), type_name: "String".to_string() },
			FieldDef { name: "age".to_string(), type_name: "i64".to_string() },
		],
		wasm_type_idx: None,
	};

	// Emit raw GC struct WASM with Alice's values
	let wasm_bytes = WasmGcEmitter::emit_raw_struct(
		&person_typedef,
		&[RawFieldValue::from("Alice"), RawFieldValue::from(30i64)],
	);

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

	// Read back via gc_struct! wrapper
	let wasm_person = WasmPerson::from_val(results[0].clone(), store, Some(instance)).unwrap();
	let name: String = wasm_person.name().unwrap();
	let age: i64 = wasm_person.age().unwrap();

	// Convert to Rust struct for comparison
	let result = RustPerson { name, age };

	assert_eq!(result, alice);
	println!("Raw GC struct roundtrip works: {:?} == {:?}", result, alice);
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
