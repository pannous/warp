//! The self-hosted pipeline's stage program (parser.warp and the pass twins) compiles once and is kept in the module
//! cache: the next run of warp with WARP_SELF_HOSTED loads it instead of compiling it again (seconds), so it lowers only
//! its own program
const CACHE_FOLDER: &str = "self_hosted_stage_cache";

fn self_hosted_run(cache: &std::path::Path, code: &str) -> (String, String) {
	let run = crate::common::warp_command()
		.env("WARP_SELF_HOSTED", "1")
		.env("WARP_MODULE_CACHE", cache)
		.env("WARP_PHASE_TIMES", "1")
		.arg(code)
		.output()
		.expect("warp runs");
	(String::from_utf8_lossy(&run.stdout).into_owned(), String::from_utf8_lossy(&run.stderr).into_owned())
}

#[test]
fn the_stage_program_is_compiled_once() {
	let cache = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(CACHE_FOLDER);
	let (first, _) = self_hosted_run(&cache, "3 + 4");
	assert!(first.contains('7'), "{first}");
	let (again, phases) = self_hosted_run(&cache, "3 + 4");
	assert!(again.contains('7'), "{again}");
	assert!(phases.contains("lowering, all (1×)"), "only the program is lowered: {phases}");
}

/// emitter.warp emits a program inside its core, so the Rust emitter never runs; one beyond it falls back to Rust
#[test]
fn the_warp_emitter_emits_inside_its_core() {
	let cache = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(CACHE_FOLDER);
	let (inside, phases) = self_hosted_run(&cache, "square(x: int) := x * x; print(square(5)); square(3)");
	assert!(inside.contains("25") && inside.contains('9'), "{inside}");
	assert!(!phases.contains("emit ("), "the Rust emitter ran: {phases}");
	let (beyond, phases) = self_hosted_run(&cache, "class Point { x: int }; p = Point(3); p.x + 4");
	assert!(beyond.contains('7'), "{beyond}");
	assert!(phases.contains("emit ("), "a class is beyond the core: {phases}");
}
