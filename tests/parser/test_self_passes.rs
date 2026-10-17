//! card twin-harness (notes/self_hosting.md step 10): a lowering pass's warp twin gets the program as the Rust passes
//! before it left it, as a node (lib/compiler/walk.warp node_of), and its result is compared, as a view, with the program
//! after the Rust pass (tests/common/twins.rs pass_differential)
use crate::common::twins::{bridge_differential, pass_differential};
use warp::pipeline::around_source_pass;

#[test]
fn a_source_pass_is_seen_before_and_after() {
	let (before, after) =
		around_source_pass("3 times {print 1}", warp::lowering::times_phrases::lower).expect("a source pass");
	assert!(before.serialize().contains("times"), "{}", before.serialize());
	assert!(after.serialize().contains("for times·1 in"), "{}", after.serialize());
	assert!(around_source_pass("1", warp::lowering::inlining::lower).is_none(), "inlining is no source pass");
}

#[test]
fn every_source_is_a_node_and_back() {
	bridge_differential();
}

#[test]
fn times_phrases_match_their_rust_twin() {
	pass_differential(warp::lowering::times_phrases::lower, "times_phrases", "times_phrases");
}

#[test]
fn number_words_match_their_rust_twin() {
	pass_differential(warp::lowering::number_words::lower, "number_words", "number_words_pass");
}
