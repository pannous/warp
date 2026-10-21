//! card exact-pow-warp: int `^` in lib/compiler/emitter.warp is runtime.warp's exact_pow over int_pow, and the module
//! gives Rust's values: powers by squaring, a negative exponent an exact rational
use crate::common::twins::emitter_differential;
use std::path::PathBuf;

const POWERS: &str = "probes/self_hosting/emitter/core/power.warp";

#[test]
fn int_powers_run_as_rust_runs_them() {
	crate::requires!(crate::common::RUST_PIPELINE);
	let inside = emitter_differential(&[PathBuf::from(POWERS)], 1);
	assert_eq!(inside, [POWERS]);
}
