//! The wasm name registry is one cache for the whole process, while every run has its own wasmtime engine. Its keys must
//! never be compared across engines: wasmtime's StructType::eq asserts "same engine", and inside a should_panic test
//! that assertion became a panic during a panic, which aborted the whole test binary at random.
use warp::gc_traits::register_gc_types_from_wasm;
use warp::gc_traits::wasm_name_resolver::field_names;
use warp::util::gc_engine;
use wasmtime::{FieldType, Mutability, StorageType, StructType, ValType};

const ENGINES: usize = 300; // enough cache entries that keys from different engines share hash buckets

#[test]
fn struct_types_of_many_engines_share_the_name_cache() {
	let module = wat::parse_str(r#"(module (type $Point (struct (field $x i64) (field $y i64))))"#).unwrap();
	register_gc_types_from_wasm(&module).unwrap();
	let int_field = || FieldType::new(Mutability::Const, StorageType::ValType(ValType::I64));
	for _ in 0..ENGINES {
		let point = StructType::new(&gc_engine(), [int_field(), int_field()]).unwrap();
		let names = field_names(&point, None).unwrap();
		assert_eq!(names, vec![Some("x".to_string()), Some("y".to_string())]);
	}
}
