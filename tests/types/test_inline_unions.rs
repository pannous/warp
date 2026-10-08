//! Card inline-union: `x: int | text` (also `int or text`, `(int|text)`) declares a slot holding either kind, the
//! anonymous form of P179's sum types; a value fitting no part is refused, a part another part covers is dropped
use crate::common::fails_with;
use crate::is;

#[test]
fn an_inline_union_holds_either_kind() {
	is!("x: int | text = 3; x", 3);
	is!("x: int | text = 3; x = \"ab\"; x", "ab");
	is!("x: (int|text) = 3; x = \"ab\"; x", "ab");
	is!("x: int or text = \"ab\"; x = 4; x + 1", 5);
	is!("x: float | text = 2; x", 2);
}

#[test]
fn an_inline_union_refuses_a_value_fitting_no_part() {
	fails_with("x: int | text = 3; x = 2.5", "x is declared int or text");
	fails_with("x: int | text = [1 2]", "x is declared int or text");
	fails_with("x: int = [1 2]", "x is declared int");
}

#[test]
fn a_covered_part_is_dropped() {
	fails_with("x: int | float = 3; x = \"ab\"", "x is declared float,");
}

#[test]
fn a_part_that_is_no_builtin_type_is_refused() {
	fails_with("class point { x: int }; p: point | text = \"a\"", "point");
}

#[test]
fn an_inline_union_parameter_takes_either_kind() {
	is!("f(x: int | text) := x; f(\"ab\")", "ab");
	is!("f(x: int or text) := x; f(3)", 3);
	fails_with("f(x: int | text) := x; f(2.5)", "2.5 is no int");
}
