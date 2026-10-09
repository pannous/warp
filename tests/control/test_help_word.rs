//! `help(x)` (card help-help, notes/reflection.md): a class's, an instance's or a function's description as text,
//! from the same compile-time knowledge as `dir(x)`, `x.fields`, `f.signature`
use crate::is;

const POINT: &str = "class Point { x: int; y: int; length() := sqrt(x*x + y*y) }\n";

#[test]
fn help_of_a_class_and_of_its_instance() {
	is!(&format!("{POINT}help(Point)"), "class Point\nfields: x y\nmethods: length");
	is!(&format!("{POINT}p = Point(3, 4)\nhelp(p)"), "class Point\nfields: x y\nmethods: length");
}

#[test]
fn help_of_a_subclass_names_its_parent() {
	is!("class A { a: int }\nclass B extends A { b: int }\nhelp(B)", "class B extends A\nfields: a b");
}

#[test]
fn help_of_a_function_is_its_signature() {
	is!("area(width: int, height: int) := width * height\nhelp(area)", "area(width:int, height:int) -> int");
}

#[test]
fn a_programs_own_help_wins() {
	is!("help(x) := x + 1\nhelp(2)", 3);
}
