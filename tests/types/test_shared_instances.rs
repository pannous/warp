//! An instance passed to a function is shared (P200, card instance-field): a field the function changes is changed for
//! the caller, as in Python or JavaScript. Giving the parameter a new value stays inside the function.
use crate::is;

const POINT: &str = "class Point { x: int }; ";

#[test]
fn a_function_changes_a_field_of_the_instance_it_is_given() {
	is!(&format!("{POINT}f(q: Point) := q.x = 7; p = Point(1); f(p); p.x"), 7);
	is!(&format!("{POINT}f(q: Point) := q.x += 2; p = Point(1); f(p); f(p); p.x"), 5);
	is!("class bag { items: texts }; put(x, v) := x.items.add(v); b = bag([\"hi\"]); put(b, \"yo\"); #b.items", 2);
}

#[test]
fn the_function_still_gives_its_value() {
	is!(&format!("{POINT}f(q: Point) := {{ q.x = 7; 3 }}; p = Point(1); y = f(p); y + p.x"), 10);
	is!(&format!("{POINT}f(q: Point) := {{ q.x = 7; return 3 }}; p = Point(1); y = f(p); y + p.x"), 10);
	is!(&format!("{POINT}f(q: Point) := {{ q.x = 7; 3 }}; f(Point(1))"), 3);
}

#[test]
fn a_change_passes_through_other_functions_and_methods() {
	is!(&format!("{POINT}f(q: Point) := q.x = 7; g(r) := f(r); p = Point(1); g(p); p.x"), 7);
	is!("class C { n: int; inc() := n += 1 }; twice(c) := { c.inc(); c.inc() }; k = C(0); twice(k); k.n", 2);
}

#[test]
fn a_new_value_of_the_parameter_stays_inside() {
	is!(&format!("{POINT}f(q: Point) := q = Point(9); p = Point(1); f(p); p.x"), 1);
}
