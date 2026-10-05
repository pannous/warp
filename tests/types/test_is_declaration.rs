// `x is number 9` (wiki Features.md, inventions.md). P61 (user, 2026-10-05: "Yes, teach `x be number 9`"): `is` always
// compares; with a name defined nowhere it is an error that teaches the definition `x be number 9`
use crate::common::fails_with;
use warp::is;

#[test]
fn is_with_a_type_and_a_value_teaches_be() {
	fails_with("x is number 3; x * 3", "x be number 3");
	fails_with("n is int 4; n + 1", "n be int 4");
	is!("x be number 3; x * 3", 9);
}

#[test]
fn is_tests_an_assigned_variable() {
	is!("x=9; x is number 9", 1);
	is!("x=8; x is number 9", 0);
	is!("f(x) := x is number 3; f(3)", 1);
	is!("f(x) := x is number 3; f(4)", 0);
}
