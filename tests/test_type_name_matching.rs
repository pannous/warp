//! D5 (user, 2026-10-03): matching by type name, "General rule" (wiki/matching.md, type.md; rule in notes/matching.md).
//! Known type words in a definition head name typed parameters.

use warp::is;
use warp::wasm_emitter::eval;

#[test]
fn test_type_then_name_parameter() {
	is!("fib int i = if i<2 : i else fib(i - 1) + fib(i - 2); fib(10)", 55);
	is!("twice int x := x+x; twice 4", 8);
}

#[test]
fn test_type_word_is_the_parameter_name() {
	is!("fibonacci number = if number<2 : number else fibonacci(number - 1) + fibonacci(number - 2); fibonacci 10", 55);
	is!("succ int := int+1; succ 3", 4);
}

#[test]
fn test_lone_type_word_parameter_is_it() {
	is!("foo of int = it + it; foo 3", 6);
	is!("square of a number = it*it; square 3", 9);
}

#[test]
fn test_to_phrase_with_a_type_word() {
	is!("to square a number: it*it; square 3", 9);
	is!("to cube a number: number*number*number; cube 2", 8);
	is!("to halve number x: x/2; halve 3", eval("1.5"));
}
