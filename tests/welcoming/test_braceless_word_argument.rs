// card p68-warning: the P68 warning holds for every braceless call that takes arithmetic, not only program functions:
// library words (floor, sin), operator words (sqrt, abs) and program functions name both readings alike
use warp::errors::diagnostic::take_warnings;
use warp::wasm_emitter::eval;
use warp::Node;

fn warnings_of(code: &str) -> (Node, Vec<String>) {
	take_warnings();
	let value = eval(code);
	(value, take_warnings().iter().map(|warning| warning.to_string()).collect())
}

fn assert_warns(code: &str, whole: &str, first: &str) {
	let (_, warnings) = warnings_of(code);
	assert!(warnings.iter().any(|warning| warning.contains(whole) && warning.contains(first)), "{code}: {warnings:?}");
}

#[test]
fn every_braceless_call_taking_arithmetic_warns() {
	assert_warns("floor 2.5 + 1", "floor(2.5 + 1)", "floor(2.5) + 1");
	assert_warns("sin 0 + 1", "sin(0 + 1)", "sin(0) + 1");
	assert_warns("sqrt 4 + 5", "sqrt(4 + 5)", "sqrt(4) + 5");
	assert_warns("abs -3 + 1", "abs(-3 + 1)", "abs(-3) + 1");
	assert_warns("r(x) := x * 2; r 4 + 5", "r(4 + 5)", "r(4) + 5");
}

#[test]
fn the_reading_stays_the_whole_expression() {
	assert_eq!(warnings_of("sqrt 4 + 5").0, Node::int(3));
	assert_eq!(warnings_of("floor 2.5 + 1").0, Node::int(3));
}

#[test]
fn unambiguous_forms_do_not_warn() {
	for code in ["sqrt 9", "√4 + 5", "sqrt(4) + 5", "floor(2.5) + 1", "print 1 + 2", "x = 3; x + 4"] {
		let (_, warnings) = warnings_of(code);
		assert!(
			warnings.iter().all(|warning| !warning.contains("taking the whole expression")),
			"{code}: {warnings:?}"
		);
	}
}

// reviewer: a call without a value has one reading, whatever its word: print, a program function returning nothing
#[test]
fn a_call_without_a_value_does_not_warn() {
	for code in ["print 1 + 2", "say(x) := print x; say 1 + 2", "sleep 0 + 0"] {
		let (_, warnings) = warnings_of(code);
		assert!(
			warnings.iter().all(|warning| !warning.contains("taking the whole expression")),
			"{code}: {warnings:?}"
		);
	}
}
