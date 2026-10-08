// The hash-table maps also start from a map literal, and a parameter every call passes a map is copied into one;
// a list or an instance given where map entries are set keeps the generic way
use crate::is;

#[test]
fn a_map_literal_and_a_map_parameter_are_hash_tables() {
	is!("m = {a:1}; m[\"b\"] = 2; m[\"a\"] + m[\"b\"] + count(m)", 5);
	is!("fill(m, n) := { for i in 0..n { m[\"k\\(i)\"] = i }; count(m) }; fill({}, 300000)", 300000);
	is!("f(m) := { m[\"x\"] = 1; m }; n = {a:1}; x = f(n); count(x) + count(n)", 4);
}

#[test]
fn a_list_given_for_map_entries_keeps_the_generic_way() {
	is!("f(m) := { m[\"x\"] = 1; count(m) }; f([1, 2])", 3);
	is!("p = {x:1}; p.@source = \"gps\"; #p", 1);
}
