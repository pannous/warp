// Every sample of the playground menu (samples/*.wasp but those web/playground/excluded_samples.txt names) runs to a
// value, never to an error (card g-_fHI): an excluded sample is back in the menu once it works and its line is deleted.
// The browser suite runs it too.
use std::fs;
use warp::wasm_emitter::eval;
use warp::Node;

const EXCLUDED_LIST: &str = "web/playground/excluded_samples.txt";
const SAMPLES: &str = "samples";

/// The names excluded_samples.txt lists: the first word of each line that is no comment
fn excluded_samples() -> Vec<String> {
	fs::read_to_string(EXCLUDED_LIST).expect(EXCLUDED_LIST).lines()
		.filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
		.filter_map(|line| line.split_whitespace().next().map(str::to_string))
		.collect()
}

fn sample_path(name: &str) -> String {
	format!("{SAMPLES}/{name}.wasp")
}

#[test]
fn every_playground_sample_runs_without_an_error() {
	let excluded = excluded_samples();
	let mut menu: Vec<String> = fs::read_dir(SAMPLES).expect(SAMPLES)
		.filter_map(|entry| entry.ok()?.path().file_name()?.to_str()?.strip_suffix(".wasp").map(str::to_string))
		.filter(|name| !excluded.contains(name))
		.collect();
	menu.sort();
	let failures: Vec<String> = menu.iter().filter_map(|name| match eval(&sample_path(name)) {
		failed @ Node::Error(_) => Some(format!("{name}: {}", failed.serialize())),
		_ => None,
	}).collect();
	assert!(failures.is_empty(), "playground samples with an error (fix them or list them in {EXCLUDED_LIST}):\n{}", failures.join("\n"));
}

#[test]
fn every_excluded_sample_exists() {
	let missing: Vec<String> = excluded_samples().into_iter().filter(|name| fs::metadata(sample_path(name)).is_err()).collect();
	assert!(missing.is_empty(), "{EXCLUDED_LIST} names samples that are gone: {missing:?}");
}
