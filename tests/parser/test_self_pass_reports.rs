//! card carries-line (notes/self_hosting.md step 12): a pass twin gets the program with the line:column of its nodes
//! where the Rust pass reports, and its hints are compared with the Rust pass's, position included
//! (tests/common/twins.rs pass_differential, lib/compiler/walk.warp report; first twin: number_words, in test_self_passes)
use crate::common::twins::positioned_bridge_differential;
use crate::is;
use warp::pipeline::around_source_pass_reporting;

#[test]
fn a_source_pass_is_seen_with_its_hints() {
	let (_, after, hints) =
		around_source_pass_reporting("x = 2\ny = one + x", warp::lowering::number_words::lower).expect("a source pass");
	assert!(after.serialize().contains("1+x"), "{}", after.serialize());
	let hint = hints.first().expect("the number word hints its digits");
	assert_eq!((hint.line, hint.column, hint.preferred.as_deref()), (2, 5, Some("1")), "{hint:?}");
}

#[test]
fn every_positioned_source_is_a_node_and_back() {
	positioned_bridge_differential();
}

#[test]
fn a_built_node_carries_its_position() {
	is!("n = positioned(symbol(\"x\"), 3, 7)\n[n.line, n.column]", warp::ints(vec![3, 7]));
	is!("n = positioned(key(symbol(\"a\"), \":\", 1), 2, 1)\nkey_of(n) == symbol(\"a\")", true);
}
