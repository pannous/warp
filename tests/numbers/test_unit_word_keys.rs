// A unit word that starts the next map entry is a key, not the unit of the number before it: `{w:2 h:3}`
use warp::is;

#[test]
fn a_unit_word_key_after_a_number() {
	is!("r={w:2 h:3}; r.w", 2);
	is!("r={w:2 h:3}; r.h", 3);
	is!("r={len:2 m:3}; r.m", 3);
}
