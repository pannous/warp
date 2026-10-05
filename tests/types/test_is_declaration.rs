// `x is number 9` (wiki Features.md "As keyword for typing", inventions.md `x is number 3`): for a name not assigned
// otherwise it declares x a number holding 9, like `x as number = 9`; for a variable it tests type and value
use crate::common::fails_with;
use warp::is;

#[test]
fn is_with_a_type_and_a_value_declares_a_new_name() {
	is!("x is number 3; x * 3", 9);
	is!("n is int 4; n + 1", 5);
	fails_with("x is int 2.5; x", "x is declared int");
}

#[test]
fn is_tests_an_assigned_variable() {
	is!("x=9; x is number 9", 1);
	is!("x=8; x is number 9", 0);
	is!("f(x) := x is number 3; f(3)", 1);
	is!("f(x) := x is number 3; f(4)", 0);
}
