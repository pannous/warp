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

#[test]
fn julia_dot_call_broadcasts() {
	assert_eq!(eval("f(x) = x^2; f.([1,2,3])").serialize(), "[1 4 9]");
	assert_eq!(eval("f(x) = x^2; xs = [1,2]; f.(xs)").serialize(), "[1 4]");
	assert_eq!(eval("add(a, b) = a + b; add.([1,2], 10)").serialize(), "[11 12]");
}

#[test]
fn library_words_and_math_operators_broadcast() {
	assert_eq!(eval("upper [\"ab\", \"cd\"]").serialize(), "[\"AB\" \"CD\"]");
	assert_eq!(eval("abs [-1, 2]").serialize(), "[1 2]"); // was 2, silently
	assert_eq!(eval("sqrt [4, 9]").serialize(), "[2 3]");
	assert_eq!(eval("xs = [-3, 4]; abs xs").serialize(), "[3 4]");
}

#[test]
fn a_broadcast_inside_a_library_word_and_a_lambda_variable() {
	assert_eq!(eval("square(x) := x*x; sum square [1, 2, 3]").serialize(), "14");
	assert_eq!(eval("square(x) := x*x; sum(square [1, 2, 3])").serialize(), "14");
	assert_eq!(eval("f = x => x + 1; f [1, 2]").serialize(), "[2 3]");
}

#[test]
fn max_and_min_without_parentheses() {
	assert_eq!(eval("max [1, 25]").serialize(), "25");
	assert_eq!(eval("xs = [4, 2]; min xs").serialize(), "2");
	assert_eq!(eval("square(x) := x*x; max square [1, -5]").serialize(), "25");
}

#[test]
fn a_function_of_several_values_broadcasts_over_its_list_argument() {
	assert_eq!(eval("add(a, b) := a + b; add [1, 2] 10").serialize(), "[11 12]");
	assert_eq!(eval("add(a, b) := a + b; add(10, [1, 2])").serialize(), "[11 12]");
	assert_eq!(eval("add(a, b) := a + b; add(all [1, 2], 10)").serialize(), "[11 12]");
	assert_eq!(eval("square(x) := x*x; xs = [1, 2]; square(all xs)").serialize(), "[1 4]");
}

#[test] // cards sqrt-square and square-error: a range broadcasts like a list literal
fn a_range_broadcasts_like_a_list() {
	crate::is!("sqrt (1 to 4) == [1, sqrt 2, sqrt 3, 2]", true);
	crate::is!("abs (-2 to 1)", warp::ints(vec![2, 1, 0, 1]));
	crate::is!("xs = 1 to 3; sqrt xs == [1, sqrt 2, sqrt 3]", true);
	crate::is!("use math; square (1 to 3)", warp::ints(vec![1, 4, 9]));
	crate::is!("sq(x) := x*x; xs = 1 to 3; sq xs", warp::ints(vec![1, 4, 9]));
	crate::is!("use math; square all 1 to 5", warp::ints(vec![1, 4, 9, 16, 25]));
	crate::is!("sq(x) := x*x; sq all 1 to 5", warp::ints(vec![1, 4, 9, 16, 25]));
}

// card broadcast-examples (P142): samples/functions.wasp chains and pipes a broadcast instead of a map
#[test]
fn broadcasting_composes_with_chaining_and_pipelines() {
	assert_eq!(eval("square(x) := x*x; numbers = [1, 2, 3, 4, 5]; (square all numbers).filter(x => x > 5).sum()").serialize(), "50");
	assert_eq!(eval("square(x) := x*x; numbers = [1, 2, 3, 4, 5]; (square numbers) |> filter(x => x > 5) |> sum").serialize(), "50");
}
