use std::f64::consts::{PI, SQRT_2};
use warp::*;

mod common;

// A declared float target is the one place where an exact real is accepted with its precision loss
#[test]
fn test_declared_float_accepts_exact_reals() {
	is!("float x = π; x", PI);
	is!("x:float = √2; x", SQRT_2);
	is!("float y = 1/3; y", 1.0 / 3.0);
	is!("y:float = 1/3; y", 1.0 / 3.0);
	is!("float z = 2π; z", 2.0 * PI);
}

#[test]
fn test_declared_float_reassignment_accepts_exact_reals() {
	is!("float x = 1.5; x = π; x", PI);
	is!("x:float = 1.5; x = 1/3; x", 1.0 / 3.0);
}

#[test]
fn test_declared_float_global_accepts_exact_reals() {
	is!("global x:float = √2; x", SQRT_2);
	is!("global float y = 1/3; y", 1.0 / 3.0);
}

#[test]
fn test_declared_float_parameter_accepts_exact_reals() {
	is!("f(x:float) := x * 2; f(1/3)", 2.0 / 3.0);
	is!("f(x:float) := x * 2; f(π)", 2.0 * PI);
}

// implicit exact → float and float → Int stay as they are
#[test]
fn test_undeclared_and_int_targets_keep_their_rules() {
	is!("x = 1/3; x * 3", 1);
	common::fails_with("x:int = 1.5", "x");
	common::fails_with("f(x:float) := [10,20,30][x]; f(2.0)", "is a float where an exact Int is expected");
}

// an exact expression is computed exactly and rounded once to the nearest f64
#[test]
fn test_declared_float_rounds_an_exact_expression_once() {
	is!("float y = 0.1+0.2; y", 0.3);
	is!("y:float = 1/3 + 1/6; y", 0.5);
	is!("x = 1/3; float y = x; y", 1.0 / 3.0);
	is!("float x = 2^100; x", 2f64.powi(100));
	is!("x:float = π; x = √2; x", SQRT_2);
}
