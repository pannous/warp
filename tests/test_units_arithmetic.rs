// Unit words are values: `3km` is 3*km, sums convert to the finer unit (wiki/unit.md)
use warp::*;

mod common;

#[test]
fn test_unit_sum_uses_the_finer_unit() {
	let sum = wasm_emitter::eval("3km+10m");
	assert_eq!(sum, 3010);
	assert_eq!(sum.to_string(), "3010 m");
}

#[test]
fn test_spaced_known_unit_multiplies() {
	let sum = wasm_emitter::eval("1 m + 1km");
	assert_eq!(sum, 1001);
	assert_eq!(sum.to_string(), "1001 m");
}

#[test]
fn test_scaling_a_quantity() {
	assert_eq!(wasm_emitter::eval("2*3km").to_string(), "6 km");
	assert_eq!(wasm_emitter::eval("3km-500m").to_string(), "2500 m");
}

#[test]
fn test_incompatible_units_are_an_error() {
	common::fails_with("1m+1kg", "incompatible");
}

#[test]
fn test_spaced_unknown_unit_stays_a_list() {
	assert!(matches!(parse("2 foo").drop_meta(), Node::List(..)));
}

#[test]
fn test_variable_shadows_the_unit() {
	is!("m=5;3m", 15);
}
