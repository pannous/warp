//! A function that hands back one of its parameters returns the function value it was given (card prints-closure):
//! `g = x => x; g(y => y*2)(4)` calls the lambda
use crate::is;

#[test]
fn a_returned_parameter_is_called() {
	is!("g = x => x; g(y => y*2)(4)", 8);
	is!("sq(y) := y*y; g = x => x; g(sq)(5)", 25);
}

#[test]
fn a_variable_holds_the_returned_function() {
	is!("g = x => x; h = g(y => y*2); h(4)", 8);
	is!("id(x) := x; y = id(5); y + 1", 6);
}
