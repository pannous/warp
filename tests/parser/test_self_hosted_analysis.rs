//! The self-hosted analysis stage (card End-end): analyzer.warp analyzes each lowered program beside src/analyzer

/// insertion sort and merge sort, merge growing its result from ø
const SORTING_SAMPLE: &str = "samples/lists/sorting_idiomatic.warp";

fn self_analyzed_run(switch: &str, code: &str) -> (String, String) {
	let cache = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("self_hosted_stage_cache");
	let run = crate::common::warp_command()
		.env("WARP_SELF_HOSTED", switch)
		.env("WARP_SELF_ANALYSIS", "1")
		.env("WARP_MODULE_CACHE", cache)
		.arg(code)
		.output()
		.expect("warp runs");
	(String::from_utf8_lossy(&run.stdout).into_owned(), String::from_utf8_lossy(&run.stderr).into_owned())
}

/// WARP_SELF_ANALYSIS beside the switch: analyzer.warp analyzes each lowered program as well, and the program runs as
/// before (card pipeline-runs)
#[test]
fn a_program_the_analyses_agree_on_runs_self_analyzed() {
	let (value, errors) = self_analyzed_run("front", "square(x: int) := x * x; y = square(3); y + 1");
	assert!(value.contains("10"), "{value} {errors}");
}

/// With the warp emitter reading analyzer.warp's kinds, a parameter first called with a list item and a local grown
/// from ø into a list give their values (card emitter-reads)
#[test]
fn programs_emitted_with_the_analyzed_kinds_give_their_values() {
	let (value, errors) = self_analyzed_run("1", "h(n) := n % 3; xs = [4, 5]; h(xs#1)");
	assert!(value.contains('1'), "{value} {errors}");
	let (value, errors) = self_analyzed_run("1", SORTING_SAMPLE);
	assert!(value.contains("-3,0,1,2,5,5,6,7,8,9"), "{value} {errors}");
}
