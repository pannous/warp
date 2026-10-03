//! Real closures: a lambda or function is a value (a GC struct holding a typed function reference and its captured values, captured
//! by value) that can be stored, passed, returned and called later. Compile-time specialisation stays where the function is known.
use warp::wasm_emitter::eval;
use warp::*;
use crate::common::fails_with;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn test_lambda_in_a_variable() {
	is!("f = x => x*2; f(3)", 6);
}

#[test]
fn test_returned_closure() {
	is!("make_adder(n) := (x => x+n); add2 = make_adder(2); add2(5)", 7);
	is!("make_adder(n) := (x => x+n); add2 = make_adder(2); add3 = make_adder(3); add2(5) + add3(5)", 15);
	is!("make_adder(n) := x => x+n; make_adder(10)(1)", 11);
}

#[test]
fn test_closure_captures_by_value() {
	is!("k=3; f = make(k); make(n) := (x => x*n); k=100; f(2)", 6);
	is!("mul(n) := (x => x*n); k=3; g = mul(k); k=5; g(2)", 6);
}

#[test]
fn test_capturing_lambda_passed_to_a_function() {
	is!("apply(f, x):=f(x); k=4; apply(y=>y*k, 3)", 12);
	is!("twice(f, x):=f(f(x)); n=1; twice(y=>y+n, 5)", 7);
}

#[test]
fn test_closure_held_in_a_variable_passed_on() {
	is!("apply(f, x):=f(x); make_adder(n) := (x => x+n); a = make_adder(4); apply(a, 1)", 5);
}

#[test]
fn test_map_and_reduce_with_a_closure_variable() {
	assert_eq!(printed("make_adder(n) := (x => x+n); a = make_adder(10); map [1 2 3] a"), "[11 12 13]");
	assert_eq!(printed("k=2; scale = make(k); make(n) := (x => x*n); [1 2 3].map(scale)"), "[2 4 6]");
	is!("sum_with(f, xs) := reduce xs f; k=0; sum_with((a b)->a+b+k, [1 2 3])", 6);
}

#[test]
fn test_function_chosen_at_run_time() {
	is!("double(x):=x*2; square(x):=x*x; pick(c) := if c then double else square; f = pick(1); f(5)", 10);
	is!("double(x):=x*2; square(x):=x*x; pick(c) := if c then double else square; f = pick(0); f(5)", 25);
}

#[test]
fn test_closures_returning_text() {
	is!("greeter(g) := (name => g + \" \" + name); hi = greeter(\"hi\"); hi(\"bob\")", "hi bob");
}

#[test]
fn test_not_a_function_stays_loud() {
	fails_with("map [1 2 3] 5", "functions are not first-class values yet");
}
