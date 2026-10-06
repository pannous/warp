// Tuple returns (decision: notes/open_decisions.md "Tuple returns"): `return a, b` leaves both values on the wasm
// stack (multi-value), `x, y = f()` binds them without allocating a list (notes/multi_value.md)
use crate::common::fails_with;
use crate::functions::test_multi_value::result_count;
use warp::wasm_emitter::compile;
use crate::is;

#[test]
fn test_two_returned_values_destructure() {
	is!("f():=return 1, 2; x, y = f(); x+y", 3);
	is!("divmod(a, b) := return a//b, a%b; q, r = divmod(17, 5); q*10+r", 32);
	is!("half(x) := return x, x/2.5; a, b = half(5); b", 2.0);
	is!("named() := return \"a\", 2; s, n = named(); s", "a");
}

#[test]
fn test_tuple_function_returns_wasm_multi_value() {
	let module = compile("divmod(a, b) := return a//b, a%b; q, r = divmod(17, 5); q").unwrap_or_else(|error| panic!("{error:?}"));
	assert_eq!(result_count(&module.bytes, "divmod"), Some(2));
}

#[test]
fn test_tuple_function_as_whole_value_is_a_list() {
	is!("f():=return 1, 2; f() == [1 2]", 1);
	is!("f():=return 1, 2; f()#2", 2);
}

#[test]
fn test_destructuring_arity_mismatch_is_loud() {
	fails_with("f():=return 1, 2; x, y, z = f()", "f returns 2 values, not 3");
	fails_with("x, y = 1, 2, 3", "3 values for 2 names");
	fails_with("f(c) := { if c { return 1 }; return 1, 2 }; f(1)", "f returns 2 values");
}

#[test]
fn test_parallel_assignment_and_swap() {
	is!("x, y = 1, 2; x*10+y", 12);
	is!("x=1; y=2; x, y = y, x; x*10+y", 21);
}

#[test]
fn test_bare_list_assignment_still_asks() {
	fails_with("a=1,2,3; a", "list or separate statements");
}

#[test]
fn test_tuple_returns_inside_functions() {
	is!("divmod(a, b) := return a//b, a%b; digits(n) := { q, r = divmod(n, 10); q + r }; digits(47)", 11);
	is!("pair(n) := { if n > 0 { return n, 1 }; return 0, 0 }; a, b = pair(5); a+b", 6);
}

#[test]
fn test_recursive_tuple_function() {
	is!("fibpair(n) := { if n == 0 { return 0, 1 }; a, b = fibpair(n-1); return b, a+b }; x, y = fibpair(10); x", 55);
	let module = compile("fibpair(n) := { if n == 0 { return 0, 1 }; a, b = fibpair(n-1); return b, a+b }; fibpair(3)").unwrap_or_else(|error| panic!("{error:?}"));
	assert_eq!(result_count(&module.bytes, "fibpair"), Some(2));
	assert_eq!(result_count(&module.bytes, "fibpair$list"), Some(1));
}
