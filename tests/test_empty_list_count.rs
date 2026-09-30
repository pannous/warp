//! ø is the empty list (Footguns.md → Null): it has no elements

use warp::is;

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
