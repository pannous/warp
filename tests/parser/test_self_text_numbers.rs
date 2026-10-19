//! card card-4: texts as numbers at run time in lib/compiler/runtime.warp (text_as_int, text_as_float, text_as_number,
//! Rust's in src/wasm_emitter): emitter.warp compiles `t as int|float|number` of a text to them, and the module's run
//! gives the value Rust's gives
use crate::common::twins::{emitter_differential, sources};
use std::path::PathBuf;

const TEXT_NUMBER_PROGRAMS: &str = "probes/self_hosting/emitter/text_numbers";

#[test]
fn texts_as_numbers_run_as_rust_runs_them() {
	let programs: Vec<PathBuf> = sources().into_iter().filter(|path| path.starts_with(TEXT_NUMBER_PROGRAMS)).collect();
	assert_eq!(programs.len(), 3);
	let inside = emitter_differential(&programs, programs.len());
	assert_eq!(inside.len(), programs.len());
}
