//! Every function definition form defines the same function (condensed from probe_def_syntax.rs and
//! tests/probes/probe_function_def.rs; the add/test2/$0 cases are in test_functions.rs and test_interpolation.rs)
use warp::is;

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
