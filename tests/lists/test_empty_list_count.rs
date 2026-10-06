//! ø is the empty list (Footguns.md → Null): it has no elements

use crate::is;

#[test]
fn the_count_of_an_empty_list_is_zero() {
	is!("xs=[]; size xs", 0);
	is!("xs=[]; count(xs)", 0);
}

#[test]
fn a_loop_over_an_empty_list_does_not_run() {
	is!("xs=[]; n=0; for x in xs {n=n+1}; n", 0);
}

#[test]
fn a_one_element_list_still_counts_one() {
	is!("xs=[7]; size xs", 1);
}

#[test]
fn the_counting_properties_of_an_empty_list_need_no_null_check() {
	for property in ["size", "count", "length", "number"] {
		is!(&format!("xs=[]; xs.{property}"), 0);
	}
	is!("xs=[]; xs.size + 1", 1);
}

#[test]
fn a_counting_property_is_a_number_in_arithmetic() {
	is!("xs=[4 5 6]; xs.size + 1", 4);
	is!("xs=[4 5 6]; 2 * xs.count", 6);
	is!("t='héllo'; t.length - 1", 4);
}
