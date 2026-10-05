//! Functions broadcast over lists and pairs (wiki/broadcasting.md, notes/broadcasting.md): a function of one scalar
//! parameter applied to a list is applied to each element, to each value of a pair list. Operators do not broadcast.
use warp::wasm_emitter::eval;
use crate::common::fails_with;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn test_function_broadcasts_over_a_list() {
	assert_eq!(printed("square:=it*it; square [1 2 3]"), "[1 4 9]");
	assert_eq!(printed("square(x):=x*x; square([1 2 3])"), "[1 4 9]");
	assert_eq!(printed("square:=it*it; xs=[1 2 3]; square xs"), "[1 4 9]");
}

#[test]
fn test_function_broadcasts_over_pairs_keeping_keys() {
	assert_eq!(printed("square:=it*it; square [a:1 b:2]"), "[a:1 b:4]");
	assert_eq!(printed("square:=it*it; square [a:1 b:2 c:3]"), "[a:1 b:4 c:9]");
}

#[test]
fn test_broadcasting_reaches_nested_lists() {
	assert_eq!(printed("square:=it*it; square [[1 2] [3]]"), "[[1 4] [9]]");
}

#[test]
fn test_scalar_calls_and_list_functions_do_not_broadcast() {
	assert_eq!(printed("square:=it*it; square 3"), "9");
	assert_eq!(printed("size(xs):=count(xs); size [1 2 3]"), "3");
	assert_eq!(printed("second(xs):=xs#2; second [5 6 7]"), "6");
	assert_eq!(printed("inc(x:int):=x+1; inc [1 2]"), "[2 3]"); // declared parameters broadcast (P50)
}

#[test]
fn test_operators_do_not_broadcast() {
	fails_with("[1 2 3]*2", "multiply each element");
}
