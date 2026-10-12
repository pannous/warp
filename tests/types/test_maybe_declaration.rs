//! `x: maybe int = 1` declares the optional type `int?`, as `x: int? = 1` does (card maybe-int)
use crate::is;

#[test]
fn a_maybe_declaration_is_optional() {
	is!("x: maybe int = 1; x", 1);
	is!("x: maybe int = ø; x", warp::Node::Empty);
	is!("x: maybe text = \"a\"; x", "a");
}
