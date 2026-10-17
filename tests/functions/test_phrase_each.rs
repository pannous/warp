// card phrase-each: `each` separates a phrase's slots like a preposition
use crate::is;

#[ignore = "cleanup-edge: cleanup-park-phrases: phrase forms parked until card prepositions-call"]
#[test]
fn each_in_a_phrase() {
	is!("to f xs each d: d\nx = f [1] each 50ms\nx == 50ms", true);
	is!("to total xs each step: step * #xs\ntotal [1, 2] each 3", 6);
}

#[ignore = "cleanup-edge: cleanup-park-phrases: phrase forms parked until card prepositions-call"]
#[test]
fn assigned_phrase_call() {
	is!("to f xs by d: d\nx = f [1] by 5\nx", 5);
}

#[ignore = "cleanup-edge: cleanup-park-phrases: phrase forms parked until card prepositions-call"]
#[test]
fn assigned_to_phrase() {
	is!("to add a to b: a+b; y = add 1 to 2; y", 3);
}

#[ignore = "cleanup-edge: cleanup-park-phrases: phrase forms parked until card prepositions-call"]
#[test]
fn assigned_to_phrase_compared() {
	is!("to add a to b: a+b; y = add 1 to 2 == 3; y", true);
}
