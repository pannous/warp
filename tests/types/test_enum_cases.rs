//! P178 (user, 2026-10-07): enum cases with values are sealed classes, the sum type of the enum: `Shape::Circle(r)`
//! constructs one, and in a match it is the type test plus the binding of its fields.
use crate::is;

const SHAPE: &str = "enum Shape { Circle(r), Rect(w, h), Dot }";

#[test]
fn an_enum_case_with_values_is_a_class() {
	is!(&format!("{SHAPE}; s = Circle(2); s.r"), 2);
	is!(&format!("{SHAPE}; s = Shape::Rect(2, 3); s.h"), 3);
	is!(&format!("{SHAPE}; Shape::Rect(2, 3) is Shape"), true);
	is!(&format!("{SHAPE}; x = Dot; x is Shape"), true);
}

#[test]
fn a_match_on_enum_cases_tests_and_binds() {
	let area = "area(s) := match s { Shape::Circle(r) => 3 * r * r, Shape::Rect(w, h) => w * h, Dot => 0 }";
	is!(&format!("{SHAPE}; {area}; area(Shape::Rect(2, 3))"), 6);
	is!(&format!("{SHAPE}; {area}; area(Circle(2))"), 12);
	is!(&format!("{SHAPE}; {area}; area(Dot)"), 0);
	is!(&format!("{SHAPE}; s = Shape::Rect(2, 3); match s {{ Shape::Circle(r) => r, Shape::Rect(w, h) => w * h, Dot => 0 }}"), 6);
}

#[test]
fn swift_case_lines_with_values() {
	is!("enum Shape { case circle(radius: int)\n case square(side: int) }; s = Shape::square(4); s.side", 4);
}

#[test]
fn an_enum_without_values_stays_numbered() {
	is!("enum Color { red, green }; Color::green", 1);
}
