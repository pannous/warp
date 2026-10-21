//! card nested-write: a write into a map held in a map or in an instance held in a map changes the inner one, the key
//! literal or a variable (nested_index keeps a pure index as written, so the emitter reads it as the map key)
use crate::is;

#[test]
fn a_write_into_a_map_held_in_a_map() {
	is!("m = {}; m[\"a\"] = {}; m[\"a\"][\"x\"] = 1; #m[\"a\"]", 1);
	is!("m = {a: {}}; m[\"a\"][\"x\"] = 1; m[\"a\"][\"x\"]", 1);
	is!("m = {}; k = \"a\"; m[k] = {}; m[k][\"x\"] = 2; m[k][\"x\"]", 2);
}

#[test]
fn a_write_into_a_field_of_an_instance_held_in_a_map() {
	is!("class S { kinds: map = {} }; scopes = {}; k = \"a\"; scopes[k] = S(); scopes[k].kinds[\"x\"] = 1; #scopes[k].kinds", 1);
}

#[test]
fn a_list_index_of_arithmetic_changes_the_inner_list() {
	is!("xs = [[1, 2], [3]]; i = 0; xs[i + 1][0] = 9; xs[1][0] + xs[0][1]", 11);
}
