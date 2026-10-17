//! card reverse-characters: reversing a text reverses its characters, each grapheme cluster kept whole (a skin tone
//! stays on its emoji, a vowel point on its letter)
use crate::is;

#[test]
fn reverse_keeps_each_character_whole() {
	is!("reverse \"👍🏽ab\"", "ba👍🏽");
	is!("\"שָלוֹם\".reverse", "םוֹלשָ");
	is!("reversed \"q\\u{301}a\"", "aq\u{301}");
	is!("t = \"👍🏽!\"; reverse reverse t", "👍🏽!");
	is!("reverse \"\"", "");
}
