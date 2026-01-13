use warp::is;

// DONE: global keyword implementation

#[test]
fn test_global_int_simple() {
	is!("global x=10; x+5", 15);
}

#[test]
fn test_global_int_multiple() {
	is!("global x=10; global y=20; x+y", 30);
}

#[test]
fn test_global_with_expression() {
	is!("global x=3*4; x+1", 13);
}

#[test]
fn test_global_float_with_pi() {
	use std::f64::consts::PI;
	is!("global x=1+π", 1.0 + PI);
	is!("global x=1+π; x+2", 3.0 + PI);
}

#[test]
fn test_global_reassignment() {
	// DONE global reassignment: second declaration reuses existing global
	is!("global x=5; global x=10; x", 10);
}
