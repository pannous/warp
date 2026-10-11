//! card keys-value: `x.keys` (values, entries) of a value that is no map fails loudly instead of echoing the value
use crate::common::fails_with;
use crate::is;

#[test]
fn the_keys_of_a_value_that_is_no_map_fail() {
	fails_with("x = 3.14; x.keys", "not a map");
	fails_with("x = \"hi\"; x.keys", "not a map");
	fails_with("x = [1 2]; x.values", "not a map");
}

#[test]
fn the_keys_of_a_map_stay_its_keys() {
	is!("x = {a:1 b:2}; x.keys == [\"a\" \"b\"]", true);
	is!("x = {a:1 b:2}; x.values", warp::parse("[1 2]"));
}
