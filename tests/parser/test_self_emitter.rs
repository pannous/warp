//! card emits-wasm (notes/self_hosting.md step 15): lib/compiler/emitter.warp turns the lowered program into a wasm
//! module of its own, whose run gives the value Rust's module gives (tests/common/twins.rs emitter_differential)
use crate::common::twins::{checked_sources, emitter_differential, sources};
use std::path::PathBuf;

const CORE_PROGRAMS: &str = "probes/self_hosting/emitter/core";
/// Programs of the checked share of samples and probes inside the core, at least (24 on 2026-10-17): a floor that only rises
const LEAST_SOURCES_INSIDE: usize = 10;

#[test]
fn core_programs_run_as_rust_runs_them() {
	let programs: Vec<PathBuf> = sources().into_iter().filter(|path| path.starts_with(CORE_PROGRAMS)).collect();
	let inside = emitter_differential(&programs, programs.len());
	assert_eq!(inside.len(), programs.len());
}

#[test]
fn every_source_inside_the_core_runs_as_rust_runs_it() {
	emitter_differential(&checked_sources(), LEAST_SOURCES_INSIDE);
}
