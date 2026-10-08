//! P84 (user 2026-10-05: "This should have already been done with broadcasting"): several juxtaposed arguments of a
//! function of one parameter are one list, `f 1 2 3` = `f [1 2 3]`; a scalar function broadcasts over it
use crate::common::fails_with;
use crate::is;
use warp::warp_parser::parse;

#[test]
fn test_several_arguments_are_one_list() {
	is!("sum := fold +; sum 1 2 3", 6);
	is!("f(x) := count x; f 1 2 3", 3);
}

#[test]
fn test_a_scalar_function_broadcasts_over_them() {
	is!("square:=it*it; square 1 2 3", parse("[1 4 9]"));
	is!("def g(x){x*2}; g 1 2", parse("[2 4]"));
}

#[test]
fn test_parenthesized_arguments_keep_their_count() {
	fails_with("f(x) := x; f(1, 2, 3)", "takes 1 argument");
	is!("add(a, b) := a + b; add 1 2", 3);
}
