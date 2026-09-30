use warp::*;

mod common;

#[test]
fn test_float_local_assignment_keeps_its_float_value() {
	is!("f(x:float) := {y=x; y}; f(2.7)", 2.7);
	is!("f(x:float) := {y=x*2; y*2}; f(1.5)", 6.0);
	is!("f(x:float) := {y=x; y=y+1; y}; f(1.5)", 2.5);
	is!("f(x:float) := y=x/2; f(3)", 1.5);
}

#[test]
fn test_float_assignment_in_exact_context_is_refused_loudly() {
	common::fails_with("f(x:float) := {y=x; [1,2,3][y=x]}; f(1.0)", "assigns a float where an exact Int is expected");
}

#[test]
fn test_as_float_promotes_and_never_truncates() {
	is!("x=3; y=x as float; y / 2", 1.5);
	is!("f(x) := x as float; f(3) / 2", 1.5);
	common::fails_with("[1,2,3][2.5 as float]", "`as float` promotes");
}
