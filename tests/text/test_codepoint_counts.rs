// card codepoints-traps: a one-character text is held as a codepoint, and it counts one code point and one grapheme
use crate::is;

#[test]
fn a_one_letter_text_counts_one_codepoint() {
	is!("t = \"ä\"; count t.codepoints", 1);
	is!("count \"a\".codepoints", 1);
	is!("c = 'a'; count c.codepoints", 1);
	is!("count \"ab\".codepoints", 2);
}

#[test]
fn a_one_letter_text_counts_one_grapheme() {
	is!("t = \"ä\"; count t.graphemes", 1);
	is!("xs = [\"é\", \"bc\"]; count xs#1.graphemes", 1);
}
