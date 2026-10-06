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
