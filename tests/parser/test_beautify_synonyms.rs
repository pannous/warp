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

// card format-beautify: `o-b` of two variables parses as one name but lowers to the subtraction, so beautify spaces
// it; format keeps it, a hyphenated name stays
#[test]
fn test_beautify_spaces_a_minus_that_lowers_the_same() {
	assert_eq!(beautified("o = 5\nb = 2\nc = o-b\nprint c\n"), "o = 5\nb = 2\nc = o - b\nprint c\n");
	let kept = "o = 5\nb = 2\nc = o-b\nprint c\n";
	assert_eq!(format(kept, Options::default()).expect("it parses").text, kept);
	let named = "full-name = 3\nprint full-name\n";
	assert_eq!(beautified(named), named);
}

// reviewer: a line whose reading is an assumption stays as written: `f it - 1` would read as f(it) - 1
#[test]
fn test_beautify_leaves_an_assumed_reading_alone() {
	let assumed = "f(x) := x*2\nit = 5\nprint f it-1\n";
	assert_eq!(beautified(assumed), "f(x) := x * 2\nit = 5\nprint f it-1\n");
}
