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

#[test] // card generic-signatures (decision compiler-typed): a type variable is any value, a class of its name is the class
fn a_type_variable_of_a_generic_signature_takes_any_value() {
	crate::is!("id(x: T): T := x; \"${id(3)}\" + id(\"a\")", "3a");
	crate::is!("first_of(xs: list of T): T := xs#1; first_of([1 2]) + 1", 2);
	crate::is!("first_of(xs: [T]): T := xs#1; first_of([\"a\"]) + \"b\"", "ab");
	crate::is!("pick(m: map of K, k: K): any := m[k]; pick({a: 1}, \"a\")", 1);
	crate::is!("class T { a: int }; f(x: T): int := x.a; f(T(a: 3))", 3);
	crate::common::fails_with("f(x: Foo) := 1; f(2)", "unknown type Foo");
}

#[test] // card rust-lane-int: a variable declared type holds the type value as any does
fn a_variable_declared_type_holds_a_type() {
	is!("t: type = int; t == int", true);
	is!("t: Type = text; t == float", false);
}

#[test] // card rust-lane-int: a type value known only at run time is the type it names, as `x = float; x is float`
fn a_type_value_parameter_tests_as_its_type() {
	is!("f(t: type) := t is float; f(float)", true);
	is!("f(t: type) := t is number; f(int)", true);
	is!("f(t) := t is int; f(float)", false);
	is!("f(x) := x is int; f(3)", true);
	is!("name(t: type): text := match t { int: \"whole\"; float: \"real\"; _: \"other\" }; name(float)", "real");
}

#[test] // card rust-lane-int: a type test of a run-time value compares type values, never a word the program defines
fn a_type_test_beside_a_function_named_like_a_type() {
	is!("operation(a, b, c, d) := 1; f(x) := x is key; f(3) or f(key)", true);
	is!("pair(a, b) := 1; f(t: type) := t is key; f(int)", false);
}
