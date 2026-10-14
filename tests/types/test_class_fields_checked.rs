//! Card class-field: a value written to a place declared of a class is admitted as the declaration was: a later
//! `s = v` of `s: C = …`, a constructor argument and a field write of a field declared `content: C` or `C | D`
use crate::common::fails_with;
use crate::is;

const SHAPES: &str = "class Circle{r:int}; class Square{side:int}; class Point{x:int}; ";

fn shapes(program: &str) -> String {
	format!("{SHAPES}{program}")
}

#[test]
fn a_declared_variable_keeps_its_class() {
	fails_with(&shapes("s: Circle = Circle(1); s = Point(2)"), "Point is not a Circle");
	fails_with(&shapes("s: Circle | Square = Circle(1); s = Point(2)"), "Point is none of them");
	is!(&shapes("s: Circle | Square = Circle(1); s = Square(2); s.side"), 2);
}

#[test]
fn a_function_local_of_the_same_name_is_its_own() {
	is!(&shapes("s: Circle = Circle(1); f() := { s = Point(2); s.x }; f()"), 2);
}

#[test]
fn a_field_of_a_class_takes_its_declared_class() {
	fails_with(&shapes("class Box{content: Circle}; b = Box(Point(3))"), "Box.content is a Circle");
	fails_with(&shapes("class Box{content: Circle | Square}; b = Box(Point(3))"), "Point is none of them");
	is!(&shapes("class Box{content: Circle | Square}; b = Box(Square(3)); b.content.side"), 3);
}

#[test]
fn a_field_write_takes_its_declared_class() {
	fails_with(
		&shapes("class Box{content: Circle}; b = Box(Circle(1)); b.content = Point(1)"),
		"Point is not a Circle",
	);
	is!(&shapes("class Box{content: Circle | Square}; b = Box(Circle(1)); b.content = Square(2); b.content.side"), 2);
}
