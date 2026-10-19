//! Static units: a function with unit literals passed as a value keeps a plain version of its own name, and an item of
//! a single quantity, in a branch for lists only, leaves the rest of the program's units checked
use crate::is;

#[test]
fn a_function_with_units_passes_as_a_value() {
	// its value crosses as the SI amount, without units
	is!("half() := 1s / 2\ncall_it(f: function) := f()\nround(call_it(half) * 10)", 5);
}

#[test]
fn an_item_of_a_single_quantity_stops_no_unit_checks() {
	is!("first_or(x) := if x is list then x#1 else x\nfirst_or(3m) == 3m", true);
}
