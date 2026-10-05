// A call with more values than the function takes is an error, never values dropped silently
use crate::common::fails_with;
use warp::is;

#[test]
fn extra_or_missing_values_are_errors() {
	fails_with("f(x) := x; f(1, 2)", "f takes 1 argument, got 2");
	fails_with("sin(0, 5)", "sin takes 1 value, got 2");
	fails_with("pow(2)", "pow takes 2 values, got 1");
	fails_with("sqrt(16, 2)", "takes 1 value");
	fails_with("floor(1.5, 2)", "floor takes 1 value");
	is!("f(a, b=2) := a + b; f(1, 5)", 6);
	is!("sin(x) := x * 2; sin(4)", 8);
	is!("pow(2, 3)", 8);
}

#[test]
fn text_builtins_and_library_words_check_their_values_too() {
	fails_with("trim(\"a\", \"b\")", "trim takes 1 value, got 2");
	fails_with("byte_at(\"ab\")", "byte_at takes 2 values, got 1");
	fails_with("x = 1\ny = upper(\"a\", \"b\")", "upper takes 1 argument, got 2 at 2:");
}

#[test]
fn a_default_reads_the_parameters_before_it() {
	is!("f(a, b = a * 2) := a + b; f(3)", 9);
	is!("f(a, b = a * 2) := a + b; f(3, 1)", 4);
	is!("g(x, y = x + 1, z = y * 2) := z; g(1)", 4);
}
