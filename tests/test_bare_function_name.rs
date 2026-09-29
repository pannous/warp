mod common;
use common::fails_with;
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
