//! P46 leftovers (user, 2026-10-05): a built-in type word filters a loop too (`for number in xs`), and an adjective with a
//! type word is a condition (`for (even number) in xs`, wiki/for.md); both announce the filter (got-it topic for-filter)
use warp::diagnostic::take_warnings;
use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn test_a_type_word_loop_visits_only_items_of_that_type() {
	is!("xs=[1, \"a\", 2.5, \"b\"]; n=0; for number in xs { n += 1 }; n", 2);
	is!("xs=[1, \"a\", 2.5, \"b\"]; s=\"\"; for text in xs { s += text }; s", "ab");
	is!("xs=[1, \"a\", 3]; s=0; for int in xs: s += it\ns", 4);
}

#[test]
fn test_a_literal_list_of_matching_items_needs_no_filter() {
	take_warnings();
	is!("s=0; for number in [1,2,3] { s += number }; s", 6);
	assert!(take_warnings().iter().all(|warning| !warning.message.contains("visits only")));
}

#[test]
fn test_an_adjective_with_a_type_word_is_a_condition() {
	is!("s=0; for (even number) in [1,2,3,4] { s += it }; s", 6);
	is!("s=0; for (odd number) in [1,2,3,4]: s += it\ns", 4);
	is!("big(x) := x > 2; s=0; for (big number) in [1,2,3,4] { s += it }; s", 7);
	take_warnings();
	eval("s=0; for (even number) in [1,2,3,4] { s += it }; s");
	assert!(take_warnings().iter().any(|warning| warning.message.contains("visits only")));
}

#[test]
fn test_a_text_appended_with_an_item_of_a_mixed_list_inside_an_if() {
	is!("xs=[1, \"a\"]; s=\"\"; y=xs#2; if 1 { s += y }; s", "a"); // was a WASM validation failure
}

#[test]
fn test_the_codepoints_of_a_text_need_no_filter() {
	take_warnings();
	is!("s=\"\"; for character in chars(\"abc\")[1:] { s += character }; s", "bc");
	assert!(take_warnings().iter().all(|warning| !warning.message.contains("visits only")));
}
