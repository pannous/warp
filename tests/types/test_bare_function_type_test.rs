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
