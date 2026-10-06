//! `filter`, `reduce`, `fold` and `each` over a literal block, lambda or defined function: a loop, lowered at compile time
use crate::is;
use warp::wasm_emitter::eval;
use crate::common::fails_with;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn test_filter() {
	assert_eq!(printed("filter [1 2 3 4] {it>2}"), "[3 4]");
	assert_eq!(printed("[1 2 3].filter(x=>x%2==1)"), "[1 3]");
	assert_eq!(printed("filter([1 2 3 4], x=>x>1)"), "[2 3 4]");
	assert_eq!(printed("big(x):=x>2; filter [1 2 3 4] big"), "[3 4]");
	assert_eq!(printed("xs=[1 2 3 4]; k=3; xs.filter {it < k}"), "[1 2]");
	assert_eq!(printed("filter [1 2 3] {it>5}"), "ø");
}

#[test]
fn test_reduce() {
	is!("reduce [1 2 3] (a b)->a+b", 6);
	is!("reduce([1 2 3 4], (a, b) => a*b)", 24);
	is!("[5 3 8].reduce((a b)->a+b)", 16);
	is!("add(a,b):=a+b; reduce [1 2 3] add", 6);
	is!("xs=[7]; reduce xs (a b)->a+b", 7);
}

#[test]
fn test_reduce_of_an_empty_list_is_an_error() {
	fails_with("xs=[]; reduce xs (a b)->a+b", "reduce of an empty list");
}

#[test]
fn test_fold() {
	is!("fold [1 2 3] 10 (a b)->a+b", 16);
	is!("fold([1 2 3], 0, (a, b) => a+b*b)", 14);
	is!("[1 2 3].fold(100, (a b)->a-b)", 94);
	is!("xs=[]; fold xs 5 (a b)->a+b", 5);
}

#[test]
fn test_each_runs_the_body_per_item_and_is_the_last_value() {
	is!("each [1 2 3] {it*10}", 30);
	is!("total=0; each [1 2 3] {total=total+it}; total", 6);
	is!("[1 2 3].each(x=>x+1)", 4);
}

#[test]
fn test_a_wrong_function_is_an_error() {
	fails_with("reduce [1 2 3] {it+1}", "reduce takes a function of two arguments");
	fails_with("filter [1 2 3] 5", "filter needs a function, got 5");
	fails_with("fold [1 2 3] 0 (a)->a", "fold takes a function of two arguments");
}

#[test]
fn test_words_a_program_defines_win() {
	is!("filter(xs, f):=42; filter([1 2], 3)", 42);
}
