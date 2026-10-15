//! card of-in-chain: `in` and `from` select like `of` when a selector word stands before them, right associative:
//! `length of characters in text` is length(text.chars); `x in xs` stays the membership test
use crate::is;

#[test]
fn a_selector_in_a_value() {
	is!("characters in \"hello\"", warp::warp_parser::parse("['h' 'e' 'l' 'l' 'o']"));
	is!("words in \"ab cd\"", warp::warp_parser::parse("[\"ab\" \"cd\"]"));
	is!("(keys in {a: 1, b: 2}) == [\"a\" \"b\"]", true);
	is!("words from \"ab cd\"", warp::warp_parser::parse("[\"ab\" \"cd\"]"));
}

#[test]
fn selectors_chain_to_the_right() {
	is!("length of characters in \"hello\"", 5);
	is!("length of words in \"hello world\"", 2);
	is!("t = \"a b c\"; count of words in t", 3);
	is!("number of words in \"a b c\"", 3);
	is!("x = length of words from \"a b\"; x", 2);
	is!("first of words in \"ab cd\"", "ab");
}

#[test]
fn characters_is_the_word_chars() {
	is!("\"abc\".characters", warp::warp_parser::parse("['a' 'b' 'c']"));
}

#[test]
fn in_stays_membership_elsewhere() {
	is!("2 in [1 2 3]", true);
	is!("words = [\"a\"]; \"a\" in words", true);
	is!("words = 3; words in [1 2 3]", true);
	is!("x = {y: [1 2]}; 2 in y of x", true);
	is!("out = \"\"; for words in [\"x\" \"y\"]: out = out + words; out", "xy");
}

#[test]
#[cfg(feature = "native")]
fn printing_a_selection_prints_each_part() {
	assert!(crate::common::printed("print words in \"ab cd\"").starts_with("ab\ncd\n"));
	assert!(crate::common::printed("print characters in \"hi\"").starts_with("h\ni\n"));
	assert!(crate::common::printed("print all words in \"ab cd\"").starts_with("ab\ncd\n"));
}

/// decision 2026-10-15: letters are the alphabetic characters only
#[test]
fn letters_are_the_alphabetic_characters() {
	is!("(letters of \"a1 b!\") == ['a' 'b']", true);
	is!("length of letters in \"héllo Ω1\"", 6);
}
