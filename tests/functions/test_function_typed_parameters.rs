//! card parameter-typed: a parameter typed as a function, `f: (float) -> float`, gives a function passed for it its
//! parameter types and casts its value to the result type (src/lowering/types/function_types.rs); a parameter passed on
//! to one is typed the same; the call's other arguments still take quantities
use crate::is;

const APPLY: &str = "apply(f: (float) -> float, x: float) := f(x)\n";

#[test]
fn a_function_passed_for_a_function_typed_parameter_takes_its_types() {
	is!(&format!("{APPLY}half(t) := t / 2\napply(half, 3.0)"), 1.5);
	is!(&format!("{APPLY}whole(t) := 3\napply(whole, 1.0) is float"), true);
}

#[test]
fn a_parameter_passed_on_to_a_function_typed_one_types_the_function() {
	is!(&format!("{APPLY}outer(g, x) := apply(g, x)\nwhole(t) := 3\nouter(whole, 1.0) is float"), true);
}

#[test]
fn a_function_typed_parameter_leaves_quantities_to_the_others() {
	is!("apply(f: (float) -> float, d) := d * 2\ng(t) := t\napply(g, 0.1s) == 0.2s", true);
}

#[test]
fn the_name_of_a_parameter_declared_function_is_the_function_word() {
	is!("use sound\nd() := 1\ng(c: function) := c.name\ng(d)", "d");
}
