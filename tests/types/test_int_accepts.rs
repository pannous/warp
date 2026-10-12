//! card int-accepts: `is int` holds of whole numbers only, a ratio held by a variable or an untyped parameter is none
use crate::is;

#[test]
fn a_rational_variable_is_no_int() {
	is!("x = 2.5; x is int", false);
	is!("x = 1/3; x is int", false);
	is!("x = 4/2; x is int", true);
	is!("x = 3; x is int", true);
	is!("x = 1/3; x is number", true);
}

#[test]
fn a_rational_argument_is_no_int() {
	is!("f(x) := x is int; f(2.5)", false);
	is!("f(x) := x is int; f(3)", true);
	is!("f(x) := x is int; f(\"a\")", false);
	is!("f(x) := x is rational; f(2.5)", true);
}
