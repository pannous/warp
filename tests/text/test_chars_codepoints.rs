use crate::is;
use warp::wasm_emitter::eval;

// card chars-codepoints (user decision): "abc".chars is a list of characters (codepoints) and prints ['a' 'b' 'c'];
// a literal ['a' 'b'] keeps its characters; 'a' == "a" stays yes

#[test]
fn test_chars_is_a_list_of_characters() {
	is!("type(\"abc\".chars)", "list of codepoint");
	is!("type(\"abc\".chars#1)", "codepoint");
	is!("type(['a' 'b']#1)", "codepoint");
	is!("type(split(\"a b\", \" \"))", "list of text");
}

#[test]
fn test_a_list_of_characters_prints_them_in_single_quotes() {
	assert_eq!(eval("\"abc\".chars").serialize(), "['a' 'b' 'c']");
	assert_eq!(eval("['a' 'b']").serialize(), "['a' 'b']");
	is!("\"abc\".chars.string", "['a' 'b' 'c']");
	is!("['a' \"bc\" 3].string", "['a' \"bc\" 3]");
	is!("['\\''].string", "['\\'']");
}

#[test]
fn test_a_character_equals_its_one_character_text() {
	is!("'a' == \"a\"", true);
	is!("\"abc\".chars == ['a' 'b' 'c']", true);
}
