//! First-class functions, resolved at compile time: a named function, `&name`, an alias, an operator or a lambda that captures
//! nothing can be passed where a function is expected; the function it is passed to is specialised for it
use warp::wasm_emitter::eval;
use warp::*;
mod common;
use common::fails_with;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn test_a_parameter_called_as_a_function() {
	is!("double2(x):=x*2; apply(f, x):=f(x); apply(double2, 3)", 6);
	is!("inc(x):=x+1; twice(f, x):=f(f(x)); twice(inc, 5)", 7);
	is!("sq(x):=x*x; apply(f, x):=f(x); apply(sq, 4) + apply(sq, 2)", 20);
}

#[test]
fn test_an_explicit_reference() {
	is!("double2(x):=x*2; apply(f, x):=f(x); apply(&double2, 3)", 6);
	assert_eq!(printed("square(x):=x*x; map &square [1 2 3]"), "[1 4 9]");
}

#[test]
fn test_an_alias_of_a_function() {
	is!("double2(x):=x*2; g=double2; g 4", 8);
	is!("double2(x):=x*2; g=&double2; g(5)", 10);
}

#[test]
fn test_map_with_on() {
	assert_eq!(printed("square(x):=x*x; map square on [1 2 3]"), "[1 4 9]");
	assert_eq!(printed("square(x):=x*x; map &square on [1 2 3]"), "[1 4 9]");
}

#[test]
fn test_operators_as_values() {
	is!("fold [1 2 3] 0 +", 6);
	is!("fold [2 3 4] 1 *", 24);
	is!("reduce [1 2 3 4] +", 10);
	is!("xs=[5 6]; reduce xs *", 30);
}

#[test]
fn test_partial_application_of_an_iteration_word() {
	is!("sum := fold +; sum [1 2 3]", 6);
	is!("product := fold *; product [2 3 4]", 24);
	is!("total := reduce +; total [4 5]", 9);
}

#[test]
fn test_a_lambda_that_captures_nothing_is_a_value() {
	is!("apply(f, x):=f(x); apply(y=>y+1, 2)", 3);
	is!("apply(f, x):=f(x); apply((a)->a*a, 5)", 25);
}

#[test]
fn test_a_lambda_that_captures_stays_an_error() {
	fails_with("k=3; apply(f, x):=f(x); apply(y=>y+k, 2)", "functions are not first-class values yet");
}

#[test]
fn test_a_function_argument_of_a_specialised_function_is_passed_on() {
	is!("dbl(x):=x*2; apply(f, x):=f(x); outer(g, x):=apply(g, x)+1; outer(dbl, 4)", 9);
	assert_eq!(printed("dbl(x):=x*2; run(f, xs):=map xs f; run(dbl, [1 2])"), "[2 4]");
}

#[test]
fn test_ordinary_calls_are_untouched() {
	is!("f(x):=x*2; f 4", 8);
	is!("add(a,b):=a+b; add(2,3)", 5);
}
