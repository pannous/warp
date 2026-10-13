// card hint-data-word: a number word in quoted data stays the word (`data [1 "two" three]`), with no hint to write 3
use crate::is;
use warp::normalize::{capture_hints, set_hint_mode, HintMode};
use warp::wasm_emitter::eval;

#[test]
fn a_number_word_in_quoted_data_stays_a_word() {
	is!("data [1 three]", warp::parse("[1 three]"));
	set_hint_mode(HintMode::Always);
	let (_, hints) = capture_hints(|| eval("mixed = data [1 \"two\" three 4.0]\nmixed"));
	assert!(hints.iter().all(|hint| hint.canonical != "3"), "{hints:?}");
	let (_, hints) = capture_hints(|| eval("three + 1"));
	assert!(hints.iter().any(|hint| hint.canonical == "3"), "{hints:?}");
}
