//! A declared return type converts the function's result (user, todo.md): `int square(x) = x*x` returns an Int also
//! for `square 3.1`, and `def square(x) : float = …` / `def square(x) as float = …` declare the return type after the
//! parameters.

use crate::is;

#[test]
fn test_type_before_the_name_converts_the_result() {
	is!("int square(x) = x*x; square 3.1", 9);
	is!("int square(x) = x*x; square(4) + square 3.1", 25);
	is!("int half(x) := x/2; half 3", 1);
	is!("float half(x) := x/2; half 3", 1.5);
}

#[test]
fn test_return_type_after_the_parameters() {
	is!("def square(x) : float = x*x; square 3", 9.0);
	is!("def square(x) as float = x*x; square 3", 9.0);
	is!("def square(x) as int = x*x; square 3.1", 9);
	is!("square(x) as int := x*x; square 3.1", 9);
}

#[test]
fn test_block_bodies_and_returns_convert_too() {
	is!("int half(x) := {return x/2}; half 3", 1);
	is!("int half(x){ return x/2 }; half 3", 1);
	is!("int half(x) := {y = x; if y > 2 {return y/2}; y/4}; half(3) + half(2)", 1);
	is!("float twice(x) := {x+x}; twice 2", 4.0);
}
