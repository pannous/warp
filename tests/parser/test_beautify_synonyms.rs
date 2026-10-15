//! card warp-format-strict: `warp beautify`'s rule "synonyms canonical" (decision warp-format): a word the compiler
//! notes as another language's word for a warp word becomes the warp word, by the note's "I meant" fix
use warp::formatting::{beautify, format, Options};

fn beautified(code: &str) -> String {
	beautify(code).expect("it parses").text
}

#[test]
fn test_foreign_words_become_the_warp_words() {
	assert_eq!(beautified("auto x = 3\nx\n"), "var x = 3\nx\n");
	assert_eq!(beautified("auto x = 3 // three\nx\n"), "var x = 3 # three\nx\n");
}

#[test]
fn test_format_keeps_the_words() {
	let foreign = "auto x = 3\nx\n";
	assert_eq!(format(foreign, Options::default()).expect("it parses").text, foreign);
}
