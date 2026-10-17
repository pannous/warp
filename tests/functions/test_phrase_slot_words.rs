// card phrase-multiword: in a slot after a preposition, a word the body never uses is part of the phrase, the slot's
// last word its parameter: `with subject title` is called `with subject "hi"`
use crate::is;

#[ignore = "cleanup-edge: cleanup-park-phrases: phrase forms parked until card prepositions-call"]
#[test]
fn an_unused_word_after_a_preposition_belongs_to_the_phrase() {
	is!(
		r#"to mail x to address with subject title { "\(address): \(title)" }; mail 1 to "a@b" with subject "hi""#,
		"a@b: hi"
	);
	is!(r#"to email address with subject text { text }; email "a@b" with subject "Hello""#, "Hello");
}

#[ignore = "cleanup-edge: cleanup-park-phrases: phrase forms parked until card prepositions-call"]
#[test]
fn used_words_stay_parameters() {
	is!("to add number a to number b: a+b; add 1 to 2", 3);
	is!("to f x y: y; f 1 2", 2);
}

#[ignore = "cleanup-edge: cleanup-park-phrases: phrase forms parked until card prepositions-call"]
#[test]
fn the_phrases_sample_runs() {
	is!("samples/hello/phrases.warp", "hi for a@b: see you");
}
