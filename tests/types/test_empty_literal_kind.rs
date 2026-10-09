//! A literal ø is the empty value to the type inference, not an Int (card infer-type): a branch giving ø keeps it,
//! where it was read as a number and failed with "not a number"
use crate::is;
use warp::Node;

#[test]
fn a_branch_giving_empty_gives_empty() {
	is!("c = 1; v = c > 0 ? ø : 3; v", Node::Empty);
	is!("f(c) := c > 0 ? ø : c; f(1)", Node::Empty);
	is!("f(c) := c > 0 ? ø : c; f(0) + 2", 2);
}

#[test]
fn an_unset_number_field_starts_at_zero() {
	is!("class Box{w:int; h:int; value(side){ w = side }}; b = Box(3); b.h", 0);
}
