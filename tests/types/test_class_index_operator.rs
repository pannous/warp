// A class overloads the index operator `#` (card class-overloads, decision music-plain-details: `F#4` is the note F
// sharp in octave 4 by the index of the note object F): `index(n)` or `#(n)` in the class body
use crate::is;

#[test]
fn a_class_defines_the_index_operator() {
	is!("class P{n:int; index(o) := n * 100 + o}; F = P(5); F#4", 504);
	is!("class P{n:int; #(o) := n * 100 + o}; F = P(5); F#4", 504);
	is!("class P{n:int; operator #(o) := n * 100 + o}; F = P(5); F#4 + 1", 505);
}

#[test]
fn an_index_of_a_class_without_the_operator_is_its_field() {
	is!("class P{n:int; m:int}; F = P(5, 7); F#2", 7);
	is!("class P{n:int; index(o) := n + o}; xs = [1, 2, 3]; xs#2", 2);
}
