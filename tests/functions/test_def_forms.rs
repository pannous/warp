//! Every function definition form defines the same function (condensed from probe_def_syntax.rs and
//! tests/probes/probe_function_def.rs; the add/test2/$0 cases are in test_functions.rs and test_interpolation.rs)
use crate::is;

#[test]
fn probe_def_block_syntax() {
	is!("def test1(x){x+1}; test1(3)", 4);
}

#[test]
fn definition_forms_agree() {
	is!("def fib(n){n+1}; fib(3)", 4);
	is!("def fib(n): n+1; fib(3)", 4);
	is!("fib(n) = n + 1; fib(3)", 4);
	is!("fib(n:int) = n + 1; fib(3)", 4);
	is!("fib := it + 1; fib(3)", 4);
}

#[test]
fn a_def_function_calls_its_function_parameter() {
	crate::is!("def app(f, x) { return f(x) }; app(y => y+1, 2)", 3);
	crate::is!("def app(f, x) { f(x) }; g(y):=y*3; app(g, 2)", 6);
}

#[test]
fn a_parameter_tested_for_membership_is_no_function() {
	crate::is!("snake=[1]; f(p) := not (p in snake); f(3)", 1);
	crate::is!("snake=[(1,2)]; def is_free(position) { return not (position in snake) }; is_free((1,2))", 0);
}
