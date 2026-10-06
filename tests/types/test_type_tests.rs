//! `x is int`, `x is a number`, `[1 2] is list of int`, `type of x`, `x as number = 9`
use crate::is;

#[test]
fn test_scalar_type_tests() {
	is!("3 is int", 1);
	is!("3.5 is int", 0);
	is!("3.5 is rational", 1);
	is!("3 is rational", 1);
	is!("3.5 is number", 1);
	is!("\"a\" is text", 1);
	is!("\"abc\" is text", 1);
	is!("\"abc\" is int", 0);
}

#[test]
fn test_articles_are_optional() {
	is!("x=9; x is a number", 1);
	is!("x=9; x is number", 1);
	is!("x=9; x is an int", 1);
	is!("x=\"hi there\"; x is a text", 1);
	is!("x=9; x is a text", 0);
}

#[test]
fn test_list_type_tests() {
	is!("[1 2] is list of int", 1);
	is!("[1 2] is ints", 1);
	is!("[1 2] is list", 1);
	is!("[1 2] is list of number", 1);
	is!("[1 2] is list of text", 0);
	is!("[1 2] is texts", 0);
	is!("x=[1 2]; x is a list of int", 1);
}

#[test]
fn test_pi_is_real_and_a_number() {
	is!("π is real", 1);
	is!("π is number", 1);
	is!("π is rational", 0);
	is!("π is int", 0);
}

#[test]
fn test_a_variable_on_the_right_stays_equality() {
	is!("x=3; y=3; x is y", 1);
	is!("x=3; y=4; x is y", 0);
	is!("3 is 3", 1);
	is!("int=3; 3 is int", 1);
}

#[test]
fn test_type_of_is_type() {
	is!("x=5; type of x", warp::Node::Symbol("int".to_string()));
	is!("type of 2.5", warp::Node::Symbol("rational".to_string()));
	is!("x=\"hello\"; type of x", warp::Node::Symbol("text".to_string()));
}

#[test]
fn test_as_declaration() {
	is!("x as number = 9; x", 9);
	is!("x as number = 9; x is a number", 1);
	is!("y as int = 4; y + 1", 5);
}
