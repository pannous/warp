// card entry-number, decision map-subscript-runtime: a number subscript of a value of no known type keys a map by its
// digits and stays a position on a list, decided when the program runs (P34 at run time)
use crate::is;

#[test]
fn a_number_subscript_keys_a_map_given_to_a_function() {
	is!("f(m) := m[1] + 1; f({1: 5, 2: 6})", 6);
	is!("f(m) := m[1] + 1; f({1: 5})", 6);
	is!("f(m) := m[1] + 1; f([5, 6])", 7);
	is!("f(m, k) := m[k]; f({1: 5, 2: 6}, 2)", 6);
	is!("f(m, k) := m[k]; f([5, 6], 0)", 5);
}

#[test]
fn a_one_entry_map_changed_by_a_method_keeps_its_number_key() {
	is!("m = {1: 5}; m.add({2: 6}); m[1]", 5);
}
