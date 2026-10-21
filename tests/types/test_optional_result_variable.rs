//! A function declared `: T` whose last value is a variable that can hold ø warns, with `: T?` as its fix, and the ø
//! reaches the caller unchanged (decision optional-result, card pure-lane-declared)
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
fn a_variable_never_empty_stays_quiet() {
	let (value, warnings) = warnings_of("def f(x: int): list { found = [x]; found = found + [x]; found }; f(1)");
	assert_eq!(value, warp::ints(vec![1, 1]));
	assert!(warnings.is_empty(), "{warnings:?}");
	let (value, warnings) = warnings_of("def f(x: int): int { a = x; a = a + 1; a }; f(1)");
	assert_eq!(value, Node::from(2));
	assert!(warnings.is_empty(), "{warnings:?}");
}
