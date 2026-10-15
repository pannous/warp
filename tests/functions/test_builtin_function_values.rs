//! card builtin-functions (decision dir-lists): a built-in is a function value of type builtin, a subtype of function
use crate::common::fails_with;
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_builtin_reference_has_type_builtin() {
	assert_eq!(printed("type(function floor)"), "\"builtin: (x:number) -> int\"");
	assert_eq!(printed("f = function upper; type(f)"), "builtin");
}

#[test]
fn a_builtin_is_a_function() {
	assert_eq!(printed("function floor is function"), "yes");
	assert_eq!(printed("f = function upper; f is function"), "yes");
}

#[test]
fn a_builtin_function_value_is_called() {
	assert_eq!(printed("f = function upper; f(\"ab\")"), "\"AB\"");
	assert_eq!(printed("g = function floor; [1.5 2.5].map(g)"), "[1 2]");
}

#[test]
fn a_bare_builtin_assigned_needs_its_arguments() {
	fails_with("f = floor; f(2.5)", "floor needs 1 argument");
	fails_with("f = abs; f(-2)", "fix: function abs");
}

#[test]
fn a_bare_builtin_passed_to_map_stays_the_function() {
	assert_eq!(printed("[4 9].map(sqrt)"), "[2 3]");
}
