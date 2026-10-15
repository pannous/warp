//! Card age-name: a map of one entry is held as its pair at run time (card single-entry), yet a map has no `name`, `key`,
//! `value` or `op` of its own: those read its field like a map of more entries, and miss it the same way
use crate::is;

#[test]
fn a_one_entry_map_misses_a_field_like_a_bigger_map() {
	crate::common::fails_with("x={age:2, k:3}; x.name", "no field name");
	crate::common::fails_with("x={age:2}; x.name", "no field name");
	crate::common::fails_with("x={age:2}; x.key", "no field key");
	crate::common::fails_with("x={age:2}; name of x", "no field name");
	crate::common::fails_with("{age:2}.value", "no field value");
}

#[test]
fn a_one_entry_map_keeps_its_fields() {
	is!("x={age:2}; x.age", 2);
	is!("x={name:\"a\"}; x.name", "a");
	is!("x={key:1}; x.key", 1);
}

#[test]
fn a_pair_keeps_its_parts() {
	is!("x=(age:2); x.name", "age");
	is!("x=(age:2); x.value", 2);
}
