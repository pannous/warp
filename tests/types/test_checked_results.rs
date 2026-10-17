//! Card lossy-return (user 2026-10-12): a declared result converts only when nothing is lost; a fraction reaching an
//! int result is an error, and `as int` truncates on purpose
use crate::common::fails_with;
use crate::is;

#[test]
fn a_fraction_reaching_an_int_result_is_an_error() {
	fails_with("def f() as int { 2.5 }; f()", "the result is declared int, cannot assign float 2.5"); // known statically
	fails_with("def half(x) as int { x / 2 }; half(3)", "an int must be a whole number");
	is!("def half(x) as int { x / 2 }; half(4)", 2);
	is!("def half(x) as int { (x / 2) as int }; half(3)", 1);
}

#[test]
fn a_lossless_result_still_converts() {
	is!("def f(x) as text { 3 }; f(2)", "3");
	is!("def f(x) as float { x / 2 }; f(3)", 1.5);
	is!("def f(x) as bool { 1 }; f(2)", true);
}
