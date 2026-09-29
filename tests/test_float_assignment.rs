use warp::*;

#[test]
fn test_float_local_assignment_keeps_its_float_value() {
	is!("f(x:float) := {y=x; y}; f(2.7)", 2.7);
	is!("f(x:float) := {y=x*2; y*2}; f(1.5)", 6.0);
	is!("f(x:float) := {y=x; y=y+1; y}; f(1.5)", 2.5);
	is!("f(x:float) := y=x/2; f(3)", 1.5);
}
