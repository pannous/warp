//! A block holding only an operator (wiki/broadcasting.md's dodgy `[1,2,3] {++}`) is no function: a loud error that
//! names the written form, instead of `Unexpected character '+'` (card operator-block)
use crate::common::fails_with;
use crate::is;

#[test]
fn a_bare_operator_block_names_the_fix() {
	fails_with("[1,2,3] {++}", "{++} is an operator without operands");
	fails_with("map [1,2,3] { * }", "{*} is an operator without operands");
	fails_with("{+}", "map xs {it + 1}");
}

#[test]
fn operators_with_operands_in_blocks_still_work() {
	is!("map [1,2,3] {it+1}", warp::ints(vec![2, 3, 4]));
	is!("x=1; {x++}; x", 2);
}
