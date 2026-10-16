//! card count-rs (topic self-hosted): `count x in y` calls lib/prelude.warp's count_item and count_of instead of warp
//! source held in Rust strings; every spelling counts as before, count_of is a word of every program
use crate::is;

#[test]
fn every_count_spelling_calls_the_prelude_words() {
	is!(r#"count "an" in "banana""#, 2);
	is!("count 3 in [1, 3, 3]", 2);
	is!(r#"count(["a", "b", "a"], "a")"#, 2);
	is!(r#""banana".count("a")"#, 3);
	is!("xs = [1, 2, 2]; xs.count(2)", 2);
	is!(r#"f(t) := t.count("an"); f("banana")"#, 2);
	// a comprehension's element is split off after the loader picked the prelude words
	is!(r#"[count "an" in t for t in ["banana", "an"]]#1"#, 2);
}

#[test]
fn count_of_is_a_word_of_every_program() {
	is!(r#"count_of("banana", "an")"#, 2);
	is!(r#""banana".count_of("an")"#, 2);
	is!(r#"use text; count_of("banana", "an")"#, 2);
}
