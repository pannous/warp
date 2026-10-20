//! card time-primitives: lib/compiler/emitter.warp makes the time nodes of a lowered program's time_of calls, Kind::Time
//! with the info bits of src/time/mod.rs TimeForm, read back as Rust's module gives them (the intrinsics runtime.warp
//! reads them with, time_form … zone_offset, wait for the runtime words of the time cards)
use crate::common::twins::{emitter_differential, sources};
use std::path::PathBuf;

const TIME_PROGRAMS: &str = "probes/self_hosting/emitter/times";

#[test]
fn time_constants_run_as_rust_runs_them() {
	crate::requires!(crate::common::RUST_PIPELINE);
	let programs: Vec<PathBuf> = sources().into_iter().filter(|path| path.starts_with(TIME_PROGRAMS)).collect();
	assert!(!programs.is_empty(), "no programs in {TIME_PROGRAMS}");
	let inside = emitter_differential(&programs, programs.len());
	assert_eq!(inside.len(), programs.len());
}
