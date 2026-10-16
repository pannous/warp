//! card floor-division: `/!` spells floor division, `7 /! 2` is 3, and `x /!= 2` divides in place; it never reads
//! as a comment the way a spaced `//` does
use crate::is;

#[test]
fn slash_bang_is_floor_division() {
	is!("7 /! 2", 3);
	is!("7/!2", 3);
	is!("-7 /! 2", -4);
	is!("x = 7; x /! 2 + 1", 4);
	is!("x = 7; x /!= 2; x", 3);
}

#[test]
fn not_equal_and_negation_stay() {
	is!("x = 3; x != 3", false);
	is!("!yes", false);
}
