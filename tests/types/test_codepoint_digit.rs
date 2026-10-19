//! card codepoint-digit: `as codepoint` of a variable agrees with that of the literal (a digit 0…9 is its character),
//! and a number that names no code point is an error, which `try` catches
use crate::common::fails_with;
use crate::is;

#[test]
fn a_variable_casts_like_its_literal() {
	is!("5 as codepoint", '5');
	is!("x = 5; x as codepoint", '5');
	is!("x = 65; x as codepoint", 'A');
}

#[test]
fn a_number_naming_no_code_point_is_an_error() {
	for number in ["-1", "1114112", "55296"] {
		fails_with(&format!("x = {number}; x as codepoint"), "invalid code point");
	}
	is!("x = 55296; try (x as codepoint) else \"fallback\"", "fallback");
}

/// A character of a text is held as a one-character text: as codepoint it is that character (card text-char)
#[test]
fn a_character_of_a_text_casts_to_itself() {
	is!("s = \"ab\"; s#2 as codepoint", 'b');
	is!("s = \"ab\"; s[1] as char", 'b');
	is!("h(s: text, at: int): codepoint := s[at]; h(\"ab\", 1)", 'b');
	is!("g(c: codepoint): codepoint := c; s = \"ab\"; g(s#2)", 'b');
}
