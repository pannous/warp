//! card number-keeps-exact: a variable declared `number` keeps the number it is given, exact or float, as a number
//! parameter and a number field do; only `float` converts (user: "`number` means any number, keeping whatever it holds")
use crate::is;

#[test]
fn a_number_variable_keeps_a_ratio() {
	is!("x: number = 1/3; x is rational", true);
	is!("x: number = 1/3; x * 3", 1);
	is!("x: number = 2; x = 1/3; x is rational", true);
	is!("x: number = 0.5 as float; x = 1/3; x * 3", 1);
}

#[test]
fn a_number_variable_keeps_a_float() {
	is!("x: number = 1/3; x = 0.5 as float; x is float", true);
}

#[test]
fn a_float_variable_still_converts() {
	is!("x: float = 1/3; x is float", true);
}
