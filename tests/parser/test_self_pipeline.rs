//! card parser-tree (notes/self_hosting.md step 13): the warp pipeline so far, lib/compiler/passes.warp `lowered`:
//! parser.warp's tree, then the pass twins in pipeline order, against the Rust parse after the same Rust passes, over
//! the sources parser.warp parses as Rust does
use super::test_self_parser::PARSED_SOURCES;
use crate::common::twins::chain_differential;
use std::path::PathBuf;

#[test]
fn the_warp_pipeline_lowers_as_rust() {
	let paths: Vec<PathBuf> = std::fs::read_to_string(PARSED_SOURCES).unwrap().lines().map(PathBuf::from).collect();
	let rust_passes = [
		warp::text::interpolation::lower_program,
		warp::lowering::times_phrases::lower,
		warp::lowering::number_words::lower,
	];
	chain_differential(&paths, "passes", "lowered", &rust_passes);
}
