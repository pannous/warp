//! Card closures-untyped: a closure reached as a value (returned, in a list) takes any argument; its call helper
//! `closure_call_n` takes Nodes unless every closure of that arity takes numbers (type_closure_calls)
use crate::is;

#[test]
fn test_a_returned_closure_takes_a_text() {
	is!("make = () => (t => t + \"!\"); a = make(); a(\"x\")", "x!");
	is!("make = () => (t => t + \"!\"); a = make(); a(\"xy\")", "xy!");
	is!("def make(){ def add(t){ t + \"!\" }; function add }; a = make(); a(\"x\")", "x!");
}

#[test]
fn test_a_closure_in_a_list_takes_a_text() {
	is!("fs = [t => t + \"!\"]; fs#1(\"x\")", "x!");
}

#[test]
fn test_a_returned_closure_takes_numbers_and_lists() {
	is!("make = () => (t => t * 2); a = make(); a(4)", 8);
	is!("make = () => (t => count t); a = make(); a([1,2,3])", 3);
}

/// a lambda reading the loop variable, given to a function, is a closure of each turn's value (it was made a function
/// of its own reading the variable before the loop: count_by(xs, y => y == x) counted 0 for every x)
#[test]
fn a_lambda_of_the_loop_variable_given_to_a_function_sees_each_turn() {
	let counted = "g(xs, p) := { n = 0; for y in xs { if p(y) { n = n + 1 } }; n }";
	is!(&format!("{counted}; xs = [1, 2, 2]; out = []; for x in xs {{ out = out + [g(xs, z => z == x)] }}; out"), warp::ints(vec![1, 2, 2]));
	is!("use list; xs = [1, 2, 2, 3]; [count_by(xs, y => y == x) for x in xs]", warp::ints(vec![1, 2, 2, 1]));
}
