use crate::common::fails_with;
use warp::*;

#[test]
fn test_bare_name_of_function_with_parameter_is_an_error() {
	fails_with("sq:=it*2;sq", "sq needs 1 argument");
}

#[test]
fn test_bare_name_of_typed_function_is_an_error() {
	fails_with("f(a:int, b:int) = a + b; f", "f needs 2 arguments");
}

#[test]
fn test_bare_name_of_function_without_parameters_calls_it() {
	is!("f:={1;2;3};f", 3);
}

#[test]
fn test_bare_name_of_function_with_only_defaults_calls_it() {
	is!("f(x=1):=x*2;f", 2);
}

#[test]
fn test_bare_name_counts_only_required_parameters() {
	fails_with("f(x, y=1) = x + y; f", "f needs 1 argument");
}
