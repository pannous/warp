// A range held by a structure (card range-descriptor): an object field, a list item, a list variable inside a list.
// `r: 1..n` and `r = 1..n` are the same range: `..` already says it is a value, no block (user, 2026-10-09)
use crate::is;
use warp::diagnostic::take_warnings;
use warp::ints;

#[test]
fn a_range_after_colon_is_its_value() {
	is!("o = {r: 1..10}; count o.r", 9);
	is!("o = {r: 1..10}; o.r#3", 3);
	is!("r: 1..5; r", ints(vec![1, 2, 3, 4]));
	is!("n = 3; o = {r: n..6}; o.r", ints(vec![3, 4, 5]));
	take_warnings();
	warp::wasm_emitter::eval("o = {r: 1..10}; count o.r");
	assert!(!take_warnings().iter().any(|warning| warning.message.contains("keeps the block")));
}

/// collected once, in linear time: a quadratic collection runs out of fuel
#[test]
fn a_held_range_is_collected_in_linear_time() {
	is!("o = {r = 1..100001}; count o.r", 100000);
	is!("o = {r: 1..100001}; count o.r", 100000);
	is!("xs = [1..100001, 5]; count xs#1", 100000);
	is!("zs = 1..100001; y = [zs, 5]; count y#1", 100000);
}
