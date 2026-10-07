use warp::wasm_emitter::eval;
use warp::Node;
use crate::is;

fn error_text(code: &str) -> String {
	match eval(code) {
		Node::Error(message) => format!("{message}"),
		other => panic!("{code} should be an error, got {other:?}"),
	}
}

#[test]
fn untyped_parameter_takes_the_kind_all_calls_agree_on() {
	is!("id(x):=x;id([1 2 3])#2", 2);
	is!("id(x):=x;id(2.5) + 1", 3.5);
	is!("id(x):=x;id('abc')", "abc");
}

#[test]
fn untyped_parameter_with_int_calls_or_none_stays_int() {
	is!("inc(x):=x+1;inc(2)", 3);
	is!("inc(x):=x+1;inc(2) + inc(5)", 9);
}

/// P173 (user, 2026-10-07): calls that disagree make the parameter take any value (was a loud "annotate it" error)
#[test]
fn disagreeing_calls_make_the_parameter_any() {
	is!("id(x):=x;id([1 2 3]);id(2)", 2);
	is!("id(x):=x;id([1 2 3])#2 + id(2)", 4);
}

#[test]
fn a_type_error_names_the_kind_with_its_article() {
	// P50: a list broadcasts now, the type error needs a text argument
	let message = error_text("foo(x:int):=x+1;foo(\"abc\")");
	assert!(message.contains("needs an Int for parameter x"), "{message}");
}
