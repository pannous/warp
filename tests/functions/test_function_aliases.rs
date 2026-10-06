//! P83 (user 2026-10-05): an alias names the function explicitly, `g = function add` or `g = &add`; a bare `g = add`
//! is P82's "add needs 1 argument" with the fix; a function-taking parameter keeps the bare argument `apply(add, 3)`
use crate::common::fails_with;
use crate::is;

#[test]
fn test_an_alias_names_the_function() {
	is!("def add(t){ t + \"!\" }; g = function add; g(\"x\")", "x!");
	is!("def add(t){ t + \"!\" }; g = &add; h = g; h(\"x\")", "x!");
}

#[test]
fn test_a_bare_alias_is_the_error_with_the_fix() {
	fails_with("def add(t){ t + \"!\" }; g = add; g(\"x\")", "fix: function add");
}

#[test]
fn test_a_function_argument_stays_bare() {
	is!("def sq(x){x*x}; def apply(f, x){ f(x) }; apply(sq, 3)", 9);
}
