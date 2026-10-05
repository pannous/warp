// P45 (user, 2026-10-05): a variable given values of two kinds that do not mix is a compile error naming both kinds,
// "compile error unless we are in script mode, which is not defined yet". Int → Float widening stays.
use crate::common::fails_with;
use warp::is;

#[test]
fn a_variable_given_another_kind_is_a_compile_error() {
	fails_with("x = 5; x = [1]; x", "x was an Int, is given a List: use another name");
	fails_with("x = [1,2]; x = 5; x + 1", "x was a List, is given an Int");
	fails_with("x = \"a\"; x = 5; x + 1", "x was a Text, is given an Int");
}


#[test]
fn a_function_body_is_checked_too() {
	fails_with("f() := (x = 1; x = \"a\"; x); f()", "x was an Int, is given a Text");
}

#[test]
fn kinds_that_mix_stay_allowed() {
	is!("x = 1; x = 2.5; x", 2.5);
	is!("x = \"a\"; x = 'b'; x", 'b');
	is!("x = 1; x = 2; x", 2);
	is!("xs = [1]; xs = [2 3]; xs#2", 3);
}
