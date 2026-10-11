//! Errors are values (Decided #1, notes/unwrap.md): a function that may return `error(…)` returns "int or error", the
//! error stays in the variable holding the result, and it is raised only where that value is used as a number.
use crate::common::fails_with;
use crate::is;

const F: &str = "f(x) := if x < 0 then error(\"neg\") else x; ";

#[test]
fn test_a_returned_error_stays_a_value() {
	is!(&format!("{F}r = f(-1); if r failed then 1 else 2"), 1);
	is!(&format!("{F}r = f(3); if r failed then 1 else 2"), 2);
}

#[test]
fn test_a_returned_error_raises_where_used_as_a_number() {
	is!(&format!("{F}f(3) + 1"), 4);
	fails_with(&format!("{F}r = f(-1); r + 1"), "neg");
}
