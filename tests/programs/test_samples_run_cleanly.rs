// Every sample runs to a value or to an error of the program, never to an error of the compiler (invalid WASM, a cast
// trap, a panic); the graphics samples (raylib, SDL) need a window and are left out. The browser suite runs it too.
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use warp::wasm_emitter::eval;

const COMPILER_FAILURES: [&str; 4] = ["internal error", "cast failure", "panicked", "unreachable"];
const NEEDS_A_WINDOW: [&str; 2] = ["raylib", "sdl"];
const MINIMUM_SAMPLES: usize = 50; // guards against an empty directory listing (the browser reads it over HTTP)

fn runnable_samples() -> Vec<String> {
	let mut names: Vec<String> = fs::read_dir("samples").expect("samples/")
		.filter_map(|entry| entry.ok().map(|entry| entry.path().to_string_lossy().to_string()))
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

#[test]
fn every_sample_runs_without_a_compiler_error() {
	let samples = runnable_samples();
	assert!(samples.len() >= MINIMUM_SAMPLES, "only {} samples found: {samples:?}", samples.len());
	let failures: Vec<String> = samples.iter().filter_map(|sample| compiler_failure(sample)).collect();
	assert!(failures.is_empty(), "compiler errors in samples:\n{}", failures.join("\n"));
}
