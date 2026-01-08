/// Experimental: Return raw GC structs instead of Node encoding
/// This allows direct struct field access without Node wrapper overhead
///
/// Two approaches demonstrated:
/// 1. WasmGcEmitter::emit_raw_struct - uses emitter for WASM generation
/// 2. gc_struct! macro - ergonomic typed wrappers with index-based access

use wasmtime::*;
use anyhow::Result;

// Import emitter and type system
use warp::{WasmGcEmitter, RawFieldValue, TypeDef, FieldDef};

// Import gc_struct! macro and gc_traits module for ergonomic access
use warp::gc_struct;
use warp::gc_traits::GcObject as ErgonomicGcObject;

/// Person struct mirrors the WASM GC $Person type
#[derive(Debug, Clone, PartialEq)]
pub struct Person {
	pub name: String,
	pub age: i64,
}

impl Person {
	pub fn new(name: &str, age: i64) -> Self {
		Self { name: name.to_string(), age }
	}
}

/// Read a Person from a GcObject (WASM GC struct)
fn person_from_gc(val: &Val, store: &mut Store<()>, instance: &Instance) -> Result<Person> {
	let anyref = val.unwrap_anyref().ok_or_else(|| anyhow::anyhow!("not anyref"))?;
	let structref = anyref.unwrap_struct(&*store)?;

	// Field 0: name (ref $String -> ptr, len)
	let name_val = structref.field(&mut *store, 0)?;
	let name = read_string_field(&name_val, store, instance)?;

	// Field 1: age (i64)
	let age_val = structref.field(&mut *store, 1)?;
	let age = age_val.unwrap_i64();

	Ok(Person { name, age })
}

/// Read string from $String struct (ptr, len) in linear memory
fn read_string_field(val: &Val, store: &mut Store<()>, instance: &Instance) -> Result<String> {
	let anyref = val.unwrap_anyref().ok_or_else(|| anyhow::anyhow!("not anyref"))?;
	let structref = anyref.unwrap_struct(&*store)?;

	let ptr = structref.field(&mut *store, 0)?.unwrap_i32();
	let len = structref.field(&mut *store, 1)?.unwrap_i32();

	if len == 0 {
		return Ok(String::new());
	}

	let memory = instance.get_memory(&mut *store, "memory")
		.ok_or_else(|| anyhow::anyhow!("no memory"))?;
	let mut buf = vec![0u8; len as usize];
	memory.read(&*store, ptr as usize, &mut buf)?;
	Ok(String::from_utf8(buf)?)
}

/// Create TypeDef for Person struct
fn person_type_def() -> TypeDef {
	TypeDef {
		name: "Person".to_string(),
		tag: 100, // arbitrary tag for testing
		fields: vec![
			FieldDef { name: "name".to_string(), type_name: "String".to_string() },
			FieldDef { name: "age".to_string(), type_name: "i64".to_string() },
		],
		wasm_type_idx: None,
	}
}

/// Create TypeDef for Point struct
fn point_type_def() -> TypeDef {
	TypeDef {
		name: "Point".to_string(),
		tag: 101, // arbitrary tag for testing
		fields: vec![
			FieldDef { name: "x".to_string(), type_name: "i64".to_string() },
			FieldDef { name: "y".to_string(), type_name: "i64".to_string() },
		],
		wasm_type_idx: None,
	}
}

/// Common runtime setup for WASM GC tests
fn run_wasm_gc(wasm_bytes: &[u8]) -> Result<(Store<()>, Instance, Val)> {
	let mut config = Config::new();
	config.wasm_gc(true);
	config.wasm_function_references(true);

	let engine = Engine::new(&config)?;
	let mut store = Store::new(&engine, ());
	let module = Module::new(&engine, wasm_bytes)?;

	let linker = Linker::new(&engine);
	let instance = linker.instantiate(&mut store, &module)?;

	let main = instance.get_func(&mut store, "main")
		.ok_or_else(|| anyhow::anyhow!("no main"))?;

	let mut results = vec![Val::I32(0)];
	main.call(&mut store, &[], &mut results)?;

	Ok((store, instance, results.remove(0)))
}

#[test]
fn test_raw_person_struct() -> Result<()> {
	let expected = Person::new("Alice", 30);

	// Emit WASM using emitter
	let wasm_bytes = WasmGcEmitter::emit_raw_struct(
		&person_type_def(),
		&[RawFieldValue::from("Alice"), RawFieldValue::from(30i64)],
	);

	let (mut store, instance, result) = run_wasm_gc(&wasm_bytes)?;

	// Convert GC struct to Person
	let person = person_from_gc(&result, &mut store, &instance)?;

	assert_eq!(person, expected);
	println!("Raw Person struct roundtrip works: {:?}", person);

	Ok(())
}

#[test]
fn test_class_instance_raw() -> Result<()> {
	let alice = Person::new("Alice", 30);

	// Emit via emitter
	let wasm_bytes = WasmGcEmitter::emit_raw_struct(
		&person_type_def(),
		&[RawFieldValue::from("Alice"), RawFieldValue::from(30i64)],
	);

	let (mut store, instance, result) = run_wasm_gc(&wasm_bytes)?;
	let person = person_from_gc(&result, &mut store, &instance)?;

	assert_eq!(person, alice);

	Ok(())
}

// ============================================================
// gc_struct! macro-based approach with ergonomic field access
// ============================================================

// Define a typed Point wrapper using gc_struct! macro
gc_struct! {
    Point {
        x: 0 => i64,
        y: 1 => i64,
    }
}

#[test]
fn test_gc_struct_macro() -> Result<()> {
	// Emit WASM using emitter
	let wasm_bytes = WasmGcEmitter::emit_raw_struct(
		&point_type_def(),
		&[RawFieldValue::from(10i64), RawFieldValue::from(20i64)],
	);

	let (store, instance, result) = run_wasm_gc(&wasm_bytes)?;

	// Create typed Point wrapper using gc_struct! generated type
	let point = Point::from_val(result, store, Some(instance))?;

	// Use generated accessors
	let x = point.x()?;
	let y = point.y()?;

	assert_eq!(x, 10);
	assert_eq!(y, 20);

	println!("gc_struct! macro works: Point({}, {})", x, y);

	Ok(())
}

#[test]
fn test_gc_object_index_access() -> Result<()> {
	// Emit WASM using emitter
	let wasm_bytes = WasmGcEmitter::emit_raw_struct(
		&point_type_def(),
		&[RawFieldValue::from(42i64), RawFieldValue::from(99i64)],
	);

	let (store, instance, result) = run_wasm_gc(&wasm_bytes)?;

	// Use ErgonomicGcObject directly with index access
	let gc_obj = ErgonomicGcObject::new(result, store, Some(instance))?;

	// Get fields by index
	let x: i64 = gc_obj.get(0)?;
	let y: i64 = gc_obj.get(1)?;

	assert_eq!(x, 42);
	assert_eq!(y, 99);

	println!("GcObject index access works: ({}, {})", x, y);

	Ok(())
}
