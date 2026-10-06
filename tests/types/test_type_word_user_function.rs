//! A user function named like a type word (`double`, `int`…) is a loud error: the word is a type, rename the function

use warp::diagnostic::take_warnings;
use warp::wasm_emitter::eval;
use warp::Node;
use crate::is;

fn clash(code: &str) -> String {
	match eval(code) {
		Node::Error(reason) => reason.serialize(),
		other => panic!("{code} should be an error, got {other:?}"),
	}
}

#[test]
fn defining_a_function_named_like_a_type_word_is_an_error() {
	for code in ["double := it*2; double 4", "double(x) := x*2; double(4)", "int(x) := x+1; int(4)"] {
		let message = clash(code);
		assert!(message.contains("is a type; rename your function"), "{code}: {message}");
	}
	assert!(clash("double := it*2; double 4").contains("double"));
}

#[test]
fn a_function_with_another_name_is_no_clash() {
	is!("twice := it*2; twice 4", 8);
	is!("square(x) := x*x; square(4)", 16);
}

#[test]
fn the_type_word_still_casts_without_a_definition() {
	is!("double 4", 4.0);
	is!("x=4; double(x)", 4.0);
}

#[test]
fn a_clash_is_an_error_not_a_warning() {
	take_warnings();
	clash("double := it*2; double 4");
	assert!(take_warnings().is_empty());
}

#[test]
fn the_number_dot_form_calls_a_user_function() {
	is!("square(x):=x*x; 4.square", 16);
	is!("square(x):=x*x; 4.square()", 16);
}
