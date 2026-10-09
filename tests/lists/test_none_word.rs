// Card none-call: `none` is the null; P185 (user, 2026-10-07): called with arguments, `none(xs, f)` is an error naming
// the null (it was briefly Kotlin's "no element matches"); `not any(xs, f)` says that
use crate::is;

#[test]
fn none_called_is_an_error_naming_the_null() {
	for code in ["none([1, 2], x => x > 5)", "[1, 2].none(x => x > 5)", "none([0, 0])"] {
		crate::common::fails_with(code, "none is the null ø, not a function");
	}
	is!("not any([1, 2], x => x > 5)", 1);
}

#[test]
fn a_bare_none_is_null() {
	is!("x = none; x", warp::Node::Empty);
}
