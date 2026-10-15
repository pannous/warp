//! card braceless-calls: `sin cos 0`, `max 3 4`, `max 3, 4` and `-sin 0` call their words without parentheses
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn real_functions_nest_without_parentheses() {
	assert_eq!(printed("sin cos 0"), "0.8414709848078965");
	assert_eq!(printed("f(x) := exp x; f(1)"), "2.718281828459045");
}

#[test]
fn min_and_max_take_their_arguments_without_parentheses() {
	assert_eq!(printed("max 3 4"), "4");
	assert_eq!(printed("min 3 4"), "3");
	assert_eq!(printed("max 1 5 2"), "5");
	assert_eq!(printed("max 3, 4"), "4");
}

#[test]
fn the_arguments_after_a_comma_join_a_braceless_call_wherever_it_stands() {
	assert_eq!(printed("x = max 3, 4; x"), "4");
	assert_eq!(printed("f(x) := x * 2; f(max 3, 4)"), "8");
	assert_eq!(printed("min 5, 2, 9"), "2");
	// as max(xs, 0): a list is no candidate beside a number
	assert!(printed("xs = [1 5]; max xs, 0").contains("cannot compare a list"));
}

#[cfg(feature = "native")]
#[test]
fn print_prints_the_braceless_call_of_the_joined_arguments() {
	let output = crate::common::warp_command().args(["--no-ask", "print max 3, 4"]).output().unwrap();
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "4");
}

#[test]
fn a_minus_before_a_call_negates_it() {
	assert_eq!(printed("-sin 0"), "0");
	assert_eq!(printed("-cos 0"), "-1");
	assert_eq!(printed("-floor 2.5"), "-2");
	assert_eq!(printed("a = 5; b = 2; a -b"), "3");
}
