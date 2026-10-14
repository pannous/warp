//! cards type-module, abc-chars (user 2026-10-14: "s.capitalize without () it's always exactly the same. Do not have
//! special logic for the same code path"; "\"abc\".chars should give ['a','b','c']"): `x.word` is `x.word()`, for the
//! words of the type modules (lib/list.warp, text.warp) as for the builtins, and `x.chars` is the characters, not their count
use crate::is;

#[test]
fn a_type_module_word_needs_no_parentheses() {
	is!("xs = [1 2 3]; xs.product", 6);
	is!("xs = [1 2 3]; xs.product == xs.product()", true);
	is!("s = \"ab\"; s.capitalize", "Ab");
}

#[test]
fn chars_are_the_characters_with_or_without_parentheses() {
	is!("\"abc\".chars", warp::texts(vec!["a", "b", "c"]));
	is!("s = \"abc\"; s.chars == s.chars()", true);
	is!("\"abc\".chars.count", 3);
}
