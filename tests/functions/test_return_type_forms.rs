// The result type as Python, Swift, TypeScript, Kotlin and Rust write it, before a block body: `-> int { … }`,
// `: number { … }`, `-> int: body`; the result converts to it, or is checked, as `def f(x) as int = …` (test_typed_returns)
use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn an_arrow_result_type_before_a_block_body() {
	is!("def f(x: int) -> int { x + 1 }; f(1)", 2); // Python with braces, Rust fn f(x: i32) -> i32 { x + 1 }
	is!("func f(x: Int) -> Int { return x + 1 }; f(1)", 2); // Swift
	crate::common::fails_with("def half(x) -> int { x / 2 }; half(3)", "an int must be a whole number"); // checked, not truncated (card lossy-return)
	is!("def half(x) -> float { x / 2 }; half(3)", 1.5);
}

#[test]
fn a_colon_result_type_before_a_block_body() {
	is!("function f(x: number): number { return x + 1 }; f(1)", 2); // TypeScript
	is!("fun f(x: Int): Int { return x + 1 }; f(1)", 2); // Kotlin
}

#[test]
fn an_arrow_result_type_before_a_colon_body() {
	is!("def f(x: int) -> int: x + 1; f(1)", 2); // Python
}

#[test]
fn named_arguments_reach_a_function_with_a_result_type() {
	is!("def f(x: String) -> String { \"Hi \" + x }; f(x: \"Bob\")", "Hi Bob"); // was a cast failure
	is!("def f(a, b=2) -> int { a * b }; f(3, b: 5)", 15);
	is!("f(a, b) : int := a - b; f(b=1, a=5)", 4);
}

#[test]
fn a_tuple_result_type() {
	assert_eq!(
		eval("func swapped(_ a: Int, _ b: Int) -> (Int, Int) { return (b, a) }; swapped(1, 2)").serialize(),
		"(2 1)"
	);
	assert_eq!(eval("def f(a) -> (int, int) { return (a, a) }; f(1)").serialize(), "(1 1)");
}
