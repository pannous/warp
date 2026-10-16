// Every sample runs to a value or to an error of the program, never to an error of the compiler (invalid WASM, a cast
// trap, a panic); the graphics samples (raylib, SDL) need a window and are left out. The browser suite runs it too.
// The samples of the playground menu are left to test_playground_samples, whose check (no error at all) is stricter:
// each sample is run once (user 2026-10-16, notes/test_speed.md)
use super::test_playground_samples::playground_menu;
use crate::common::{sample_files, sample_name};
use std::panic::{catch_unwind, AssertUnwindSafe};
use warp::wasm_emitter::eval;

const COMPILER_FAILURES: [&str; 4] = ["internal error", "cast failure", "panicked", "unreachable"];
const NEEDS_A_WINDOW: [&str; 2] = ["raylib", "sdl"];
const MINIMUM_SAMPLES: usize = 50; // guards against an empty directory listing (the browser reads it over HTTP)

/// The samples outside the playground menu
fn runnable_samples() -> Vec<String> {
	let files = sample_files();
	assert!(files.len() >= MINIMUM_SAMPLES, "only {} samples found: {files:?}", files.len());
	let menu = playground_menu();
	let mut names: Vec<String> = files
		.iter()
		.filter(|path| !menu.contains(&sample_name(path)))
		.map(|path| path.to_string_lossy().to_string())
		.filter(|name| name.ends_with(".warp") && !NEEDS_A_WINDOW.iter().any(|word| name.contains(word)))
		.collect();
	names.sort();
	names
}

fn compiler_failure(sample: &str) -> Option<String> {
	let result = match catch_unwind(AssertUnwindSafe(|| eval(sample).serialize())) {
		Ok(result) => result,
		Err(_) => "panicked".to_string(),
	};
	COMPILER_FAILURES.iter().any(|failure| result.contains(failure)).then(|| format!("{sample}: {result}"))
}

fn slice_runs_cleanly(slice: usize) {
	let samples = runnable_samples();
	let failures: Vec<String> =
		crate::common::sample_slice(&samples, slice).filter_map(|sample| compiler_failure(sample)).collect();
	assert!(failures.is_empty(), "compiler errors in samples:\n{}", failures.join("\n"));
}

#[test]
fn every_sample_runs_without_a_compiler_error() {
	slice_runs_cleanly(0);
}

#[test]
fn every_sample_runs_without_a_compiler_error_slice_2() {
	slice_runs_cleanly(1);
}

#[test]
fn every_sample_runs_without_a_compiler_error_slice_3() {
	slice_runs_cleanly(2);
}

#[test]
fn every_sample_runs_without_a_compiler_error_slice_4() {
	slice_runs_cleanly(3);
}
