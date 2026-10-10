// Card map-literal: a map literal with number keys is keyed like `d={}; d[1]="a"` (P34): `m = {1: 5}; m[1] = 2` sets
// the entry 1, and the written key 1 and the text key "1" are one entry
use crate::is;

#[test]
fn a_number_subscript_keys_a_map_literal() {
	is!("m = {1: 5}; m[1] = 2; m[1]", 2);
	is!("m = {1: 5}; m[1] = 2; count(m)", 1);
	is!("m = {1: 5, 2: 6}; m[2]", 6);
	is!("m = {1: 5}; m[3] = 7; count(m)", 2);
}

#[test]
fn number_and_text_keys_of_a_map_literal_agree() {
	is!("m = {1: 5}; m[1] = 2; m[\"1\"]", 2);
	is!("m: map = {1: 5}; m[1] + 1", 6);
}
