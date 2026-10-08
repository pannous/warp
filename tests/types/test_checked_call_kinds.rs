// The type model W0 (notes/type_theory.md) types a call f(e) only when e fits f's declared parameter, and a variable
// keeps the kind of its first value, a call's result included: warp refuses both at compile time
use crate::is;
use crate::common::fails_with;

#[test]
fn a_declared_parameter_refuses_an_argument_of_another_kind() {
	fails_with("f(x: text) := x; f(3)", "f needs a Text for parameter x, got 3 (an Int)");
	fails_with("f(x: int) := x; f(\"ab\")", "f needs an Int for parameter x");
	fails_with("f(x: text) := x; n = 3; f(n)", "f needs a Text for parameter x");
}

#[test]
fn a_declared_parameter_takes_its_subtypes() {
	is!("f(x: text) := x; f(\"a\")", "a");
	is!("f(x: number) := x * 2; f(3)", 6);
	is!("f(x: float) := x; f(2)", 2.0);
	is!("f(x: int) := x + 1; f(true)", 2);
	is!("f(x: any) := x; f(\"a\")", "a");
	// a list broadcasts over a scalar parameter (functions/test_broadcasting.rs)
	is!("f(x: int) := x + 1; f([1])#1", 2);
}

#[test]
fn a_variable_keeps_the_kind_of_a_call_result() {
	fails_with("f(x: int) := x + 1; y = f(2); y = \"a\"", "y was an Int, is given a Text");
	fails_with("f() := \"a\"; y = f(); y = 2", "y was a Text, is given an Int");
	is!("f(x: int) := x + 1; y = f(2); y = 7; y", 7);
	is!("f(x: int) := x + 1; y = f(2); y = 1.5; y", 1.5);
}
