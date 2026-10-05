// User programs the emitter used to panic on: each one is an error value now
use warp::wasm_emitter::eval;
use warp::*;

use crate::common;

fn assert_error(code: &str) {
	match eval(code) {
		Node::Error(_) => {}
		other => panic!("expected an error for {code}, got {other:?}"),
	}
}

#[test]
fn test_float_function_result_in_exact_context_is_an_error() {
	common::fails_with("f(x:float) := x * 2; [1,2,3][f(1.0)]", "is a float where an exact Int is expected");
}

#[test]
fn test_missing_argument_without_default_is_an_error() {
	assert_error("f(a,b) := a+b; f(1)");
}

#[test]
fn test_malformed_ternary_is_an_error() {
	assert_error("1 ? 2");
}

#[test]
fn test_non_constant_range_bound_is_a_list() {
	is!("n=3; 1..n", ints(vec![1, 2]));
}

#[test]
fn test_cube_root_prefix_is_an_error_or_a_value() {
	assert!(!matches!(eval("∛8"), Node::Empty));
}

#[test]
fn test_update_of_a_non_variable_is_an_error() {
	assert_error("(1+2)++");
	assert_error("3 += 1");
}

#[test]
fn test_global_declaration_of_a_non_name_is_an_error() {
	assert_error("global 3");
}

#[test]
fn test_struct_field_of_an_unknown_type_is_an_error() {
	common::fails_with("type Point{x:Vec3}; 1+1", "unknown type");
}

#[test]
fn test_float_compound_assignment_uses_the_float_operator() {
	is!("x:float=7.5; x %= 2; x", 1.5);
	is!("x:float=1.5; x ^= 2; x", 2.25);
}
