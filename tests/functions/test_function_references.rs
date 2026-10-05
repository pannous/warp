//! P82 (user 2026-10-05): a function reference is explicit, `function add` or `&add` (wiki/function-pointer.md); a bare
//! name that needs arguments is "add needs 1 argument" everywhere, a body's last value too, fix: function add
use crate::common::fails_with;
use warp::is;

#[test]
fn test_a_body_returns_a_function_reference() {
	is!("def add(t){ t + \"!\" }; def make(){ function add }; a = make(); a(\"x\")", "x!");
	is!("def add(t){ t + \"!\" }; def make(){ &add }; a = make(); a(\"x\")", "x!");
	is!("def make(){ def add(t){ t + \"!\" }; function add }; a = make(); a(\"x\")", "x!");
}

#[test]
fn test_a_function_reference_as_a_value_and_an_argument() {
	is!("def add(t){ t + \"!\" }; g = function add; g(\"x\")", "x!");
	is!("def sq(x){x*x}; def apply(f, x){ f(x) }; apply(function sq, 3)", 9);
	is!("function g(x) := x*2; g(3)", 6);
}

#[test]
fn test_a_bare_name_needing_arguments_is_an_error_with_the_fix() {
	fails_with("def add(t){ t + \"!\" }; def make(){ add }; make()", "add needs 1 argument");
	fails_with("def make(){ def add(t){ t + \"!\" }; add }; a = make(); a(\"x\")", "fix: function add");
}
