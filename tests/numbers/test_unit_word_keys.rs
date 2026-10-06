// A unit word that starts the next map entry is a key, not the unit of the number before it: `{w:2 h:3}`
use crate::is;

#[test]
fn a_unit_word_key_after_a_number() {
	is!("r={w:2 h:3}; r.w", 2);
	is!("r={w:2 h:3}; r.h", 3);
	is!("r={len:2 m:3}; r.m", 3);
}

#[test]
fn a_quantity_held_in_a_variable() {
	is!("x=2 km; x > 1500 m", 1);
	is!("x=2 km; x = x + 500 m; x == 2500 m", 1);
}

#[test]
fn minutes_are_a_time_unit() {
	is!("2 h + 30 min == 150 min", 1);
	is!("1 h == 60 min", 1);
	is!("xs=[3 1 2]; min(xs)", 1);
}
