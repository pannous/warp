use warp::*;

mod common;

const FLOAT_IN_EXACT_CONTEXT: &str = "is a float where an exact Int is expected";

#[test]
fn test_logical_and_bit_operators_on_a_float_are_refused_loudly() {
	for operator in ["and", "or", "xor", "&", "|"] {
		common::fails_with(&format!("f(x:float) := x {operator} 1; f(2.5)"), FLOAT_IN_EXACT_CONTEXT);
	}
}

#[test]
fn test_logical_and_bit_operators_on_ints_still_work() {
	is!("12 & 10", 10);
	is!("12 xor 10", 6);
}
