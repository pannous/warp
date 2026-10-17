//! A loop's value is a number or ø, decided at run time (P55): a loop that never ran is ø also where its value is
//! assigned (card loop-empty), a function's loop over a parameter gives its last body value (card loop-param)
use crate::is;
use warp::*;

#[test]
fn a_loop_that_never_ran_is_empty_also_assigned() {
	is!("x = for i in [] { i }; x", Empty);
	is!("i = 5; while i < 3 { i++ }", Empty);
	is!("x = for i in [1 2] { i }; x + 1", 3);
}

#[test]
fn a_function_loop_over_a_parameter_gives_its_last_body_value() {
	is!("f(xs) := for x in xs { x + 1 }; f([1, 2])", 3);
	is!("f(n) := for i in 1..n { i * 2 }; f(3)", 4);
	is!("f(xs) := { r = []; for x in xs { r.add(x) }; r }; f([1 2])", ints(vec![1, 2]));
}
