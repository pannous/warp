//! P52 (user, 2026-10-05): a phrase-defined function is called with its own prepositions: `add 1 to 2`, `square of 4`.
//! Everywhere else `to` stays a range and `of` a field lookup.
use crate::is;

#[ignore = "cleanup-edge: cleanup-park-phrases: phrase forms parked until card prepositions-call"]
#[test]
fn test_to_phrases_are_called_with_their_prepositions() {
	is!("to add number a to number b: a+b; add 1 to 2", 3);
	is!("to add number a to number b: a+b; add 1+1 to 2*3", 8);
	is!("to move x from a to b: x+a*b; move 1 from 2 to 3", 7);
	is!("to add number a to number b: a+b; add(1, 2)", 3); // the call form stays
}

#[ignore = "cleanup-edge: cleanup-park-phrases: phrase forms parked until card prepositions-call"]
#[test]
fn test_spaced_phrases_are_called_with_their_prepositions() {
	is!("square of a number = it*it; square of 4", 16);
	is!("square of a number = it*it; square 4", 16);
}

#[ignore = "cleanup-edge: cleanup-park-phrases: phrase forms parked until card prepositions-call"]
#[test]
fn test_prepositions_keep_their_meaning_elsewhere() {
	is!("to add number a to number b: a+b; xs = 1 to 3; count(xs)", 3);
	is!("p={name:\"Joe\"}; name of p", "Joe");
}

// card get-phrase-result: a phrase starting with an HTTP method word is the call, not a route (routes write their
// path as text: `get "/api" {…}`)
#[ignore = "cleanup-edge: cleanup-park-phrases: phrase forms parked until card prepositions-call"]
#[test]
fn a_get_phrase_is_a_call() {
	is!("to get cents of coin: coin * 2; get cents of 3", 6);
}
