//! `if a < n {…}` and `while i < n {…}`: the block is the body, not the argument of the variable n

use warp::{is, parse};

#[test]
fn the_block_after_a_comparison_with_a_variable_is_the_body() {
	is!("n=4; if 3 < n {1} else {2}", 1);
	is!("n=2; if 3 < n {1} else {2}", 2);
	is!("n=4; i=0; while i < n {i++}; i", 4);
	is!("n=4; i=0; while n > i {i++}; i", 4);
	is!("n=4; i=0; while i < n and 1 {i++}; i", 4);
}

#[test]
fn a_condition_still_ends_before_its_block() {
	assert_eq!(parse("while i < n {i++}").to_string(), parse("while i < n do {i++}").to_string());
}
