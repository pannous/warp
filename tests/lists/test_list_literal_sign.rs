//! list-sign (user, 2026-10-12, refines D13): in a list literal a space plus a glued sign starts a new item after
//! anything; spaced or glued operators, indexes and code outside brackets keep subtracting
use crate::is;
use warp::errors::diagnostic::{with_warning_mode, WarningMode};
use warp::node::ints;

#[test]
fn a_glued_sign_starts_the_next_item_after_anything() {
	is!("a=5; [a -1]", ints(vec![5, -1]));
	is!("a=5; [a +1]", ints(vec![5, 1]));
	is!("a=5; [a*2 -1 a]", ints(vec![10, -1, 5]));
	with_warning_mode(WarningMode::Error, || is!("a=5; [a -1]", ints(vec![5, -1])));
}

#[test]
fn spaced_and_glued_operators_still_subtract() {
	is!("a=5; [a - 1]", ints(vec![4]));
	is!("a=5; [a-1]", ints(vec![4]));
	is!("a=5; [a -1, 2]", ints(vec![4, 2]));
	is!("a=5; xs=[1 2 3 4]; xs[a -2]", 4);
	is!("a=5; [(a -1)]", ints(vec![4]));
	is!("a=5; a -1", 4);
}
