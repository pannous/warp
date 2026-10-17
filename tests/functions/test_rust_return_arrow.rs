//! card rust-fn-arrow: Rust's `fn f(x: i32) -> i32 { … }` reads as the function, the type after `->` its result type
//! (decision rust-forms-welcome; the warp form stays `f(x:int) := …`). The card's `fn double` fails for its name, not
//! the arrow: a function named like the type word double is refused (decision P20)
use crate::is;

#[test]
fn a_rust_function_with_a_return_type_runs() {
	is!("fn square(x: i32) -> i32 { x * x } square(3)", 9);
	is!("fn square(x: i32) -> i32 { return x * x; } square(3)", 9);
	is!("fn add(a: i32, b: i32) -> i32 { a + b } add(2, 3)", 5);
	is!("fn half(x: f64) -> f64 { x / 2.0 } half(3.0)", 1.5);
}

#[test]
fn a_function_named_like_a_type_word_is_refused() {
	crate::common::fails_with(
		"fn double(x: i32) -> i32 { x * 2 } double(3)",
		"double is a type; name your function to_double",
	);
}
