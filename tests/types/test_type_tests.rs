//! `x is int`, `x is a number`, `[1 2] is list of int`, `type of x`, `x as number = 9`
use crate::is;

#[test]
fn test_scalar_type_tests() {
	is!("3 is int", 1);
	is!("3.5 is int", 0);
	is!("7/2 is rational", 1); // 3.5 is a float (decision exact-default)
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
	is!("type of 2.5", warp::Node::Symbol("float".to_string())); // decision exact-default
	is!("x=\"hello\"; type of x", warp::Node::Symbol("text".to_string()));
}

#[test]
fn test_as_declaration() {
	is!("x as number = 9; x", 9);
	is!("x as number = 9; x is a number", 1);
	is!("y as int = 4; y + 1", 5);
}

#[test]
fn test_type_of_value_is_type() {
	// card type-int: `type(x) is T` tests x, so it agrees with what type(x) prints
	is!("type(0.0) is float", 1); // decision exact-default: 0.0 is a float
	is!("type(0) is int", 1);
	is!("x = 0.0; type(x) is float", 1);
	is!("type(1.5) is int", 0);
	is!("type(3/2) is rational", 1);
	is!("type(\"a\") is text", 1);
	is!("int is number", 1);
	is!("number is int", 0);
}

#[test]
fn test_types_compare_exactly_with_equals() {
	// card type-equal, implied by decision #30: `is` tests the subtype, `==` between types asks for the same type
	is!("type(2) == type(3)", 1);
	is!("x=2; y=3; type(x) == type(y)", 1);
	is!("type(2) == type(3.5)", 0);
	is!("type(2) != type(3)", 0);
	is!("type(0.0) == float", 1); // decision exact-default
	is!("type(3/2) == rational", 1);
	is!("type(0) == number", 0);
	is!("type(0) is number", 1);
}

#[test]
fn test_bare_type_names_are_type_values() {
	// cards type-value-type, real-equality: a type word standing bare is that type as a value (one table, cleanup-closed-lists)
	is!("type(π) == real", 1);
	is!("t = type(0); t is int", 1);
	is!("t = type(0); t is number", 1);
	is!("t = type(0); t is text", 0);
	is!("t = type(0); t == int", 1);
	is!("int == int", 1);
	is!("int == number", 0);
	is!("3 == int", 0);
}
