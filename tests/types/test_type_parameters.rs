//! A parameter of type type takes the type itself, never its constructor's call (card random-should, decision
//! random-forms): `f(t: type) := t == int; f int`
use crate::is;

#[test]
fn a_type_parameter_takes_a_bare_type_word() {
	is!("f(t: type) := t == int; f int", true);
	is!("f(t: type) := t == int; f(float)", false);
	is!("f(t: Type) := t; f(text)", "text");
	is!("def g(t: type, n) { t == int and n > 1 }; g(int, 2)", true);
}

#[test]
fn a_type_parameter_takes_a_class() {
	is!("Circle{r:1}; f(t: type) := t == Circle; f Circle", true);
}

#[test]
fn a_value_for_a_type_parameter_is_an_error() {
	crate::common::fails_with("f(t: type) := t; f 3", "takes a type");
	crate::common::fails_with("f(t: type) := t; f \"x\"", "takes a type");
}
