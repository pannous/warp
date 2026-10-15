//! card sin-cos: the real functions sin, cos, tan, exp and ln are function values like floor: passed to map, named with
//! `f := sin`, referenced with `function sin`
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_real_function_passed_to_map_applies_to_each() {
	assert_eq!(printed("map([0 0], sin)"), "[0 0]");
	assert_eq!(printed("[0 0].map(cos)"), "[1 1]");
}

#[test]
fn a_real_function_defined_is_its_alias() {
	assert_eq!(printed("f := sin; f(0)"), "0");
	assert_eq!(printed("f := exp; f(0)"), "1");
	assert_eq!(printed("f := ln; f(1)"), "0");
	assert_eq!(printed("f := tan; f(0)"), "0");
}

#[test]
fn a_real_function_referenced_is_a_function() {
	assert_eq!(printed("f = function sin; f(0)"), "0");
}

#[test]
fn a_program_defining_sin_keeps_its_own() {
	assert_eq!(printed("sin(x) := x + 1; f := sin; f(1)"), "2");
}
