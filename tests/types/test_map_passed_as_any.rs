// card map-passed: a slot typed any takes a one-entry map as an untyped one does (card single-entry): `{a:1}` given
// to an `x: any` parameter or held in a `z: any` variable is the map of its entry
use crate::is;

#[test]
fn a_one_entry_map_in_an_any_slot_is_a_map() {
	is!("f(n: any): bool := n is map; f({a: 1})", true);
	is!("f(n: any): text := type(n).name; f({a: 1})", "map");
	is!("f(n: any): bool := n is map; y = {a: 1}; f(y)", true);
	is!("z: any = {a: 1}; z is map", true);
	is!("f(n: any): bool := n is pair; f((a: 1))", true);
}
