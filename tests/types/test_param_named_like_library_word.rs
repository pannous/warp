//! card word-param-list: a parameter named like a library word (size, count) is the parameter wherever it stands,
//! an item of a list included: `[size, 4]` lists two items, no call of size

use crate::is;
use warp::*;

#[test]
fn a_parameter_named_like_a_counting_word_is_a_list_item() {
	is!("def k(size: any): any := [size, 4]; k(3)", ints![3, 4]);
	is!("def c(count: any): any := [count, 4]; c(3)", ints![3, 4]);
	is!("k = (size: int) -> [size, 4]; k(3)", ints![3, 4]);
}

#[test]
fn a_variable_named_like_a_counting_word_prints_in_a_list() {
	is!("size = 3; print([size, 4]); 0", 0);
	is!("def k(size: int) := { print([size, 4]); 0 }; k(3)", 0);
}
