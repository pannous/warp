// card calling-closure: a variable holding a function is called without parentheses, as with them
use crate::is;

#[test]
fn a_function_variable_is_called_without_parentheses() {
	is!("f = function upper; f \"a\"", "A");
	is!("add(a, b) := a + b; g = function add; g 1 2", 3);
	assert_eq!(warp::wasm_emitter::eval("f = function upper; [f \"a\", f \"b\"]").serialize(), "['A' 'B']");
	is!("f = x => x + 1; f 3 == 4", true);
}
