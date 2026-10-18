//! The self-hosted pipeline's stage program (parser.warp and the pass twins) compiles once and is kept in the module
//! cache: the next run of warp with WARP_SELF_HOSTED loads it instead of compiling it again (seconds), so it lowers only
//! its own program
const CACHE_FOLDER: &str = "self_hosted_stage_cache";

fn self_hosted_run(cache: &std::path::Path) -> (String, String) {
	let run = crate::common::warp_command()
		.env("WARP_SELF_HOSTED", "1")
		.env("WARP_MODULE_CACHE", cache)
		.env("WARP_PHASE_TIMES", "1")
		.arg("3 + 4")
		.output()
		.expect("warp runs");
	(String::from_utf8_lossy(&run.stdout).into_owned(), String::from_utf8_lossy(&run.stderr).into_owned())
}

#[test]
fn the_stage_program_is_compiled_once() {
	let cache = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(CACHE_FOLDER);
	let (first, _) = self_hosted_run(&cache);
	assert!(first.contains('7'), "{first}");
	let (again, phases) = self_hosted_run(&cache);
	assert!(again.contains('7'), "{again}");
	assert!(phases.contains("lowering, all (1×)"), "only the program is lowered: {phases}");
}
