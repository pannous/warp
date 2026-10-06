//! `x failed` (wiki/optional.md, Error.md): true when x holds an Error value, unlike `missing`/`empty`, which test for ø
use crate::is;

#[test]
fn test_failed_tests_for_an_error_value() {
	is!("x=error(\"bad\"); if x failed {1} else {2}", 1);
	is!("x=3; if x failed {1} else {2}", 2);
	is!("x=ø; if x failed {1} else {2}", 2); // missing is not failed
	is!("x=error(\"bad\"); if x is failed {1} else {2}", 1);
}

#[test]
fn test_failed_ends_a_condition_like_the_other_test_words() {
	is!("x=error(\"bad\"); y=1; if x failed and y {1} else {2}", 1);
	is!("x=3; if x failed then 1 else 2", 2);
}
