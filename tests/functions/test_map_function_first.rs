//! card map-function: the function may come first in an iteration call, a library word, a reference or a variable
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_library_function_before_on() {
	assert_eq!(printed("map floor on [1.5 2.5]"), "[1 2]");
	assert_eq!(printed("map function floor on [1.5 2.5]"), "[1 2]");
}

#[test]
fn a_function_reference_as_first_argument() {
	assert_eq!(printed("map(function floor, [1.5 2.5])"), "[1 2]");
	assert_eq!(printed("map(floor, [1.5 2.5])"), "[1 2]");
	assert_eq!(printed("map([1.5 2.5], floor)"), "[1 2]");
}

#[test]
fn a_function_variable_before_on() {
	assert_eq!(printed("square := it*it; f = function square; map f on [2 3]"), "[4 9]");
}

#[test]
fn a_prefix_operator_word_as_function() {
	assert_eq!(printed("[4 9].map(function sqrt)"), "[2 3]");
	assert_eq!(printed("[-1 2].map(function abs)"), "[1 2]");
}
