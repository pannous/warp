// A unit result is a value that prints as its canonical form: the smallest unit involved (wiki/unit.md)
use warp::*;

#[test]
fn test_unit_sum_serializes_in_the_smallest_unit() {
	let sum = wasm_emitter::eval("3km+10m");
	assert_eq!(sum.serialize(), "3010m");
	assert_eq!(format!("{sum:?}"), "3010m");
}

#[test]
fn test_single_quantity_keeps_its_unit() {
	let quantity = wasm_emitter::eval("2 km");
	assert_eq!(quantity.serialize(), "2km");
	assert_eq!(format!("{quantity:?}"), "2km");
}

#[test]
fn test_value_with_tolerance_prints_back() {
	assert_eq!(wasm_emitter::eval("1950 ± 50").serialize(), "1950 ± 50");
}

#[test]
fn test_plus_minus_ascii_spelling() {
	assert_eq!(wasm_emitter::eval("1950 +- 50").serialize(), "1950 ± 50");
}

#[test]
fn test_tolerance_in_a_unit() {
	assert_eq!(wasm_emitter::eval("1950 cm ± 50").serialize(), "1950 ± 50cm");
	assert_eq!(wasm_emitter::eval("2m ± 5cm").serialize(), "200 ± 5cm");
}

#[test]
fn test_negative_tolerance_is_an_error() {
	match wasm_emitter::eval("1950 ± -5") {
		Node::Error(message) => assert!(format!("{message}").contains("negative")),
		other => panic!("expected an error, got {other:?}"),
	}
}

#[test]
fn test_range_with_a_unit() {
	assert_eq!(wasm_emitter::eval("1900 - 2000 AD").serialize(), "1900 - 2000AD");
	assert_eq!(wasm_emitter::eval("1900 - 2000 cm").serialize(), "1900 - 2000cm");
}

#[test]
fn test_descending_range_is_an_error() {
	match wasm_emitter::eval("2000 - 1900 AD") {
		Node::Error(message) => assert!(format!("{message}").contains("descending")),
		other => panic!("expected an error, got {other:?}"),
	}
}
