// English phrasings of samples/natural.warp (card natural-phrases)
use crate::is;

/// operator.md lists `mod modulo %`: `modulo` is the word `mod`
#[test]
fn modulo_is_mod() {
	is!("10 modulo 3", 1);
	is!("x = -7; x modulo 3", 2);
	is!("ten modulo three", 1);
}

/// a function word the parser reads as a prefix operator is a sort key too
#[test]
fn sort_by_a_prefix_function() {
	is!("numbers = [3, -5, 2]; numbers.sort by abs", warp::ints(vec![2, 3, -5]));
	is!("numbers = [3, -5, 2]; numbers sorted by abs", warp::ints(vec![2, 3, -5]));
	is!("numbers = [9, 1, 4]; numbers sorted by sqrt", warp::ints(vec![1, 4, 9]));
}

/// a number word's hint names where the word is written, not where the parser read another source last
#[test]
fn number_word_hint_is_where_it_is_written() {
	warp::normalize::clear_shown_hints();
	let (_, hints) = warp::normalize::capture_hints(|| warp::wasm_emitter::eval("x = 1\ny = two"));
	let two = hints.iter().find(|hint| hint.original == "two").expect("a hint for two");
	assert!(two.position.starts_with("2:"), "{}", two.position);
}
