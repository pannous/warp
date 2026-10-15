//! card floor-says: a library word passed to map is a function value, `f = function floor` holds it and `f := floor`
//! names it (card floor-error). The bare `f = floor` gets P82's "needs 1 argument" from card builtin-functions
//! (decision reflection, relayed by warp-interviewer 2026-10-15)
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_library_word_referenced_is_a_function() {
	assert_eq!(printed("f = function floor; f(2.5)"), "2");
	assert_eq!(printed("f = function upper; f(\"a\")"), "\"A\"");
}

#[test]
fn a_bare_library_word_assigned_needs_its_arguments() {
	// decision reflection: a bare name is never the function value implicitly (P82)
	crate::common::fails_with("f = floor; f(2.5)", "floor needs 1 argument");
}

#[test]
fn a_library_word_defined_is_its_alias() {
	assert_eq!(printed("f := floor; f(2.5)"), "2");
	assert_eq!(printed("f := upper; f(\"a\")"), "\"A\"");
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
