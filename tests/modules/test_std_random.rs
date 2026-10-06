//! The standard library module random (notes/stdlib.md)
use crate::is;
use warp::wasp_parser::parse;

#[test]
fn use_random_brings_choice_shuffle_sample() {
	is!("use random; c = choice([7, 8, 9]); c >= 7 and c <= 9", 1);
	is!("use random; sort(shuffle([3, 1, 2, 5, 4]))", parse("[1 2 3 4 5]"));
	is!("use random; count(sample([1, 2, 3, 4], 2))", 2);
	is!("use random; count(sample([1, 2], 5))", 2);
}
