//! P50 (user, 2026-10-05): a function whose one parameter is declared with a scalar type broadcasts too
//! (wiki/broadcasting.md `square number = number*number; square [1 2 3] == [1 4 9]`).
use crate::common::fails_with;
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn test_declared_scalar_parameters_broadcast() {
	assert_eq!(printed("square number = number*number; square [1 2 3]"), "[1 4 9]");
	assert_eq!(printed("inc(x:int):=x+1; inc [1 2]"), "[2 3]");
	assert_eq!(printed("half(x:float):=x/2; half [1.0 3.0]"), "[0.5 1.5]");
	assert_eq!(printed("inc(x:int):=x+1; inc [a:1 b:2]"), "[a:2 b:3]");
}

#[test]
fn test_declared_list_parameters_take_the_list() {
	assert_eq!(printed("total(xs:list):=count(xs); total [1 2 3]"), "3");
	fails_with("inc(x:int):=x+1; inc \"abc\"", "inc needs an Int for parameter x");
}

// card typed-list-upper: a variable declared with a list type broadcasts like an untyped list variable
#[test]
fn a_declared_list_variable_broadcasts() {
	assert_eq!(printed("names: texts = [\"hi\"]; upper names"), "[\"HI\"]");
	assert_eq!(printed("names: texts = [\"hi\", \"yo\"]; upper names"), "[\"HI\" \"YO\"]");
	assert_eq!(printed("names: list of text = [\"hi\"]; upper names"), "[\"HI\"]");
	assert_eq!(printed("xs: ints = [1 2]; inc(x:int):=x+1; inc xs"), "[2 3]");
}

// card method-broadcast (default, question queued with the user): the method form broadcasts like the prefix form
#[test]
fn a_method_of_a_list_broadcasts() {
	assert_eq!(printed("names = [\"hi\", \"yo\"]; names.upper"), "[\"HI\" \"YO\"]");
	assert_eq!(printed("names: texts = [\"hi\"]; names.upper()"), "[\"HI\"]");
	assert_eq!(printed("[1.5 2.5].floor"), "[1 2]");
	assert_eq!(printed("inc(x:int):=x+1; xs = [1 2]; xs.inc"), "[2 3]");
	assert_eq!(printed("\"hi\".upper"), "\"HI\"");
}
