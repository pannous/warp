//! Card union-types: an inline union may name declared classes, `s: Circle | Square`, beside builtin types,
//! `p: Point | text`; an instance of another class or a value fitting no part is refused
use crate::common::fails_with;
use crate::is;

const SHAPES: &str = "class Circle{r:int}; class Square{side:int}; class Point{x:int}; area(s: Circle | Square) := match s { Circle{r} => r*r*3; Square{side} => side*side; _ => 0 }; ";

fn shapes(program: &str) -> String {
	format!("{SHAPES}{program}")
}

#[test]
fn a_union_of_classes_takes_an_instance_of_each() {
	is!(&shapes("area(Square(2)) + area(Circle(1))"), 7);
	is!(&shapes("s: Circle | Square = Square(2); area(s)"), 4);
	is!(&shapes("class Box{content: Circle | Square}; b = Box(Square(3)); area(b.content)"), 9);
}

#[test]
fn a_union_of_classes_refuses_another_class() {
	fails_with(&shapes("area(Point(1))"), "Point is none of them");
	fails_with(&shapes("p = Point(1); area(p)"), "Point is none of them");
	fails_with(&shapes("s: Circle | Square = Point(2)"), "Point is none of them");
	fails_with(&shapes("area(3)"), "area needs a Circle or Square for parameter s");
}

#[test]
fn a_union_of_a_class_and_a_builtin_type() {
	is!("class Point{x:int}; p: Point | text = \"ab\"; p", "ab");
	is!("class Point{x:int}; p: text | Point = Point(2); p.x", 2);
	fails_with("class Point{x:int}; p: Point | text = 3", "needs a Point or text");
	fails_with("class Point{x:int}; f(p: Point | int) := p; f(2.5)", "2.5 is no Point or int");
}
