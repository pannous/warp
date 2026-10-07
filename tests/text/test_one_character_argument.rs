// card char-text: a one-character text passed to an untyped parameter is a text, as for a declared t:text and a
// default `t = "a"`: g("3") as float is 3, not the code point 51, and a later call with a longer text agrees
use crate::is;

#[test]
fn a_one_character_argument_is_a_text() {
	is!("g(t) := t as float; g(\"3\")", 3.0);
	is!("g(t) := t + \"!\"; g(\"a\")", "a!");
	is!("g(t) := t; x = \"3\"; g(x)", "3");
	is!("g(t) := count(t); g(\"a\") + g(\"bc\")", 3);
}
