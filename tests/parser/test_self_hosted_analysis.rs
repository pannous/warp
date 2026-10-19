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

/// WARP_SELF_ANALYSIS beside the switch: analyzer.warp analyzes each lowered program as well, and a program whose kinds
/// agree with src/analyzer's runs as before (card pipeline-runs)
#[test]
fn a_program_the_analyses_agree_on_runs_self_analyzed() {
	let cache = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("self_hosted_stage_cache");
	let run = crate::common::warp_command()
		.env("WARP_SELF_HOSTED", "front")
		.env("WARP_SELF_ANALYSIS", "1")
		.env("WARP_MODULE_CACHE", cache)
		.arg("square(x: int) := x * x; y = square(3); y + 1")
		.output()
		.expect("warp runs");
	let (value, errors) = (String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr));
	assert!(value.contains("10"), "{value} {errors}");
}
