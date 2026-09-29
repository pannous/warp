use warp::*;

mod common;

#[test]
fn test_float_read_in_exact_context_is_refused_loudly() {
	common::fails_with("f(x:float) := [10,20,30][x]; f(2.0)", "is a float where an exact Int is expected");
	common::fails_with("f(x:float) := x << 1; f(2.5)", "float");
}

#[test]
fn test_explicit_cast_truncates_a_float() {
	is!("f(x:float) := x as int; f(2.7)", 2);
	is!("f(x:float) := int(x); f(-2.7)", -2);
	is!("f(x:float) := [10,20,30][int(x)]; f(2.0)", 30);
}
