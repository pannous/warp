use crate::is;

// card chars-codepoints follow-up (user decision A, a one-character text is a character): a parameter given "a" holds
// a one-character text, whose type at run time is codepoint too, as type("a") and x = "a"; type(x) say

#[test]
fn test_a_parameter_holding_one_character_is_a_codepoint() {
	is!("def g(t) := type(t); g(\"a\")", "codepoint");
	is!("f(t:text) := type(t); f(\"a\")", "codepoint");
	is!("def g(t) := type(t); g(\"ä\")", "codepoint");
}

#[test]
fn test_a_parameter_holding_more_or_fewer_characters_is_a_text() {
	is!("def g(t) := type(t); g(\"ab\")", "text");
	is!("def g(t) := type(t); g(\"\")", "text");
}
