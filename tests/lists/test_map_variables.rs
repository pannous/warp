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

// card values-keys: keys and values of a big map collect its entries in a loop (one call per entry exhausted the
// call stack near ten thousand)
#[test]
fn keys_and_values_of_a_big_map() {
	is!("m = {}; for i in 1 to 100000 { m[i] = i * 2 }; count(keys(m)) + count(values(m))", 200000);
	is!("m = {}; for i in 1 to 100000 { m[i] = i * 2 }; values(m)#100000 + count(keys(m))", 300000);
	is!("m = {a: 1, b: 2}; keys(m)#2 + \"\" + values(m)#2", "b2");
}
