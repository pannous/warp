// `print x` where x is a type error reports x's own error, not "print of an Error"; text * int repeats (user decision 2026-10-03)
use crate::common::fails_with;

#[test]
fn print_of_a_type_error_reports_that_error() {
	fails_with("greeting = \"Hello \" + \"🌍\"\nprint greeting*2.5", "text * float"); // 2.5 is a float (decision exact-default)
}

#[test]
fn text_times_number_says_how_to_repeat_a_text() {
	crate::is!("\"ab\"*2", "abab");
	crate::is!("x=\"ab\"; x*3", "ababab");
	fails_with("\"ab\"*2.5", "`2.5 times \"ab\"`");
}
