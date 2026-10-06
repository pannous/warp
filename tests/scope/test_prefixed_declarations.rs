//! `int x = v` declares like `x:int = v`: the value must fit the declared type

use warp::wasm_emitter::eval;
use warp::Node;
use crate::{is, eq};

fn error_text(code: &str) -> String {
	match eval(code) {
		Node::Error(message) => format!("{message}"),
		other => panic!("{code} should be an error, got {other:?}"),
	}
}

#[test]
fn a_value_of_the_wrong_kind_is_refused_with_a_prefixed_type() {
	for (code, declared) in [("int x = 2.5", "x is declared int"), ("int x = 'ab'", "x is declared int"), ("string s = 3", "s is declared string"), ("float f = 'ab'", "f is declared float")] {
		let message = error_text(code);
		assert!(message.contains(declared), "{code}: {message}");
	}
}

#[test]
fn a_fitting_value_is_accepted_with_a_prefixed_type() {
	is!("int x = 3; x + 1", 4);
	is!("string s = 'ab'; s", "ab");
	is!("float f = 2; f", 2.0);
	eq!(eval("float x = π; x"), eval("π"));
}
