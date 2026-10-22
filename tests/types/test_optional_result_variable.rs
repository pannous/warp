//! A function declared `: T` whose last value is a variable that can hold ø warns, with `: T?` as its fix, and the ø
//! reaches the caller unchanged; so does a call of a function whose result can be ø (decision optional-result, cards
//! pure-lane-declared, optional-result-flow)
use crate::common::{warnings_of, warns_with};
use crate::is;
use warp::Node;

const CAN_ANSWER_EMPTY: &str = "can answer ø; declare";

#[test]
fn a_variable_assigned_empty_can_answer_empty() {
	let found = "def f(x: int): list { found = ø; if x > 0 { found = [x] }; found }";
	warns_with(&format!("{found}; f(0)"), Node::Empty, CAN_ANSWER_EMPTY);
	is!(&format!("{found}; f(2)"), warp::ints(vec![2]));
	warns_with("def f(x: int): int { a = ø; b = a; if x > 0 { b = x }; b }; f(0)", Node::Empty, CAN_ANSWER_EMPTY);
	warns_with("def f(x: int): text { found = if x > 0 then \"big\"; found }; f(0)", Node::Empty, CAN_ANSWER_EMPTY);
}

#[test]
fn a_call_of_a_function_that_can_answer_empty_can_answer_empty() {
	let find = "def find(x: int): int? { if x > 0 { return x }; ø }";
	warns_with(&format!("{find}; def first(x: int): int {{ find(x) }}; first(0)"), Node::Empty, CAN_ANSWER_EMPTY);
	warns_with(
		&format!("{find}; def first(x: int): int {{ return find(x) }}; first(0)"),
		Node::Empty,
		CAN_ANSWER_EMPTY,
	);
	warns_with(
		&format!("{find}; def first(x: int): int {{ find(x) }}; def outer(x: int): int {{ first(x) }}; outer(0)"),
		Node::Empty,
		CAN_ANSWER_EMPTY,
	);
	is!(&format!("{find}; def first(x: int): int {{ find(x) }}; first(3)"), 3);
}

#[test]
fn a_variable_never_empty_stays_quiet() {
	let (value, warnings) = warnings_of("def f(x: int): list { found = [x]; found = found + [x]; found }; f(1)");
	assert_eq!(value, warp::ints(vec![1, 1]));
	assert!(warnings.is_empty(), "{warnings:?}");
	let (value, warnings) = warnings_of("def f(x: int): int { a = x; a = a + 1; a }; f(1)");
	assert_eq!(value, Node::from(2));
	assert!(warnings.is_empty(), "{warnings:?}");
	// a call's ø kept in a variable is tested before it is given back
	let (value, warnings) = warnings_of(
		"def find(x: int): int? { if x > 0 { return x }; ø }; def f(x: int): int { y = find(x); if y == ø { return 0 }; y }; f(0)",
	);
	assert_eq!(value, Node::from(0));
	assert!(warnings.is_empty(), "{warnings:?}");
}
