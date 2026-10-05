// An unknown word applied to a value (`cube 3`) is no call: it stays data, with a got-it warning naming the explicit forms
// (Decided #7 makes `cube(3)` an error; notes/unknown_prefix_word.md)
use crate::common::fails_with;
use warp::diagnostic::take_warnings;
use warp::wasm_emitter::eval;

fn warns_unknown_word(code: &str) -> bool {
	take_warnings();
	eval(code);
	take_warnings().iter().any(|warning| warning.message.contains("is no function"))
}

#[test]
fn an_unknown_word_applied_to_a_value_warns() {
	assert!(warns_unknown_word("cube 3"));
	assert!(warns_unknown_word("cube \"a\""));
	assert!(warns_unknown_word("cube [1 2]"));
	assert_eq!(eval("cube 3").serialize().trim(), "cube 3"); // still the data it was
}

#[test]
fn known_words_and_plain_data_do_not_warn() {
	assert!(!warns_unknown_word("cube(x):=x*x*x; cube 3"));
	assert!(!warns_unknown_word("print 3"));
	assert!(!warns_unknown_word("hello world"));
	assert!(!warns_unknown_word("person{name:\"Joe\"}"));
	assert!(!warns_unknown_word("[cube 3]"));
	assert!(!warns_unknown_word("x=3; x"));
}

#[test]
fn the_call_form_stays_an_error() {
	fails_with("cube(3)", "undefined function: cube");
}
