//! `max` and `min` of one list: `max([1 5 2])`, `max(xs)`; several arguments are a list as well

use crate::common::fails_with;
use crate::is;

#[test]
fn test_extremum_of_a_list_literal() {
	is!("max([1 5 2])", 5);
	is!("min([4 2 9])", 2);
	is!("max([1.5 2.5])", 2.5);
	is!("min([7])", 7);
}

#[test]
fn test_extremum_of_a_list_variable() {
	is!("xs=[4 2 9]; max(xs)", 9);
	is!("xs=[4 2 9]; min(xs)", 2);
	is!("xs=[4 2 9]; 1+max(xs)", 10);
	is!("xs=[4 2 9]; max(xs)+min(xs)", 11);
}

#[test]
fn test_several_arguments_still_work() {
	is!("max(3,7)", 7);
	is!("min(3,7,1)", 1);
}

#[test]
fn test_extremum_of_an_empty_list_is_an_error() {
	fails_with("max([])", "max of an empty list");
	fails_with("min([])", "min of an empty list");
	fails_with("xs=[]; max(xs)", "max of an empty list");
	fails_with("xs=[]; min(xs)", "min of an empty list");
}

#[test]
fn test_a_single_number_is_still_refused() {
	fails_with("max(3)", "max takes at least 2");
}

#[test]
fn test_extremum_of_a_list_variable_in_float_arithmetic() {
	is!("xs=[4 2 9]; 0.5+max(xs)", 9.5);
}
