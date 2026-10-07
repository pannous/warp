use warp::wasm_emitter::eval;
use warp::Node;
use crate::is;

fn error_text(code: &str) -> String {
	match eval(code) {
		Node::Error(message) => format!("{message}"),
		other => panic!("{code} should be a type error, got {other:?}"),
	}
}

#[test]
fn list_argument_for_an_int_parameter_is_a_type_error() {
	// P50: a list broadcasts now, the type error needs a text argument
	let message = error_text("foo(x:int):=x+1;foo(\"abc\")");
	assert!(message.contains("foo") && message.contains("x"), "{message}");
}

#[test]
fn text_argument_for_an_int_parameter_is_a_type_error() {
	let message = error_text("foo(x:int):=x+1;foo('abc')");
	assert!(message.contains("foo") && message.contains("x"), "{message}");
}

#[test]
fn list_is_never_silently_reduced_to_its_last_element() {
	assert_ne!(eval("foo(x):=x;foo([1 2 3])"), Node::Number(warp::Number::Int(3)));
}

#[test]
fn counting_a_parameter_makes_it_a_list() {
	is!("foo(x):=#x;foo [4 5 6 7]", 4);
	is!("foo:=#it;foo [4 5 6]", 3);
}

#[test]
fn matching_arguments_still_pass() {
	is!("foo(x:int):=x+1;foo(2)", 3);
	is!("foo(x=[1 2 3]):=x#1;foo([4 5 6])", 4);
}

#[test]
fn an_unknown_word_returned_or_cast_is_an_error() {
	// card functions-unknown: these trapped or gave back the data `cube 3`
	for code in ["def g(n: str) -> str: return cube \"x\"; g(\"a\")", "def g(n) { return cube 3 }; g(1)", "def g(n) -> int { cube 3 }; g(1)", "cube 3 as int"] {
		crate::common::fails_with(code, "cube is in the standard module math: write `use math`");
	}
}
