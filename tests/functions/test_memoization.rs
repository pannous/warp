// Memoization (wiki/charged.md §3, P72: the compiler's decision alone): a pure recursive function of one Int whose calls
// overlap (fib-shaped: it calls itself more than once) caches its results for small arguments; same values, no blow-up
use crate::is;
use std::time::{Duration, Instant};

const FAST: Duration = Duration::from_secs(3);

fn lowered(code: &str) -> String {
	warp::pipeline::lower(code).expect("a program that needs a module").serialize()
}

#[test]
fn fib_is_memoized() {
	let started = Instant::now();
	is!("def fib(n): n<2 ? n : fib(n-1)+fib(n-2); fib(40)", 102334155);
	assert!(started.elapsed() < FAST, "fib(40) took {:?}", started.elapsed());
	is!("def fib(n): n<2 ? n : fib(n-1)+fib(n-2); fib(10) + fib(-3)", 52); // outside the cache: computed as before
}

#[test]
fn only_overlapping_recursion_is_memoized() {
	assert!(!lowered("sq(n) := n*n; sq(5)").contains("memo_"), "a non-recursive function gets no cache");
	assert!(!lowered("fact(n) := n<2 ? 1 : n*fact(n-1); fact(5)").contains("memo_"), "one recursive call: nothing to share");
	assert!(lowered("def fib(n): n<2 ? n : fib(n-1)+fib(n-2); fib(20)").contains("memo_"));
	assert!(!lowered("def f(n): n<2 ? n : f(n-1)+f(n-2)+random_below(2); f(5)").contains("memo_"), "effects: never cached");
}
