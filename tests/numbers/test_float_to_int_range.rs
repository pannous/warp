use crate::is;
use crate::common;

#[test]
fn test_rounding_functions_give_exact_ints() {
	is!("floor(2.7)", 2);
	is!("ceil(2.1)", 3);
	is!("round(2.5)", 2);
	is!("round(3.5)", 4);
	is!("floor(0 - 2.5)", -3);
}

#[test]
fn test_float_beyond_int_range_fails_cleanly() {
	common::fails_with("x = √1e40; floor(x)", "float out of int range");
	common::fails_with("x = √1e40; ceil(x)", "float out of int range");
	common::fails_with("x = 0 - √1e40; round(x)", "float out of int range");
	common::fails_with("f(x:float) := int(x); f(1e20)", "float out of int range");
}
