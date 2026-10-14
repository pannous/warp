// wiki/string.md, wiki/iteration.md: `for chars in text: print it` walks the text by that unit, the item is `it`
use crate::is;

#[test]
fn a_unit_word_walks_the_text_by_that_unit() {
	is!("t = \"héllo\"; n = 0; for chars in t: n += 1; n", 5);
	is!("t = \"héllo\"; n = 0; for bytes in t: n += 1; n", 6);
	is!("t = \"ab\"; s = 0; for bytes in t { s += it }; s", 195);
	is!("t = \"ab\"; out = \"\"; for chars in t { out = it + out }; out", "ba");
	is!("s = 0; for chars in [5 6] { s += chars }; s", 11);
}

// card each-byte-in-text (wiki/Purpose.md, typed iteration): over a text the loop variable `byte` names the unit, its
// UTF-8 bytes, as `for each char` walks the characters
#[test]
fn the_loop_variable_byte_walks_a_text_by_its_bytes() {
	is!("s = 0; for each byte in \"Hello\" { s += byte }; s", 500);
	is!("t = \"héllo\"; n = 0; for byte in t: n += 1; n", 6);
	is!("out = \"\"; for each char in \"你好\" { out = char + out }; out", "好你");
}
