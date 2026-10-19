// Memoization (wiki/charged.md §3, P72: the compiler's decision alone): a pure recursive function of one Int whose calls
// overlap (fib-shaped: it calls itself more than once) caches its results for small arguments; same values, no blow-up
use crate::is;
use crate::parser::test_big_literals::thread_cpu_time;
use std::time::Duration;

const FAST: Duration = Duration::from_secs(3);

#[test]
fn fib_is_memoized() {
	// CPU time of this thread: machine load stretches wall-clock time, not this (card memo-wall-clock)
	let started = thread_cpu_time();
	is!("def fib(n): n<2 ? n : fib(n-1)+fib(n-2); fib(40)", 102334155);
	let spent = thread_cpu_time() - started;
	assert!(spent < FAST, "fib(40) took {spent:?} of CPU time");
	is!("def fib(n): n<2 ? n : fib(n-1)+fib(n-2); fib(10) + fib(-3)", 52); // outside the cache: computed as before
}
