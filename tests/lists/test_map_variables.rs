// A map variable built by its own entries (`m = {}`, then `m[k] = v` with name keys) is a hash table: setting and
// finding an entry no longer copies and walks the map, so two million keys run (map_replace_entry's recursion
// exhausted the call stack near there) and the map still reads as the Node its entries make
use crate::is;
use warp::*;

const MILLIONS_OF_KEYS: &str = "m = {}; for i in 0..2000000 { m[\"k\\(i)\"] = i }; s = 0; for i in 0..2000000 { s += m[\"k\\(i)\"] }; s + count(m)";

#[test]
fn two_million_keys_are_set_and_found() {
	is!(MILLIONS_OF_KEYS, 1999999000000i64 + 2000000);
}

#[test]
fn a_map_variable_reads_as_the_node_of_its_entries() {
	is!("m = {}; m", Node::Empty);
	is!("m = {}; m[\"a\"] = 1; m[\"b\"] = 2; m[\"a\"] = 3; m == {a:3, b:2}", true);
	is!("m = {}; m[\"a\"] = 1; n = m; m[\"b\"] = 2; n == {a:1, b:2}", true);
	is!("m = {}; m[\"a\"] = 1; m.a + count(m)", 2);
}

#[test]
fn a_missing_key_of_a_map_variable_is_the_same_error() {
	crate::common::fails_with("m = {}; m[\"a\"] = 1; m[\"zz\"]", "no field zz");
}
