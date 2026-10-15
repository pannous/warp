//! card print-function: the host's print is a function value where a function is expected, like a library word
use crate::common::fails_with;
use warp::wasm_emitter::eval;

#[test]
#[cfg(feature = "native")]
fn print_passed_to_each_prints_every_item() {
	assert_eq!(crate::common::printed("[1 2].each(print)"), "1\n2\n");
	assert_eq!(crate::common::printed("xs = [3 4]; each print on: xs"), "3\n4\n");
}

#[test]
#[cfg(feature = "native")]
fn a_print_reference_is_called_later() {
	assert_eq!(crate::common::printed("f = function print; f(\"hello\")"), "hello\n");
}

#[test]
fn a_bare_print_assigned_needs_its_argument() {
	fails_with("f = print; f(2)", "print needs 1 argument");
	fails_with("f = print; f(2)", "fix: function print");
}

#[test]
fn counting_words_and_conversions_are_function_values() {
	assert_eq!(eval("m = [\"ab\" \"c\"]; m.map(len)").serialize(), "[2 1]");
	assert_eq!(eval("m = [\"ab\" \"c\"]; map(m, size)").serialize(), "[2 1]");
	assert_eq!(eval("xs = [1.5 2]; xs.map(int)").serialize(), "[1 2]");
	assert_eq!(eval("xs = [1 2]; xs.map(str)").serialize(), "['1' '2']");
	fails_with("f = len", "fix: function len");
}
