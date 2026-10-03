use warp::wasm_emitter::eval;
use warp::is;

#[test]
fn to_defines_a_function_with_a_body_to_the_end_of_the_statement() {
	is!("to square x: x*x; square 3", 9);
	is!("to add a b: a+b; add 2 3", 5);
	is!("to greet: \"hi\"; greet", "hi");
	is!("to square x: x*x\nsquare 4", 16);
}

#[test]
fn to_takes_a_block_or_an_indented_block_as_body() {
	is!("to bump x: {y=x+1; y*2}; bump 3", 8);
	is!("to bump x:\n\ty=x+1\n\ty*2\nbump 3", 8);
}

#[test]
fn to_stays_a_range_and_a_variable_name() {
	is!("x=0; for i in 1 to 4 {x+=i}; x", 10);
	is!("to=5; to+1", 6);
	is!("r=1 to 3; count r", 3);
}

#[test]
fn a_typed_phrase_parameter_is_named_by_its_type() {
	crate::common::fails_with("to square a number: a*a; square 3", "undefined variable: a");
	is!("to square a number: number*number; square 3", 9);
}

#[test]
fn nand_is_not_and() {
	is!("1 nand 1", 0);
	is!("1 nand 0", 1);
	is!("0 nand 0", 1);
	is!("1 ¬& 1", 0);
	is!("1 ¬& 0", 1);
	is!("1 nand 1 nand 1", 1);
}
