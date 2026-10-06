//! `b={}` binds the empty block, which evaluates like `{}` itself: ø, the empty list

use warp::wasm_emitter::eval;
use warp::Node;
use crate::eq;

#[test]
fn an_empty_block_can_be_bound() {
	eq!(eval("b={}; b"), Node::Empty);
	eq!(eval("a=[]; b={}; b"), Node::Empty);
}

#[test]
fn a_bound_empty_block_counts_zero() {
	eq!(eval("b={}; count(b)"), eval("0"));
}
