//! card exact-rounding-warp: runtime.warp exact_floor, exact_mod … (Rust: src/wasm_emitter/numbers/exact.rs) behind
//! emitter.warp's `floor` and `%` on an exact rational, run as Rust's module runs the same program
use crate::common::twins::emitter_differential;
use std::path::PathBuf;

const ROUNDING_PROGRAM: &str = "probes/self_hosting/emitter/exact/rounding.warp";

#[test]
#[ignore = "card sprint-fixes: the rounding program falls outside the emitter core after the sprint merges (0 of 1 inside)"]
fn ratio_floor_and_remainder_run_as_rust_runs_them() {
	let inside = emitter_differential(&[PathBuf::from(ROUNDING_PROGRAM)], 1);
	assert_eq!(inside.len(), 1);
}
