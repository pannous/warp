//! P52 (user, 2026-10-05): a phrase-defined function is called with its own prepositions: `add 1 to 2`, `square of 4`.
//! Everywhere else `to` stays a range and `of` a field lookup.
use warp::is;

#[test]
fn test_to_phrases_are_called_with_their_prepositions() {
	is!("to add number a to number b: a+b; add 1 to 2", 3);
	is!("to add number a to number b: a+b; add 1+1 to 2*3", 8);
	is!("to move x from a to b: x+a*b; move 1 from 2 to 3", 7);
	is!("to add number a to number b: a+b; add(1, 2)", 3); // the call form stays
}

#[test]
fn test_spaced_phrases_are_called_with_their_prepositions() {
	is!("square of a number = it*it; square of 4", 16);
	is!("square of a number = it*it; square 4", 16);
}

#[test]
fn test_prepositions_keep_their_meaning_elsewhere() {
	is!("to add number a to number b: a+b; xs = 1 to 3; count(xs)", 3);
	is!("p={name:\"Joe\"}; name of p", "Joe");
}
