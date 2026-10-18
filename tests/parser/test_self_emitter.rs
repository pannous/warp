//! card emits-wasm (notes/self_hosting.md step 15): lib/compiler/emitter.warp turns the lowered program into a wasm
//! module of its own, whose run gives the value Rust's module gives (tests/common/twins.rs emitter_differential)
use crate::common::twins::{emitter_differential, sources, twin_sources};
use std::path::PathBuf;

const CORE_PROGRAMS: &str = "probes/self_hosting/emitter/core";
/// Programs of the checked share of samples and probes inside the core, at least (24 on 2026-10-17): a floor that only rises
const LEAST_SOURCES_INSIDE: usize = 100;
/// Sources whose lowered view lacks what Rust's result depends on, named in notes/self_hosting_dropped.md
const DROPPED: [&str; 3] = [
	"probes/variance/runtime_lax_alias.warp",
	"probes/variance/runtime_unannotated_param.warp",
	"probes/static_result.warp",
];

#[test]
fn core_programs_run_as_rust_runs_them() {
	let programs: Vec<PathBuf> = sources().into_iter().filter(|path| path.starts_with(CORE_PROGRAMS)).collect();
	let inside = emitter_differential(&programs, programs.len());
	assert_eq!(inside.len(), programs.len());
}

#[test]
fn every_source_inside_the_core_runs_as_rust_runs_it() {
	emitter_differential(&twin_sources(&DROPPED), LEAST_SOURCES_INSIDE);
}
