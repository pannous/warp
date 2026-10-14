//! card sum-helper: the temporaries lowering passes make (`word·sum·3`, `extremum·best·2`, `try·value·1`) are no
//! variables of the program, so a function and main both using sum, max or try ask no local-or-global question
use crate::common::warnings_of;
use warp::Node;

#[test]
fn compiler_temporaries_ask_no_scope_question() {
	for (code, expected) in [
		("total(xs) := sum(xs); total([1 2 3]) + sum([4 5])", 15),
		("biggest(xs) := max(xs); biggest([3 9 2]) + max([1 2])", 11),
		("halved(n) := try n / 0 else 0; halved(4) + (try 4 / 0 else 1)", 1),
		("first_or_zero(xs) := try xs#9 else 0; first_or_zero([1]) + (try [1]#9 else 2)", 2),
	] {
		let (value, warnings) = warnings_of(code);
		assert_eq!(value, Node::from(expected), "{code}");
		assert!(!warnings.iter().any(|warning| warning.contains("make a new local")), "{code}: {warnings:?}");
	}
}
