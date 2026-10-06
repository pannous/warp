// The result type as Python, Swift, TypeScript, Kotlin and Rust write it, before a block body: `-> int { … }`,
// `: number { … }`, `-> int: body`; the result converts to it as `def f(x) as int = …` does (test_typed_returns)
use crate::is;

#[test]
fn an_arrow_result_type_before_a_block_body() {
	is!("def f(x: int) -> int { x + 1 }; f(1)", 2); // Python with braces, Rust fn f(x: i32) -> i32 { x + 1 }
	is!("func f(x: Int) -> Int { return x + 1 }; f(1)", 2); // Swift
	is!("def half(x) -> int { x / 2 }; half(3)", 1); // the result converts
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
