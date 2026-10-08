//! Wiki row 29 (mutable.md): `add "c" to x` and `x.add("c")` of a text variable grow the text; of a list they append
use crate::is;
use warp::warp_parser::parse;

#[test]
fn test_add_to_a_text() {
	is!("x=\"ab\"; add \"c\" to x; x", "abc");
	is!("x=\"ab\"; add \"cd\" to x; x", "abcd");
	is!("x=\"ab\"; x.add(\"c\"); x", "abc");
	is!("x=\"a\"; x = x + \"b\"; add \"c\" to x; x", "abc");
}

#[test]
fn test_add_to_a_list_still_appends() {
	is!("x=[1 2]; add 3 to x; x", parse("[1 2 3]"));
	is!("xs=[\"a\"]; add \"b\" to xs; count xs", 2);
}
