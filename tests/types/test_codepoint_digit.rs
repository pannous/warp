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
