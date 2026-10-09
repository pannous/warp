// A one-character text is held as a codepoint (equal to 'a'), yet its bytes are those of its UTF-8 text, not 8 per element
use crate::is;

#[test]
fn a_one_letter_text_has_its_utf8_bytes() {
	is!("\"a\".bytes", 1);
	is!("c = 'a'; c.bytes", 1);
	is!("\"é\".bytes", 2);
}

#[test]
fn one_letter_list_items_have_their_utf8_bytes() {
	is!("xs = [\"a\", \"bc\"]; xs[0].bytes", 1);
	is!("xs = [\"é\", \"bc\"]; x = xs[0]; x.bytes", 2);
	is!("const xs = [\"a\", \"bc\"]; xs[0].bytes", 1);
}
