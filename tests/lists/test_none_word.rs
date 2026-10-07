// Card none-call: `none(xs, f)` is the opposite of `any(xs, f)` (Kotlin's none); a bare `none` stays the null
use crate::is;

#[test]
fn none_of_a_list_is_the_opposite_of_any() {
	is!("none([1, 2], x => x > 5)", 1);
	is!("none([1, 2], x => x > 1)", 0);
	is!("[1, 2].none(x => x > 5)", 1);
	is!("none([0, 0])", 1);
	is!("none([0, 3])", 0);
}

#[test]
fn a_bare_none_is_null() {
	is!("x = none; x", warp::Node::Empty);
}
