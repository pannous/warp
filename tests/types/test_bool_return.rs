//! Card bool-return: a function whose every result is a bool returns one, a literal `true` too and through its own
//! recursion; a function giving numbers stays an int
use warp::wasm_emitter::eval;

fn text_of(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_function_giving_a_bool_returns_a_bool() {
	assert_eq!(text_of("h() := true\nh()"), "yes");
	assert_eq!(text_of("h() := false\nh()"), "no");
	assert_eq!(text_of("f(n) := if n == 0 then true else f(n - 1)\nf(2)"), "yes");
	assert_eq!(text_of("f(n) := n == 0 ? false : f(n - 1)\nf(2)"), "no");
	assert_eq!(text_of("g(n) := if n == 0 then 5 else g(n - 1)\ng(2)"), "5");
	assert_eq!(text_of("fib(n) := if n < 2 then n else fib(n-1) + fib(n-2)\nfib(10)"), "55");
}
