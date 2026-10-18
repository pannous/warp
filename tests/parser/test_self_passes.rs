//! card twin-harness (notes/self_hosting.md step 10): a lowering pass's warp twin gets the program as the Rust passes
//! before it left it, as a node (lib/compiler/walk.warp node_of), and its result is compared, as a view, with the program
//! after the Rust pass (tests/common/twins.rs pass_differential)
use crate::common::twins::{bridge_differential, pass_differential};
use warp::pipeline::around_source_pass;

/// The sources of sql and sh templates, which the interpolation twin leaves out (notes/self_hosting_dropped.md)
pub(super) const SQL_AND_SH: [&str; 4] = [
	"probes/docs_sync/sandbox_exec.warp",
	"probes/sandbox/escape.warp",
	"probes/sandbox/exec.warp",
	"samples/system/crontab.warp",
];
/// The sources of `(function f).dump`, whose function-reference mark the bridge drops (notes/self_hosting_dropped.md)
pub(super) const FUNCTION_REFERENCES: [&str; 1] = ["probes/joinable_error.warp"];
/// Where scope.warp is cleaner than the Rust passes' own name walks (notes/self_hosting_dropped.md, Deviations)
pub(super) const SCOPE_DEVIATIONS: [&str; 0] = [];

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
	pass_differential(warp::lowering::times_phrases::lower, "times_phrases", "times_phrases", &[]);
}

#[test]
fn number_words_match_their_rust_twin() {
	pass_differential(warp::lowering::number_words::lower, "number_words", "number_words_pass", &[]);
}

#[test]
fn interpolation_matches_its_rust_twin() {
	pass_differential(warp::text::interpolation::lower_program, "interpolation", "interpolation", &SQL_AND_SH);
}

#[test]
fn match_arms_match_their_rust_twin() {
	pass_differential(warp::lowering::welcome_forms::lower_match_arms, "welcome_forms", "match_arms", &[]);
}

#[test]
fn arrow_result_types_match_their_rust_twin() {
	pass_differential(
		warp::lowering::welcome_forms::lower_arrow_result_types,
		"welcome_forms",
		"arrow_result_types",
		&[],
	);
}

#[test]
fn of_lookups_match_their_rust_twin() {
	pass_differential(warp::lowering::of_lookups::lower, "of_lookups", "of_lookups", &[]);
}

#[test]
fn grouped_receivers_match_their_rust_twin() {
	pass_differential(
		warp::lowering::grouped_receivers::lower,
		"grouped_receivers",
		"grouped_receivers",
		&FUNCTION_REFERENCES,
	);
}

#[test]
fn element_wise_operators_match_their_rust_twin() {
	pass_differential(
		warp::lowering::broadcasting::lower_element_wise_operators,
		"broadcasting",
		"element_wise_operators_pass",
		&[],
	);
}
