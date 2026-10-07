//! P184 (user, 2026-10-07): English number words are numbers (`one plus two` = 3) when no variable of that name
//! exists, with a hint to write the digits instead (card natural-phrases)
use crate::is;
use warp::normalize::{capture_hints, set_hint_mode, HintMode};
use warp::wasm_emitter::eval;

#[test]
fn number_words_are_numbers() {
	is!("one plus two", 3);
	is!("three times four", 12);
	is!("x = twenty; x + zero", 20);
	is!("ninety - ten", 80);
}

#[test]
fn a_variable_of_that_name_wins() {
	is!("one = 5; one plus two", 7);
	is!("f(two) := two * 10; f(3)", 30);
}

#[test]
fn keys_and_texts_stay_words() {
	is!("{one: 1}.one", 1);
	is!("\"one\"", "one");
}

#[test]
fn a_number_word_hints_its_digits() {
	set_hint_mode(HintMode::Always);
	let (_, hints) = capture_hints(|| eval("one plus two"));
	let rewrites: Vec<(String, String)> = hints.into_iter().map(|hint| (hint.original, hint.canonical)).collect();
	assert!(rewrites.contains(&("one".to_string(), "1".to_string())), "{rewrites:?}");
	assert!(rewrites.contains(&("two".to_string(), "2".to_string())), "{rewrites:?}");
}
