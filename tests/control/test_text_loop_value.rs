//! The value of a loop is its last body value (P55), a text, character or list one too: never the count of passes or
//! a code point (card loop-value)
use crate::is;
use warp::*;

#[test]
fn a_loop_ending_in_a_text_gives_that_text() {
	is!("s = \"\"; for c in \"ab\" { s += c }", "ab");
	is!("for c in [\"a\"] { c + 1 }", "a1");
	is!("s = \"\"; i = 0; while i < 2 { i++; s += \"a\" }", "aa");
}

#[test]
fn a_loop_ending_in_a_character_gives_that_character() {
	is!("for c in [\"a\"] { \"x\" }", 'x');
	is!("for c in \"ab\" { c }", 'b');
	is!("for c in \"ab\" { if c == \"b\" { break }; c }", 'a');
}

#[test]
fn a_text_loop_that_never_ran_is_empty() {
	is!("for c in [] { \"x\" }", Empty);
}

#[test]
fn numeric_loops_keep_their_values() {
	is!("for i in 1..3 { i * 2 }", 4);
	is!("n = 0; for c in \"ab\" { n += 1 }; n", 2);
}
