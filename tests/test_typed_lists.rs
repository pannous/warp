// A list provably of ints (or floats) is held as a wasm GC array: O(1) index and count, no boxing per element.
// Results must be exactly those of the cons-cell list.
use crate::common::fails_with;
use warp::wasm_emitter::{compile, eval};
use warp::*;
use wasmparser::{CompositeInnerType, Operator, Parser, Payload, StorageType, ValType};

/// How many operators in the module of `code` create, read or write an `(array i64)` or `(array f64)`
fn typed_array_operations(code: &str) -> usize {
	let bytes = compile(code).unwrap_or_else(|error| panic!("{code} does not compile: {error:?}")).bytes;
	let mut array_types = Vec::new();
	let mut type_index = 0u32;
	let mut operations = 0;
	for payload in Parser::new(0).parse_all(&bytes) {
		match payload.expect("valid module") {
			Payload::TypeSection(reader) => {
				for group in reader {
					for sub_type in group.expect("type group").into_types() {
						if let CompositeInnerType::Array(array) = &sub_type.composite_type.inner {
							if matches!(array.0.element_type, StorageType::Val(ValType::I64 | ValType::F64)) {
								array_types.push(type_index);
							}
						}
						type_index += 1;
					}
				}
			}
			Payload::CodeSectionEntry(body) => {
				let mut reader = body.get_operators_reader().expect("operators");
				while !reader.eof() {
					let typed = match reader.read().expect("operator") {
						Operator::ArrayGet { array_type_index } | Operator::ArraySet { array_type_index }
						| Operator::ArrayNewFixed { array_type_index, .. } => array_types.contains(&array_type_index),
						_ => false,
					};
					operations += typed as usize;
				}
			}
			_ => {}
		}
	}
	operations
}

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn test_int_list_literal_is_an_array() {
	assert!(typed_array_operations("xs=[3,1,4]; xs#2") > 0);
	assert!(typed_array_operations("xs=[3,1,4]; s=0; for x in xs {s+=x}; s") > 0);
	assert!(typed_array_operations("xs=[3,1,4]; sum xs") > 0);
}

#[test]
fn test_mixed_list_stays_cons_cells() {
	assert_eq!(typed_array_operations("xs=[3,\"a\",4]; xs#2"), 0);
	assert_eq!(typed_array_operations("xs=[3,1.5f,4]; xs#2"), 0);
	assert_eq!(typed_array_operations("xs=[ø]; if xs {1} else {2}"), 0);
	assert_eq!(typed_array_operations("xs=[1, ø]; #xs"), 0);
}

#[test]
fn test_int_array_index_count_sum_for() {
	is!("xs=[3,1,4]; xs#2", 1);
	is!("xs=[3,1,4]; xs[0]", 3);
	is!("xs=[3,1,4]; #xs", 3);
	is!("xs=[3,1,4]; count xs", 3);
	is!("xs=[3,1,4]; xs.count", 3);
	is!("xs=[3,1,4]; sum xs", 8);
	is!("xs=[3,1,4]; s=0; for x in xs {s+=x}; s", 8);
	is!("xs=[3,1,4]; s=0; for x in xs {s=s*10+x}; s", 314);
	is!("xs=[3,1,4]; i=xs#3; i*2", 8);
}

#[test]
fn test_int_array_assignment_keeps_value_semantics() {
	is!("xs=[1,2,3]; xs#2=7; xs#2", 7);
	is!("xs=[1,2,3]; xs[1]=7; sum xs", 11);
	is!("xs=[1,2,3]; ys=xs; ys#1=9; xs#1", 1);
	is!("xs=[1,2,3]; ys=xs; ys#1=9; ys#1", 9);
	is!("xs=[1,2,3]; s=0; for x in xs {xs#1=10; s+=x}; s", 6);
	is!("xs=[1,2,3]; xs=[4,5]; #xs", 2);
}

#[test]
fn test_int_array_reads_back_as_the_same_node() {
	assert_eq!(printed("xs=[3,1,4]; xs"), printed("[3,1,4]"));
	assert_eq!(printed("xs=[3,1,4]; xs#2=7; xs"), "[3 7 4]");
	assert_eq!(printed("xs=[3,1,4]; ys=xs; ys"), printed("[3,1,4]"));
	assert_eq!(eval("xs=[3,1,4]; xs"), eval("[3,1,4]"));
}

#[test]
fn test_int_array_errors_like_cons_list() {
	fails_with("xs=[3,1,4]; xs#4", "index");
	fails_with("xs=[3,1,4]; xs#0", "index");
	fails_with("xs=[3,1,4]; xs#4=1", "index");
}

#[test]
fn test_zero_filled_int_list_is_an_array() {
	assert!(typed_array_operations("xs = int[5]; xs[2] = 7; xs[2]") > 0);
	is!("xs = int[5]; xs[2] = 7; sum xs", 7);
	is!("xs = int[5]; #xs", 5);
	assert_eq!(printed("xs = int[3]; xs[1] = 4; xs"), "[0 4 0]");
}

#[test]
fn test_int_expression_items_are_an_array() {
	assert!(typed_array_operations("a=2; xs=[a, a+1, a*3]; xs#3") > 0);
	is!("a=2; xs=[a, a+1, a*3]; xs#3", 6);
	assert_eq!(printed("a=2; xs=[a, a+1, 7/7]; xs"), "[2 3 1]");
	fails_with("xs=[3,1,4]; xs#-1", "index out of range");
}
