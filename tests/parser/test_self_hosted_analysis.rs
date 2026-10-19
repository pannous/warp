//! The self-hosted analysis stage (card End-end): analyzer.warp analyzes each lowered program beside src/analyzer
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
