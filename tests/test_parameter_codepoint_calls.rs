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

#[test]
fn a_character_literal_argument_disagrees_with_a_text_argument() {
	let message = error_text("id(x):=x;id('x');id('xy')");
	assert!(message.contains("annotate it"), "{message}");
}
