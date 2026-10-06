//! D12 (user, 2026-10-03): an unbracketed `a=1,2,3` / `a=1 2 3` asks whether it is a list or separate statements,
//! fallback Error; a bracketed list never asks
use crate::is;
use crate::common::fails_with;
use warp::node::ints;

#[test]
fn an_unbracketed_list_assignment_asks_list_or_statements() {
	fails_with("a=1,2,3; a", "list or separate statements");
	fails_with("a=1 2 3; a", "(too ambiguous to guess)");
	fails_with("a=1 2 3; a", "`a=[1 2 3]` for a list");
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
