//! The pure lane (card pure-runs): WARP_SELF_HOSTED=pure compiles a program with parser.warp, the warp passes,
//! analyzer.warp and emitter.warp alone; each program it compiled writes "pure: ran" to WARP_EMIT_REFUSALS
use std::path::PathBuf;

const RAN_PURE: &str = "pure: ran";

/// The program's output and what the lane recorded for it
fn pure_run(name: &str, code: &str) -> (String, String) {
	let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
	let record = scratch.join(format!("pure_{name}.txt"));
	let _ = std::fs::remove_file(&record);
	let run = crate::common::warp_command()
		.env("WARP_SELF_HOSTED", "pure")
		.env("WARP_EMIT_REFUSALS", &record)
		.env("WARP_MODULE_CACHE", scratch.join("self_hosted_stage_cache"))
		.arg(code)
		.output()
		.expect("warp runs");
	(String::from_utf8_lossy(&run.stdout).into_owned(), std::fs::read_to_string(&record).unwrap_or_default())
}

fn runs_pure(name: &str, code: &str, value: &str) {
	let (output, record) = pure_run(name, code);
	assert!(output.contains(value), "{code}: {output}");
	assert_eq!(record.trim(), RAN_PURE, "{code}");
}

#[test]
fn the_core_runs_pure() {
	runs_pure("arithmetic", "1 + 2", "3");
	runs_pure("function", "f(x) := x * 2; f(4)", "8");
}

/// `for` lowers to `while` in lib/compiler/for_loops.warp: counted ranges, `to` including its end, walked lists
#[test]
fn for_loops_run_pure() {
	runs_pure("for_range", "s = 0; for i in 0..4 { s += i }; s", "6");
	runs_pure("for_to", "n = 3; t = 0; for i in 1 to n { t += i }; t", "6");
	runs_pure("for_list", "xs = [4, 5, 6]; t = 0; for x in xs { t += x }; t", "15");
	runs_pure("for_nested", "t = 0; for i in 1 to 2 { for j in 1 to 3 { t += i * j } }; t", "18");
}

/// node_matches.warp: an arm of a number, text or character tests the subject equals it
#[test]
fn match_values_run_pure() {
	runs_pure("match_number", "f(x) := match x { 1: 10; 2: 20; _: 0 }; f(2) + f(5)", "20");
	runs_pure("match_text", "g(w) := match w { \"hi\": 1; \"hello\": 2; _: 0 }; g(\"hello\")", "2");
}
