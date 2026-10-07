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

/// `seed(n)`: the same numbers after the same seed, natively and in the browser (the same xorshift64*)
#[test]
fn use_random_seed_repeats_the_numbers() {
	is!("use random; seed(42); [random_below(100), random_below(100)]", warp::ints(vec![15, 64]));
	is!("use random; seed(42); a = shuffle([1, 2, 3, 4, 5]); seed(42); a == shuffle([1, 2, 3, 4, 5])", true);
}
