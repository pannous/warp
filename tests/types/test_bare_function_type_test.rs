//! card bare-fn-type: a bare name of a function that needs arguments, tested against a type after `is`, is the function
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_library_word_is_a_function() {
	assert_eq!(printed("floor is function"), "yes");
	assert_eq!(printed("floor is builtin"), "yes");
	assert_eq!(printed("floor is int"), "no");
	assert_eq!(printed("print is function"), "yes");
	assert_eq!(printed("floor is (function or int)"), "yes");
}

#[test]
fn a_programs_function_is_a_function() {
	assert_eq!(printed("f(x):=x*2; f is function"), "yes");
	assert_eq!(printed("f(x):=x*2; f is builtin"), "no");
}

#[test]
fn a_variable_of_the_name_is_the_value() {
	assert_eq!(printed("floor = 3; floor is int"), "yes");
	assert_eq!(printed("f(floor):=floor is int; f 2"), "yes");
	assert_eq!(printed("x = 2.5; floor(x) is int"), "yes");
}

#[test]
fn a_lambda_parameter_of_the_name_is_the_value() {
	assert_eq!(printed("g = (floor => floor is int); g(3)"), "yes");
}

/// a function never equals a type (decision #30): no, hinting the `is` test
#[test]
fn a_function_compared_with_a_type_by_equality_is_no() {
	assert_eq!(printed("floor == function"), "no");
	assert_eq!(printed("floor != int"), "yes");
	assert_eq!(printed("f(x):=x; function == f"), "no");
}
