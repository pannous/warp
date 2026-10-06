// A repeat count held in a variable must be whole: `y=2.5; y times "ab"` is a runtime error, not ø or ""
use crate::common::fails_with;
use warp::ints;
use crate::is;

#[test]
fn a_fractional_count_in_a_variable_is_an_error() {
	fails_with("y=2.5; y times \"ab\"", "count must be an integer");
	fails_with("y=2.5; y times [1]", "count must be an integer");
}

#[test]
fn a_whole_count_in_a_variable_repeats() {
	is!("y=2; y times [1]", ints(vec![1, 1]));
	is!("y=5/2; try y times [1] else 7", 7);
}
