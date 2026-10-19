// Memoization (wiki/charged.md §3, P72: the compiler's decision alone): a pure recursive function of one Int whose calls
// overlap (fib-shaped: it calls itself more than once) caches its results for small arguments; same values, no blow-up
use crate::is;
use std::time::{Duration, Instant};

const FAST: Duration = Duration::from_secs(3);

#[test]
fn fib_is_memoized() {
	let started = Instant::now();
	is!("def fib(n): n<2 ? n : fib(n-1)+fib(n-2); fib(40)", 102334155);
	assert!(started.elapsed() < FAST, "fib(40) took {:?}", started.elapsed());
	is!("def fib(n): n<2 ? n : fib(n-1)+fib(n-2); fib(10) + fib(-3)", 52); // outside the cache: computed as before
}
