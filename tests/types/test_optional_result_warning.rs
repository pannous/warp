//! A function declared `: T` that can answer ø warns, with `: T?` as its fix, and the ø reaches the caller unchanged
//! (decision optional-result, card optional-result-warning)
use crate::common::warns_with;
use crate::is;
use warp::Node;

const CAN_ANSWER_EMPTY: &str = "can answer ø; declare";

#[test]
fn a_returned_empty_warns_and_reaches_the_caller() {
	is!("def evens(x: int): list { if x < 0 { return ø }; [x, x+2] }; evens(-1)", Node::empty_list());
	warns_with("def half(x: int): int { if x < 0 { return ø }; x / 2 }; half(-1)", Node::Empty, CAN_ANSWER_EMPTY);
	warns_with("def name(x: int): text { if x < 0 { return }; \"n\" }; name(-1)", Node::Empty, CAN_ANSWER_EMPTY);
	warns_with("def ratio(x: int): float { if x < 0 { return ø }; x / 2 }; ratio(-1)", Node::Empty, CAN_ANSWER_EMPTY);
	is!("def half(x: int): int { if x < 0 { return ø }; x / 2 }; half(4)", 2);
}

#[test]
fn an_if_without_else_or_a_loop_last_can_answer_empty() {
	warns_with("def pick(x: int): int { if x > 0 { x } }; pick(-1)", Node::Empty, CAN_ANSWER_EMPTY);
	warns_with("int pick(x: int) { if x > 0 { return x } }; pick(-1)", Node::Empty, CAN_ANSWER_EMPTY);
	warns_with(
		"def first_big(xs: list): int { for x in xs { if x > 5 { return x } } }; first_big([1, 2])",
		Node::Empty,
		CAN_ANSWER_EMPTY,
	);
	warns_with("def sign(x: int): text { x > 0 ? \"+\" : ø }; sign(-1)", Node::Empty, CAN_ANSWER_EMPTY);
}

#[test]
fn a_result_that_cannot_be_empty_or_is_declared_optional_stays_quiet() {
	let (value, warnings) =
		crate::common::warnings_of("def sign(x: int): int { if x > 0 { return 1 } else { return -1 } }; sign(2)");
	assert_eq!(value, Node::from(1));
	assert!(warnings.is_empty(), "{warnings:?}");
	let (value, warnings) =
		crate::common::warnings_of("def half(x: int): int? { if x < 0 { return ø }; x / 2 }; half(-1)");
	assert_eq!(value, Node::Empty);
	assert!(warnings.is_empty(), "{warnings:?}");
}
