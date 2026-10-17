// card parameter-patterns: a parameter may be a pattern, as a match arm is: `add_pair([a, b]) := a + b` takes a list of
// two and names its items, `sum(Point{x, y}) := x + y` a Point and names its fields
use crate::common::fails_with;
use crate::is;

const POINT: &str = "class Point { x: int; y: int }";

#[test]
fn a_list_parameter_names_its_items() {
	is!("add_pair([a, b]) := a + b; add_pair([3, 4])", 7);
	is!("second([_, b, _]) := b; second([1, 2, 3])", 2);
	is!("f(n, [a, b]) := n * a + b; f(10, [3, 4])", 34);
}

#[test]
fn a_class_parameter_names_its_fields() {
	is!(&format!("{POINT}; sum(Point{{x, y}}) := x + y; sum(Point(1, 2))"), 3);
	fails_with(&format!("{POINT}; total(Point{{x, y}}) := x + y; total(5)"), "needs a Point");
}

#[test]
fn pattern_names_leave_a_global_of_the_same_name_alone() {
	is!(&format!("{POINT}; total(Point{{x, y}}) := x + y; x = 10; total(Point{{x: 3, y: 4}}) + x"), 17);
}

#[test]
fn a_value_of_another_shape_is_an_error() {
	fails_with("add_pair([a, b]) := a + b; add_pair([1, 2, 3])", "no case");
}
