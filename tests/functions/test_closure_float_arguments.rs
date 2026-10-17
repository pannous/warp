//! card if-list: a closure called with a parameter that its calls make a float (a libm result, a list element) takes a
//! float, as it does for a float literal
use crate::is;

#[test]
fn a_closure_takes_a_float_passed_through_a_parameter() {
	is!("h(x, f) := f(x); h(cos(0.0), a => a * a)", 1.0);
	is!("h(x, f) := f(x); h(sqrt(2.25), a => a + 1)", 2.5);
	is!("g(a) := a * a; h(x, f) := f(x); h(sqrt(2.25), g)", 2.25);
	is!("h(x, f) := f(x); y = sqrt(2.25); h(y, a => a * a)", 2.25);
}

#[test]
fn a_closure_takes_a_float_element_after_narrowing() {
	is!("h(x: any, f) := if x is list then f(x#1) else 0; h([sqrt(2.25), 1], a => a * a)", 2.25);
	is!("h(x: any, f) := if x is list then f(x#1) else 0; h([0.5, 1], a => a * a)", 0.25);
}
