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

/// `T?` is T, ø or an Error (decision error-optional), also written before the name and as a result type
#[test]
fn test_an_optional_type_before_a_name_or_as_a_result() {
	is!("int? i = 3; i", 3);
	is!("int? i = ø; if i is ø then 1 else 2", 1);
	is!("def f(x) as int? := x; f(2)", 2);
	is!("def f(x) as int? := if x < 0 then error(\"neg\") else x; r = f(-1); if r failed then 1 else 2", 1);
	is!("c = 1; c ? 2 : 3", 2);
}

#[test]
fn test_every_builtin_type_joins_a_union() {
	is!("x: symbol or int = 3; x", 3);
	for other in ["symbol", "key", "code", "error", "time", "operation", "list"] {
		fails_with(&format!("x: {other} or int = 2.5\nx"), "cannot assign float 2.5");
	}
}
