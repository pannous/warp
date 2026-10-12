// A minus glued to its operand after a callable name is a negative argument, for every callable as for the math words
// (`abs -2`): `int -3`, `f -2`. Written apart it subtracts (`abs - 2`, `f - 2`); after a variable it always subtracts
use crate::common::fails_with;
use crate::is;

#[test]
fn a_type_takes_a_negative_argument() {
	is!("int -3", -3);
	is!("int8 -3", -3);
	is!("float -2", -2.0);
	is!("text -5", "-5");
}

#[test]
fn a_user_function_takes_a_negative_argument() {
	is!("f(x):=x*2; f -2", -4);
	is!("f(x):=x*2; f -2 + 1", -3);
}

#[test]
fn a_library_word_takes_a_negative_argument() {
	is!("floor -2.5", -3);
	is!("ceil -2.5", -2);
	is!("abs -2", 2);
}

#[test]
fn a_minus_written_apart_subtracts() {
	fails_with("f(x):=x*2; f - 2", "f");
	fails_with("abs - 2", "function - int");
}

#[test]
fn a_variable_minus_a_number_subtracts() {
	is!("x=5; x -2", 3);
	is!("x=5; x - 2", 3);
	is!("int = 3; int -1", 2);
	is!("p = {size: 5}; p.size -1", 4);
}
