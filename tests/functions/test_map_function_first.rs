//! card map-function: the function may come first in an iteration call, a library word, a reference or a variable;
//! the list may be named by its label, `map f on: xs` (card labels-calls)
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_library_function_before_on() {
	assert_eq!(printed("map floor on [1.5 2.5]"), "[1 2]");
	assert_eq!(printed("map function floor on [1.5 2.5]"), "[1 2]");
	assert_eq!(printed("map floor on: [1.5 2.5]"), "[1 2]");
	assert_eq!(printed("map function floor on: [1.5 2.5]"), "[1 2]");
}

#[test]
fn a_function_reference_as_first_argument() {
	assert_eq!(printed("map(function floor, [1.5 2.5])"), "[1 2]");
	assert_eq!(printed("map(floor, [1.5 2.5])"), "[1 2]");
	assert_eq!(printed("map([1.5 2.5], floor)"), "[1 2]");
}

#[test]
fn a_function_variable_before_the_labeled_list() {
	assert_eq!(printed("square := it*it; f = function square; map f on: [2 3]"), "[4 9]");
}

#[test]
fn a_prefix_operator_word_as_function() {
	assert_eq!(printed("[4 9].map(function sqrt)"), "[2 3]");
	assert_eq!(printed("[-1 2].map(function abs)"), "[1 2]");
}

/// card map-sqrt: under an iteration word an operator word is its function, never applied to the next word
#[test]
fn a_prefix_operator_word_in_every_order() {
	assert_eq!(printed("map abs [-1 2]"), "[1 2]");
	assert_eq!(printed("xs = [-1 3]; map abs xs"), "[1 3]");
	assert_eq!(printed("map [-1 2] abs"), "[1 2]");
	assert_eq!(printed("map abs on [-1 2]"), "[1 2]");
	assert_eq!(printed("map sqrt on [4 9]"), "[2 3]");
	assert_eq!(printed("map sqrt on: [4 9]"), "[2 3]");
	assert_eq!(printed("-7 abs"), "7");
}

/// one rule: wherever a parameter expects a function, an operator word is that function, as a library word is
#[test]
fn a_prefix_operator_word_for_a_function_parameter() {
	assert_eq!(printed("apply(f, x) := f(x); apply floor 2.5"), "2");
	assert_eq!(printed("apply(f, x) := f(x); apply sqrt 9"), "3");
	assert_eq!(printed("apply(f, x) := f(x); apply abs -3"), "3");
}

/// card map-library-word
#[test]
fn a_library_word_before_the_list() {
	assert_eq!(printed("map floor [1.5 2.5]"), "[1 2]");
	assert_eq!(printed("map reverse [[1 2] [3 4]]"), "[[2 1] [4 3]]");
}
