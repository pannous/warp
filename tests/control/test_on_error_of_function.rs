// Card g_ADRM (wiki/Error.md): `on error of f {handler}` attaches a catch handler to f from outside: a call of f that
// fails has the handler's value, as `try body else handler`
use crate::is;
use crate::common::fails_with;

#[test]
fn a_failing_call_has_the_handler_value() {
	is!("f(x) := 10/x; on error of f { 7 }; f(0) + f(5)", 9);
	is!("f(x) := { raise oops }; on error of f { 9 }; f(1)", 9);
}

#[test]
fn every_definition_and_handler_form() {
	is!("fun f(x) { 10/x }; on error of f: -1; f(0)", -1);
	is!("def f(x): 10/x; on error of f { 0 }; f(0)", 0);
	is!("function f(x) { return 10/x }; on error of f { 4 }; f(0) + f(2)", 9);
	is!("f(x) := 10/x; on error of f do 3; f(0)", 3);
	is!("on error of f { 0 }; f(x) := 10/x; f(0)", 0);
}

#[test]
fn an_unknown_function_is_named() {
	fails_with("on error of g { 0 }; 1", "defines no function g");
}
