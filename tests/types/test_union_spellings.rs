//! Card undeclared-union: every part of an inline union names a builtin type or a declared class, `x: int | zork` is
//! refused for a variable, a parameter and a field. User decision 2026-10-12: `or` spells the union as `|` does, in
//! every position, while `a or b` of values stays the boolean or
use crate::common::fails_with;
use crate::is;

const SHAPES: &str = "class Circle{r:int}; class Square{side:int}; area(s: Circle or Square) := match s { Circle{r} => r*r*3; Square{side} => side*side }; ";

#[test]
fn a_union_part_that_names_no_type_is_refused() {
	fails_with("x: int | zork = 3", "zork is neither");
	fails_with("x: zork or int = 3", "zork is neither");
	fails_with("f(a: int or zork) := a; f(2)", "parameter a is declared int or zork");
	fails_with("class B{size: int | zork}; B(2)", "field B.size is declared int or zork");
}

#[test]
fn or_spells_a_union_of_a_variable() {
	is!("x: int or text = \"ab\"; x", "ab");
	fails_with("x: int or text = 2.5", "x is declared int or text");
}

#[test]
fn or_spells_a_union_of_a_parameter() {
	is!("f(a: int or float) := a; f(2.5)", 2.5);
	is!(&format!("{SHAPES}area(Square(2))"), 4);
	fails_with("f(a: int or text) := a; f(2.5)", "2.5 is no int or text");
}

#[test]
fn or_spells_a_union_of_a_field() {
	is!("class B{size: int or text}; B(\"x\").size", "x");
	fails_with("class B{size: int or text}; B(2.5)", "B.size is int or text");
}

#[test]
fn or_of_values_stays_the_boolean_or() {
	is!("x = 0; x or 5", 5);
	is!("yes or no", true);
}
