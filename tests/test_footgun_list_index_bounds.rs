mod common;
use common::fails_with;

const INDEX_ERROR: &str = "index out of range";

#[test]
fn test_list_index_past_end_is_an_error() {
	fails_with("x=[1 2 3]; x[3]", INDEX_ERROR);
}

#[test]
fn test_selector_zero_is_an_error() {
	fails_with("x=[1 2 3]; x#0", INDEX_ERROR);
}

#[test]
fn test_negative_bracket_index_is_an_error() {
	fails_with("x=[1 2 3]; x[-1]", INDEX_ERROR);
}
