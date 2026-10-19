//! card sort-warp: lib/compiler/runtime.warp sorts as Rust's list_sort and list_sorted_by do (stable bottom-up merge
//! sort, node_order), checked by running the sorting probes through emitter.warp next to Rust's module
use crate::common::twins::emitter_differential;
use std::path::PathBuf;

const SORTING_PROGRAM: &str = "probes/self_hosting/emitter/core/sorting.warp";

#[test]
fn sort_runs_as_rust_sorts() {
	let inside = emitter_differential(&[PathBuf::from(SORTING_PROGRAM)], 1);
	assert_eq!(inside, vec![SORTING_PROGRAM.to_string()]);
}
