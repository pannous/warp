// card raise-nb: an error's text keeps all its lines, not only the first
use warp::node::Node;
use warp::wasm_emitter::eval;

fn message_of(code: &str) -> String {
	match eval(code) {
		Node::Error(message) => message.drop_meta().serialize().trim_matches('"').to_string(),
		other => panic!("expected an error for {code}, got {other:?}"),
	}
}

#[test]
fn a_raised_text_keeps_its_lines() {
	assert_eq!(message_of("raise(\"a\nb\")"), "a\nb");
	assert_eq!(message_of("f = [\"x\", \"y\"]; raise(join(f, \"\\n\") + \"\\nend\")"), "x\ny\nend");
	assert_eq!(message_of("raise(\"a back\\\\slash\")"), "a back\\slash");
}
