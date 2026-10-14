// card list-text-quotes, decided by P126's follow-up (notes/decisions.md): a `"` or `\` inside a quoted text is
// escaped by a `\`, so Node::serialize writes what the runtime prints (text_quoted) and the text reads back
use warp::parse;
use warp::wasm_emitter::eval;

#[test]
fn a_quote_or_backslash_in_a_serialized_text_is_escaped() {
	assert_eq!(eval("quote = \"\\\"\"; [quote + \"ab\" + quote, \"b\"]").serialize(), "[\"\\\"ab\\\"\" \"b\"]");
	assert_eq!(parse("[\"a\\\\b\"]").serialize(), "[\"a\\\\b\"]");
	assert_eq!(parse("['\\\\']").serialize(), "[\"\\\\\"]");
	assert_eq!(parse("\"say \\\"hi\\\"\"").serialize(), "\"say \\\"hi\\\"\"");
	for written in ["[\"say \\\"hi\\\"\" \"a\\\\b\"]", "\"\\\\\"", "['\\\\' 'b']"] {
		let node = parse(written);
		assert_eq!(parse(&node.serialize()), node, "{written}");
	}
}
