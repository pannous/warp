// `//` starts a comment only when a space or the end of the line follows it; glued to its divisor it divides
// (user decision 2026-10-08, notes/decisions.md)
use crate::is;

#[test]
fn slash_slash_without_space_divides() {
	is!("x = 7; x //= 3\nx", 2);
	is!("x = 7; x//=3; x", 2);
	is!("7 //2", 3);
	is!("a = 7; a //2", 3);
	is!("7//2", 3);
}

#[test]
fn slash_slash_with_space_is_a_comment() {
	is!("x = 7 // note\nx", 7);
	is!("x = 7 // 2\nx", 7);
	is!("x = 7 //\nx", 7);
	is!("//note\n3", 3);
}
