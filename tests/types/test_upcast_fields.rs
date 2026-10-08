//! P201 (from P203: annotations make code strict, card upcast-field): a field of a subclass or variant read through the
//! declared parent type is a compile error that hints at a match or a cast; a cast checks the class at run time, a type
//! test smart-casts, and unannotated code reads the field at run time.
use crate::common::fails_with;
use crate::is;

const SHAPES: &str = "class Shape { name: text }; class Circle extends Shape { r: int }; ";

#[test]
fn a_subclass_field_through_the_declared_parent_is_an_error() {
	fails_with(&format!("{SHAPES}s: Shape = Circle(\"a\", 2); s.r"), "Shape has no field r");
	fails_with(&format!("{SHAPES}f(s: Shape) := s.r; f(Circle(\"a\", 2))"), "Shape has no field r");
	fails_with("type Color = red | rgb(r: int, g: int, b: int); c: Color = rgb(1, 2, 3); c.r", "Color has no field r");
	fails_with(&format!("{SHAPES}s: Shape = Circle(\"a\", 2); s.r"), "s as Circle");
}

#[test]
fn a_cast_or_a_match_reads_it() {
	is!(&format!("{SHAPES}s: Shape = Circle(\"a\", 2); (s as Circle).r"), 2);
	is!(&format!("{SHAPES}s: Shape = Circle(\"a\", 2); c = s as Circle; c.r"), 2);
	is!(&format!("{SHAPES}s: Shape = Circle(\"a\", 2); match s {{ Circle(r) => r; _ => 0 }}"), 2);
	is!(&format!("{SHAPES}s: Shape = Circle(\"a\", 2); if s is Circle {{ s.r }} else {{ 0 }}"), 2);
	fails_with(&format!("{SHAPES}s: Shape = Shape(\"a\"); (s as Circle).r"), "is no Circle");
}

#[test]
fn unannotated_code_reads_it_at_run_time() {
	is!(&format!("{SHAPES}s = Circle(\"a\", 2); s.r"), 2);
	is!(&format!("{SHAPES}s: Shape = Circle(\"a\", 2); s.name"), "a");
	is!(&format!("{SHAPES}s: Circle = Circle(\"a\", 2); s.r"), 2);
}
