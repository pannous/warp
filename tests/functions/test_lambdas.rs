//! Compile-time lambdas: `f = x=>x*x` defines a function at that point (capture by value), a block with bindings is called
//! at once, `map` over a literal block or lambda is a loop. A lambda that cannot be inlined is a closure (test_closures.rs).
use crate::is;
use warp::wasm_emitter::eval;
use crate::common::fails_with;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn test_assigned_lambda_is_a_function() {
	is!("sq = x=>x*x; sq 3", 9);
	is!("sq = x->x*x; sq(4)", 16);
	is!("add = (x y)->x+y; add 2 3", 5);
	is!("add = (x, y) => x+y; add(2, 3)", 5);
	is!("add = (x y)=>x+y; add(1, 1)", 2);
}

#[test]
fn test_lambda_captures_by_value() {
	// late binding (wiki/charged.md §3, released 2026-10-05) replaced D7's silent snapshot: the change is an error
	fails_with("n=10; f = x=>x+n; n=20; f 1", "f reads n (line 1): declare `global n` in f");
	is!("n=10; f = x=>x+n; f 1", 11);
}

#[test]
fn test_block_called_with_bindings() {
	is!("{x*x}(x=5)", 25);
	is!("{x+y}(x=1 y=2)", 3);
	is!("{x+y}(x=1, y=2)", 3);
}

#[test]
fn test_map_over_a_block_or_lambda() {
	assert_eq!(printed("map [1 2 3] {it*it}"), "[1 4 9]");
	assert_eq!(printed("map [1 2 3] (x=>x+1)"), "[2 3 4]");
	assert_eq!(printed("map([1 2 3], x=>x*2)"), "[2 4 6]");
	assert_eq!(printed("xs=[1 2 3]; map xs {it+10}"), "[11 12 13]");
	assert_eq!(printed("twice(x):=x*2; map [1 2 3] twice"), "[2 4 6]");
	assert_eq!(printed("k=3; map [1 2] (x=>x*k)"), "[3 6]");
}

#[test]
fn test_map_as_a_method() {
	assert_eq!(printed("[1 2 3].map {it*it}"), "[1 4 9]");
	assert_eq!(printed("xs=[1 2 3]; xs.map(x=>x-1)"), "[0 1 2]");
}

#[test]
fn test_a_lambda_that_cannot_be_inlined_is_a_closure_value() {
	is!("apply(f, x):=x; apply(y=>y+1, 2)", 2);
	fails_with("map [1 2 3] 5", "map needs a function, got 5 (an Int); fix: map [1 2 3] (x => …)");
}

#[test]
fn test_named_functions_and_data_are_untouched() {
	is!("f(x):=x*2; f 4", 8);
	is!("p={a:1}; p.a", 1);
	assert_eq!(printed("{a:1 b:2}"), "{a:1 b:2}");
}
