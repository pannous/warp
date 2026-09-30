//! Ordering a list against a number is an error value (equality stays false)

mod common;
use common::fails_with;
use warp::is;

#[test]
fn ordering_a_list_with_a_number_is_a_loud_error() {
	for code in ["x=[1 2]; x > 1", "x=[1 2]; x < 1", "x=[1 2]; 1 < x", "[1 2] >= 1"] {
		fails_with(code, "compare");
	}
}

#[test]
fn a_list_is_not_equal_to_a_number() {
	is!("x=[1 2]; x == 1", false);
}
