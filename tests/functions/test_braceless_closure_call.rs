// card calling-closure: a variable holding a function is called without parentheses, as with them
use crate::is;

#[test]
fn a_function_variable_is_called_without_parentheses() {
	is!("f = function upper; f \"a\"", "A");
	is!("add(a, b) := a + b; g = function add; g 1 2", 3);
	assert_eq!(warp::wasm_emitter::eval("f = function upper; [f \"a\", f \"b\"]").serialize(), "['A' 'B']");
	is!("f = x => x + 1; f 3 == 4", true);
}

#[test]
fn a_function_variable_given_a_number_is_refused() {
	crate::common::fails_with("f = function upper; f = 5; f \"a\"", "f was a function, is given an Int");
	is!("f = function upper; f = function lower; f \"A\"", "a");
}
