//! The value of `if c then a else b` has the kind of its branches: two characters give a character, never its code number
use crate::is;

#[test] // samples/game_of_life.wasp: line + (if alive then "█" else " ") appended 9608 and 32
fn test_if_of_characters_joins_a_text() {
	is!("c=1; \"a\" + (if c then \"x\" else \"y\")", "ax");
	is!("c=0; x = if c then \"x\" else \"y\"; \"a\" + x", "ay");
	is!("c=1; \"a\" + (c ? \"x\" : \"yy\")", "ax");
}
