//! `T or error` is a type (notes/decisions.md error-type): a result or variable declared so holds a T or an Error
use crate::common::fails_with;
use crate::is;

const F: &str = "def f(x) as int or error := if x < 0 then error(\"neg\") else x; ";

#[test]
fn test_a_result_declared_int_or_error_holds_both() {
	is!(&format!("{F}f(2)"), 2);
	is!(&format!("{F}r = f(-1); if r failed then 1 else 2"), 1);
}

#[test]
fn test_a_variable_declared_int_or_error_holds_both() {
	is!("x: int or error = 3; x", 3);
	is!("x: int or error = error(\"e\"); if x failed then 1 else 2", 1);
	fails_with("x: int or error = \"a\"; x", "not an int or error");
}

#[test]
fn test_every_builtin_type_joins_a_union() {
	is!("x: symbol or int = 3; x", 3);
}
