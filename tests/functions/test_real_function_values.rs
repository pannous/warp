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

#[test]
fn a_real_function_applies_to_the_value_after_it() {
	assert_eq!(printed("sin cos 0"), "0.8414709848078965");
	assert_eq!(printed("f(x) := exp x; f(1)"), "2.718281828459045");
}

#[test]
fn every_library_word_of_one_value_applies_braceless_and_as_suffix() {
	assert_eq!(printed("f(x) := trunc x; f(2.5)"), "2");
	assert_eq!(printed("f(x) := log10 x; f(100)"), "2");
	assert_eq!(printed("map([2.5], trunc)"), "[2]");
	assert_eq!(printed("x = 1; x sined"), "0.8414709848078965");
	assert_eq!(printed("x = 2.5; x floored"), "2");
}

#[test]
fn every_library_word_of_one_value_counts_the_value_after_it() {
	assert_eq!(printed("floor#2"), printed("floor(#2)"));
	assert_eq!(printed("sin#2"), printed("sin(#2)"));
}
