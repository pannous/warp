//! card float-as-exact-warp: `x as rational` of a run-time float compiles in lib/compiler/emitter.warp, through
//! runtime.warp's float_as_exact over float_digits, and runs as Rust's float_as_exact runs
use crate::common::twins::emitter_differential;
use std::path::PathBuf;

const FLOAT_AS_RATIONAL: &str = "probes/self_hosting/emitter/core/float_as_rational.warp";

#[test]
fn float_as_rational_runs_as_rust_runs_it() {
	crate::requires!(crate::common::RUST_PIPELINE);
	let inside = emitter_differential(&[PathBuf::from(FLOAT_AS_RATIONAL)], 1);
	assert_eq!(inside, [FLOAT_AS_RATIONAL]);
}
