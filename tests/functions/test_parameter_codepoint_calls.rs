use warp::wasm_emitter::eval;
use warp::Node;

fn error_text(code: &str) -> String {
	match eval(code) {
		Node::Error(message) => format!("{message}"),
		other => panic!("{code} should be an error, got {other:?}"),
	}
}

#[test]
fn a_character_literal_argument_disagrees_with_an_int_argument() {
	for code in ["id(x):=x;id(1)+id('x')", "id(x):=x;id(1);id('x')", "id(x):=x;id('x');id(1)"] {
		let message = error_text(code);
		assert!(message.contains("id is called with"), "{code}: {message}");
		assert!(message.contains("annotate it"), "{code}: {message}");
	}
}

// card char-text: a one-character text is a text argument too, so the calls agree (was: an "annotate it" error)
#[test]
fn a_character_literal_argument_agrees_with_a_text_argument() {
	assert_eq!(eval("id(x):=x;id('x');id('xy')"), Node::Text("xy".into()));
}
