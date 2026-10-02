//! Builtins `min` and `max`.

mod common;
use common::fails_with;
use warp::is;

#[test]
fn test_min_max_of_ints() {
	is!("min(1,2)", 1);
	is!("max(1,2)", 2);
	is!("min(3,1,2)", 1);
	is!("max(3,1,2)", 3);
}

#[test]
fn test_min_max_of_floats_and_mixed() {
	is!("min(1.5,2.5)", 1.5);
	is!("max(1.5,2)", 2);
}

#[test]
fn test_min_max_of_variables_and_expressions() {
	is!("x=5; min(x,3)", 3);
	is!("max(1+1,3)", 3);
	is!("1+max(1,2)", 3);
	is!("max(1,min(5,2))", 2);
}

#[test]
fn test_user_definition_wins_over_the_builtin() {
	is!("min(a,b):=a*b; min(2,3)", 6);
}

#[test]
fn test_min_max_arguments_must_be_side_effect_free() {
	is!("f(x):=x; min(f(1),2)", 1);
}

#[test]
fn test_min_needs_two_arguments() {
	fails_with("min(1)", "min takes at least 2");
}
