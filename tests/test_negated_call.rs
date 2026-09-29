use warp::*;

#[test]
fn test_function_applied_to_a_negative_number() {
	is!("twice(x):=x*2;twice -3", -6);
}

#[test]
fn test_variable_minus_number_stays_subtraction() {
	is!("x=5;x -1", 4);
}
