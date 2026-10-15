// card character-text, decided by P126's follow-up (notes/decisions.md): a character inside a container serializes
// with double quotes like a text, `["a" "bb"]`, and reads back the same (one-letter "a" parses as a character); a
// character on its own keeps its single quotes
use warp::parse;
use warp::wasm_emitter::eval;

#[test]
fn a_character_in_a_container_serializes_with_double_quotes() {
	assert_eq!(eval("def wrap(b) := [\"x\", b]; wrap(\" \")").serialize(), "['x' ' ']");
	assert_eq!(eval("[\"x\", \" \"]").serialize(), "['x' ' ']");
	assert_eq!(eval("sort([\"bb\", \"a\", \"ccc\"])").serialize(), "['a' \"bb\" \"ccc\"]");
	assert_eq!(parse("['\"']").serialize(), "['\"']");
	assert_eq!(parse("'x'").serialize(), "'x'");
	for written in ["[\"a\" \"bb\"]", "[\"\\\"\" 'b']"] {
		let node = parse(written);
		assert_eq!(parse(&node.serialize()), node, "{written}");
	}
}
