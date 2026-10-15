//! D12 (user, 2026-10-03): an unbracketed `a=1,2,3` / `a=1 2 3` asks whether it is a list or separate statements,
//! fallback Error; a bracketed list never asks
use crate::common::fails_with;
use crate::is;
use warp::node::ints;

#[test]
fn an_unbracketed_list_assignment_asks_list_or_statements() {
	fails_with("a=1,2,3; a", "list or separate statements");
	fails_with("a=1,2,3; a", "(too ambiguous to guess)");
	fails_with("a=1,2,3; a", "`a=[1, 2, 3]` for a list");
}

/// reconsider-spaced (user 2026-10-15): spaced values are a list, after `=` and on a line of their own; a glued sign
/// (`x = 5 -2`) still asks
#[test]
fn spaced_values_are_a_list() {
	is!("a=1 2 3; a", ints(vec![1, 2, 3]));
	is!("xs = 1+1 2+2; xs", ints(vec![2, 4]));
	is!("2⁰ 2¹ 2²", ints(vec![1, 2, 4]));
	is!("xs = \"a\" \"b\"; count xs", 2);
	fails_with("x = 5 -2; x", "one list or separate statements");
}

#[test]
fn each_reading_has_its_explicit_form() {
	is!("a=[1,2,3]; a", ints(vec![1, 2, 3]));
	is!("a=[1 2 3]; count a", 3);
	is!("a=1; 2; 3; a", 1);
}

#[test]
fn a_bracketed_list_never_asks() {
	is!("a=[1,2,3]; a", ints(vec![1, 2, 3]));
	is!("a=[1 2 3]; count a", 3);
	is!("a=(1,2,3); count a", 3);
	is!("a=1; b=2; a+b", 3);
	is!("x=1\ny=2\nx+y", 3);
}
