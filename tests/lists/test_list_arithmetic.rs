use warp::wasm_emitter::eval;
use warp::*;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn test_square_list_keeps_all_items_that_are_arithmetic() {
	assert_eq!(printed("[1+2, 3]"), "[3 3]");
	assert_eq!(printed("[2*3, 4]"), "[6 4]");
	assert_eq!(printed("[6/3, 4]"), "[2 4]");
	assert_eq!(printed("[1/2, 3/4]"), "[0.5 0.75]");
	assert_eq!(printed("[1 2/2]"), "[1 1]");
}

#[test]
fn test_square_list_of_one_arithmetic_item_is_a_list() {
	assert_eq!(printed("[1+2]"), "[3]");
	is!("x=[1/2, 3/4]; count x", 2);
}

#[test]
fn test_round_group_of_arithmetic_is_still_an_expression() {
	is!("(1+2)*3", 9);
	is!("(1+2)", 3);
}
