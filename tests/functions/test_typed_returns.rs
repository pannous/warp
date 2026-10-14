//! A declared return type converts the function's result when nothing is lost, and an int result refuses a fraction
//! (user 2026-10-12, card lossy-return; it truncated before, user todo.md): `int square(x) = x*x` of `square 3.1`
//! fails with "an int must be a whole number", and `def square(x) : float = …` / `def square(x) as float = …` declare
//! the return type after the parameters.

use crate::common::fails_with;
use crate::is;

const NOT_WHOLE: &str = "an int must be a whole number";

#[test]
fn test_type_before_the_name_converts_the_result() {
	fails_with("int square(x) = x*x; square 3.1", NOT_WHOLE);
	fails_with("int square(x) = x*x; square(4) + square 3.1", NOT_WHOLE);
	fails_with("int half(x) := x/2; half 3", NOT_WHOLE);
	is!("float half(x) := x/2; half 3", 1.5);
}

#[test]
fn test_return_type_after_the_parameters() {
	is!("def square(x) : float = x*x; square 3", 9.0);
	is!("def square(x) as float = x*x; square 3", 9.0);
	fails_with("def square(x) as int = x*x; square 3.1", NOT_WHOLE);
	fails_with("square(x) as int := x*x; square 3.1", NOT_WHOLE);
}

#[test]
fn test_block_bodies_and_returns_convert_too() {
	fails_with("int half(x) := {return x/2}; half 3", NOT_WHOLE);
	fails_with("int half(x){ return x/2 }; half 3", NOT_WHOLE);
	fails_with("int half(x) := {y = x; if y > 2 {return y/2}; y/4}; half(3) + half(2)", NOT_WHOLE);
	is!("float twice(x) := {x+x}; twice 2", 4.0);
}
