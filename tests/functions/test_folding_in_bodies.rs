// Precomputed paths (wiki/charged.md §3, package 3): a pure call with constant arguments inside a function body or a
// loop is computed once at compile time, not at every call; the program's value is unchanged
#[cfg(feature = "native")]
use crate::eq;
use crate::is;
#[cfg(feature = "native")]
use warp::*;

const FIB: &str = "def fib(n): n<2 ? n : fib(n-1)+fib(n-2); ";

#[cfg(feature = "native")]
fn compiled(code: &str) -> Node {
	let module = warp::pipeline::compile(code).expect("compiles");
	warp::run::wasm_reader::read_bytes(&module.bytes).expect("runs")
}

#[test]
fn a_constant_call_in_a_body_is_folded() {
	let code = format!("{FIB}def g(x): x + fib(20); g(1) + g(2)");
	is!(&code, 13533);
	#[cfg(feature = "native")]
	eq!(compiled(&code), 13533);
}

#[test]
fn a_constant_call_in_a_loop_is_folded() {
	let code = format!("{FIB}s = 0; for i in 1 to 3 {{ s += fib(15) * i }}; s");
	is!(&code, 3660);
}
