//! `x is int`, `x is a number`, `[1 2] is list of int`, `type of x`, `x as number = 9`
use crate::is;

#[test]
fn test_scalar_type_tests() {
	is!("3.5 is int", 0);
	is!("3.5 is rational", 1);
	is!("7/2 is rational", 1);
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

#[test]
fn test_type_of_value_is_type() {
	// card type-int: `type(x) is T` tests x, so it agrees with what type(x) prints
	is!("type(0.0) is int", 1);
	is!("type(0) is int", 1);
	is!("x = 0.0; type(x) is int", 1);
	is!("type(1.5) is int", 0);
	is!("type(1.5) is rational", 1);
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
	is!("type(0.0) == int", 1);
	is!("type(1.5) == rational", 1);
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
}

// card list-class: a list of instances is a list of their class, also where only the run time knows the items
#[test]
fn a_list_of_instances_is_a_list_of_their_class() {
	let classes = "class note{n:int}; class file{name:text}; ";
	is!(&format!("{classes}xs = [note(1), note(2)]; xs is list of note"), 1);
	is!(&format!("{classes}xs = [note(1), note(2)]; xs is list of file"), 0);
	is!(&format!("{classes}f(v) := v is list of note; f([note(1)]) * 10 + f([file(\"a\")])"), 10);
	is!(&format!("{classes}f(v) := v is list of note; f([1])"), 0);
	is!("f(v) := v is ints; f([1, 2]) * 10 + f([\"a\"])", 10);
	is!("f(v) := v is list of int; f([])", 1);
}
