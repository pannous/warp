// Every sample runs to a value or to an error of the program, never to an error of the compiler (invalid WASM, a cast
// trap, a panic); the graphics samples (raylib, SDL) need a window and are left out. The browser suite runs it too.
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use warp::wasm_emitter::eval;

const COMPILER_FAILURES: [&str; 4] = ["internal error", "cast failure", "panicked", "unreachable"];
const NEEDS_A_WINDOW: [&str; 2] = ["raylib", "sdl"];
const MINIMUM_SAMPLES: usize = 50; // guards against an empty directory listing (the browser reads it over HTTP)

fn runnable_samples() -> Vec<String> {
	let folder = crate::common::samples_run_folder();
	let mut names: Vec<String> = fs::read_dir(folder)
		.expect("samples/")
		.filter_map(|entry| entry.ok()?.file_name().into_string().ok())
		.filter(|name| name.ends_with(".warp") && !NEEDS_A_WINDOW.iter().any(|word| name.contains(word)))
		.map(|name| folder.join(name).to_string_lossy().into_owned())
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
	assert!(samples.len() >= MINIMUM_SAMPLES, "only {} samples found: {samples:?}", samples.len());
	let failures: Vec<String> =
		crate::common::sample_slice(&samples, slice).filter_map(|sample| compiler_failure(sample)).collect();
	assert!(failures.is_empty(), "compiler errors in samples:\n{}", failures.join("\n"));
}

/// card samples-store: samples/store.warp kept its values in samples/store.stored.json of the checkout
#[cfg(feature = "native")]
#[test]
fn samples_keep_their_stored_values_out_of_the_checkout() {
	let kept_in_checkout = || fs::read("samples/store.stored.json").ok();
	let before = kept_in_checkout();
	let folder = crate::common::samples_run_folder();
	let outcome = eval(&folder.join("store.warp").to_string_lossy()).serialize();
	assert_eq!(kept_in_checkout(), before, "a sample run changed samples/store.stored.json of the checkout: {outcome}");
	assert!(folder.join("store.stored.json").exists(), "the run keeps its values beside the copy it ran: {outcome}");
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
