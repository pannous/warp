//! #35 (user decision 2026-10-03): an int list `as string` joins to "[1 2]" also at runtime;
//! `"x" as float` is a loud error with a hint, only single-quote codepoints convert to numbers.

use crate::common::fails_with;
use crate::is;

#[test]
fn an_int_list_variable_as_string_is_its_text() {
	is!("x=[1 2]; x as string", "[1 2]");
	is!("x=[1 2 3]; y = x as string; y", "[1 2 3]");
	is!("x=[7]; x as string", "[7]");
}

#[test]
fn a_double_quoted_text_as_float_is_loud() {
	fails_with("\"x\" as float", "as float");
	fails_with("\"x\" as float", "fix: codepoint('x') as float"); // P74: codepoint is the preferred name
	fails_with("\"x\" as int", "a text is no number");
	is!("\"5\" as int", 5); // a numeric text still converts
}

#[test]
fn a_single_quoted_codepoint_still_converts() {
	is!("ord('x')", 120);
}
