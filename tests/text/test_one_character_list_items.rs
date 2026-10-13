// card character-text: a one-character literal in a list that holds texts is a text too, so a list built from a
// parameter reads back like the same list written out: `wrap(" ")` is ["x" " "], not ['x' " "] (found by self-parser)
use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn a_one_character_literal_beside_a_text_is_a_text() {
	assert_eq!(
		eval("def wrap(b) := [\"x\", b]; wrap(\" \")").serialize(),
		"[\"x\" \" \"]"
	);
	assert_eq!(eval("[\"x\", \"yy\"]").serialize(), "[\"x\" \"yy\"]");
	is!("def wrap(b) := [\"x\", b]; type(wrap(\" \")[0])", "text");
	// characters on their own stay characters
	assert_eq!(eval("['x', 'y']").serialize(), "['x' 'y']");
}
