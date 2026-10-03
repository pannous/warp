// `print x` where x is a type error reports x's own error, not "print of an Error"; text * number names how to repeat
use crate::common::fails_with;

#[test]
fn print_of_a_type_error_reports_that_error() {
	fails_with("greeting = \"Hello \" + \"🌍\"\nprint greeting*2", "text * int");
}

#[test]
fn text_times_number_says_how_to_repeat_a_text() {
	fails_with("\"ab\"*2", "(2 times [\"ab\"]).join(\"\")");
	fails_with("x=\"ab\"; x*3", "(3 times [x]).join(\"\")");
}
