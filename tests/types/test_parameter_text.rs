// Card param-text: a parameter typed by a class with a text() method prints by it, as a variable of that class does
// (found in units-dynamic: lib/units.warp worked around it with quantity_text)
use crate::is;

const V: &str = "class V{x:int; text() := \"v\" + str(x)}\n";

#[test]
fn a_typed_parameter_prints_by_its_text_method() {
	is!(&format!("{V}f(q:V) := str(q)\nf(V(1))"), "v1");
	is!(&format!("{V}f(q:V) := q.text()\nf(V(2))"), "v2");
	is!(&format!("{V}f(q:V) := q as text\nf(V(3))"), "v3");
	is!(&format!("{V}f(q:V) := \"is \\(q)\"\nf(V(4))"), "is v4");
	is!(&format!("{V}f(n, q:V) := str(n) + str(q)\nf(5, V(6))"), "5v6");
}

#[test]
fn a_typed_parameter_types_only_its_own_function() {
	is!(&format!("{V}f(q:V) := str(q)\ng(q) := str(q)\ng(3) + f(V(1))"), "3v1");
	is!(&format!("{V}q = V(1)\ng(q) := str(q)\ng(3)"), "3");
}
