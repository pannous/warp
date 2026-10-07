use warp::wasm_emitter::eval;
use warp::Node;

/// P173 (user, 2026-10-07): a character and an Int make the parameter take any value (was an "annotate it" error)
#[test]
fn a_character_literal_argument_and_an_int_argument_make_it_any() {
	assert_eq!(eval("id(x):=x;id(1)+id('x')"), Node::Text("1x".into()));
	assert_eq!(eval("id(x):=x;id('x');id(1)"), warp::int(1));
}

// card char-text: a one-character text is a text argument too, so the calls agree (was: an "annotate it" error)
#[test]
fn a_character_literal_argument_agrees_with_a_text_argument() {
	assert_eq!(eval("id(x):=x;id('x');id('xy')"), Node::Text("xy".into()));
}
