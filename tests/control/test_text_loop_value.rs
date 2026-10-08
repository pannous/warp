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

#[test]
fn a_text_loop_value_stays_a_text_nested_assigned_and_returned() {
	is!("for x in [1 2] { for y in [\"a\"] { y + x } }", "a2");
	is!("f(xs) := for x in xs { x + \"!\" }; f([\"a\" \"b\"])", "b!");
	is!("x = for c in \"ab\" { c + 1 }; x + \"!\"", "b1!");
	is!("f(n) := for i in 1..n { i * 2 }; f(3)", 4);
}
