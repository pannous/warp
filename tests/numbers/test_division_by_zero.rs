// P66 (user, 2026-10-05): Int division by zero is always the catchable error divide_by_zero; only a float division
// (a decimal point written in it: 1.0/0.0) is ∞
use crate::common::fails_with;
use warp::is;

#[test]
fn int_division_by_zero_is_an_error() {
	fails_with("1/0", "divide by zero");
	fails_with("a=1; b=0; a/b", "divide by zero");
	fails_with("f() := 1/0; f()", "divide by zero");
	fails_with("{1/0}", "divide by zero");
	is!("try 1/0 else 7", 7);
	is!("f() := 1/0; try f() else 7", 7);
}

#[test]
fn float_division_by_zero_is_infinity() {
	is!("1.0/0.0 == ∞", 1);
	is!("1/0.0 == ∞", 1);
}
