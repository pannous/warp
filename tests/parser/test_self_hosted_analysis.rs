//! The self-hosted analysis stage (card End-end): analyzer.warp `kinds` of a lowered program, as the JSON data the
//! Rust analyzer's self-hosted hand-off maps
use warp::compiler::node_views::plain_view;
use warp::compiler::self_hosted::kinds_of;

#[test]
fn the_analysis_stage_gives_the_kinds_as_data() {
	let program = warp::pipeline::lower("square(x: int) := x * x; y = square(3); \"square: ${y}\"").expect("lowers");
	let kinds = kinds_of(&plain_view(&program)).expect("the analysis stage answers");
	assert_eq!(kinds["functions"]["square"]["params"], serde_json::json!(["int"]), "{kinds}");
	assert_eq!(kinds["functions"]["square"]["result"], "int", "{kinds}");
	assert_eq!(kinds["variables"]["y"], "int", "{kinds}");
	assert_eq!(kinds["value"], "text", "{kinds}");
}

fn self_analyzed_run(switch: &str, code: &str) -> (String, String) {
	let cache = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("self_hosted_stage_cache");
	let run = crate::common::warp_command()
		.env("WARP_SELF_HOSTED", switch)
		.env("WARP_SELF_ANALYSIS", "1")
		.env("WARP_MODULE_CACHE", cache)
		.env("WARP_PHASE_TIMES", "1")
		.arg(code)
		.output()
		.expect("warp runs");
	(String::from_utf8_lossy(&run.stdout).into_owned(), String::from_utf8_lossy(&run.stderr).into_owned())
}

/// WARP_SELF_ANALYSIS beside the switch: analyzer.warp analyzes each lowered program as well, and a program whose kinds
/// agree with src/analyzer's runs as before (card pipeline-runs)
#[test]
fn a_program_the_analyses_agree_on_runs_self_analyzed() {
	let (value, errors) = self_analyzed_run("front", "square(x: int) := x * x; y = square(3); y + 1");
	assert!(value.contains("10"), "{value} {errors}");
}

/// emitter.warp takes an unannotated parameter's kind from analyzer.warp, not from the first call: `h(xs#1)` passes a
/// node, yet h's n is an int, so `n % 3` stays inside the core (card emitter-reads)
#[test]
fn the_warp_emitter_reads_the_analyzed_parameter_kinds() {
	self_analyzed_run("1", "x = 3; x + 4"); // the stage programs compile with the Rust emitter, once
	let (value, phases) = self_analyzed_run("1", "h(n) := n % 3; xs = [4, 5]; h(xs#1)");
	assert!(value.contains('1'), "{value}");
	assert!(!phases.contains("emit ("), "the Rust emitter ran: {phases}");
}
