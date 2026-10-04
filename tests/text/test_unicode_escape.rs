// `\u{e9}` in a text is the code point U+00E9, and the text is normalized to NFC like source text (wiki/Footguns.md)
use warp::is;

#[test]
fn a_unicode_escape_is_its_code_point() {
	is!("'\\u{e9}'=='e\\u{301}'", 1);
	is!("\"caf\\u{e9}\"", "café");
	is!("#\"\\u{1F600}\"", 1);
}
