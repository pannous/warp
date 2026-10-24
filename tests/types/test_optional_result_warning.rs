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

/// A list result autocasts ø to [] without a warning (user 2026-10-22, card list-result-autocast); other types still warn
#[test]
fn a_list_result_of_empty_is_the_empty_list() {
	let (value, warnings) = crate::common::warnings_of("def found(x: int): ints { if x > 0 { [x] } }; found(-1)");
	assert_eq!(value, Node::empty_list());
	assert!(warnings.is_empty(), "{warnings:?}");
	is!("def found(x: int): list of int { if x > 0 { [x] } }; found(3)", warp::ints(vec![3]));
	warns_with("def half(x: int): int { if x < 0 { return ø }; x / 2 }; half(-1)", Node::Empty, CAN_ANSWER_EMPTY);
}

/// `warp fix` (beautify) applies the warning's fix: the declared result gains its `?`, a parameter's type stays
#[test]
fn warp_fix_declares_the_result_optional() {
	let fixed = |code: &str| warp::formatting::beautify(code).expect("it parses").text;
	assert_eq!(
		fixed("def half(x: int): int {\n\tif x < 0 { return ø }\n\tx / 2\n}\nhalf(4)\n"),
		"def half(x: int): int? {\n\tif x < 0 { return ø }\n\tx / 2\n}\nhalf(4)\n"
	);
	let declared = "def half(x: int): int? {\n\tif x < 0 { return ø }\n\tx / 2\n}\nhalf(4)\n";
	assert_eq!(fixed(declared), declared);
}
