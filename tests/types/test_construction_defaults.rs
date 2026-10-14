//! card construction-default: a field default that constructs an instance (`p: Point = Point(0)`) is that instance,
//! made anew by each construction leaving the field out, so two instances never share it (P200)
use crate::is;

const CLASSES: &str = "class Point { x: int }; class A { p: Point = Point(0) }; ";

#[test]
fn a_construction_default_is_an_instance() {
	is!(&format!("{CLASSES}a = A(); a.p.x"), 0);
	is!(&format!("{CLASSES}A{{}}.p.x"), 0);
	is!("class Point { x: int }; class B { q: Point = Point{x: 7} }; B().q.x", 7);
}

#[test]
fn each_instance_gets_its_own_default() {
	is!(&format!("{CLASSES}a = A(); b = A(); a.p.x = 5; b.p.x"), 0);
	is!(&format!("{CLASSES}a = A(); a.p.x = 5; a.p.x"), 5);
}

#[test]
fn a_nested_construction_default_is_constructed_too() {
	is!("class Point { x: int = 3 }; class A { p: Point = Point() }; class C { a: A = A() }; C().a.p.x", 3);
}
