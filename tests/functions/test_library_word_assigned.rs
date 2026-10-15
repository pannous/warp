//! card floor-says: a library word assigned to a variable, or passed to map, is a function value
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_library_word_assigned_is_a_function() {
	assert_eq!(printed("f = function floor; f(2.5)"), "2");
	assert_eq!(printed("f = function upper; f(\"a\")"), "\"A\"");
	// decision reflection: a bare name is never the function value implicitly (P82)
	crate::common::fails_with("f = floor; f(2.5)", "floor needs 1 argument");
}

#[test]
fn a_libm_word_maps_over_a_list() {
	assert_eq!(printed("map([1.5 2.5], floor)"), "[1 2]");
	assert_eq!(printed("[1.5 2.5].map(floor)"), "[1 2]");
}

#[test]
fn a_variable_named_like_a_library_word_stays_a_value() {
	assert_eq!(printed("floor = 3; f = floor; f"), "3");
}
