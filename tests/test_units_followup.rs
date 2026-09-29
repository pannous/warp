// A unit result is a value that prints as its canonical form: the smallest unit involved (wiki/unit.md)
use warp::*;

#[test]
fn test_unit_sum_serializes_in_the_smallest_unit() {
	let sum = wasm_emitter::eval("3km+10m");
	assert_eq!(sum.serialize(), "3010 m");
	assert_eq!(format!("{sum:?}"), "3010 m");
}

#[test]
fn test_single_quantity_keeps_its_unit() {
	let quantity = wasm_emitter::eval("2 km");
	assert_eq!(quantity.serialize(), "2 km");
	assert_eq!(format!("{quantity:?}"), "2 km");
}
