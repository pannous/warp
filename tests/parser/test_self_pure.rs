//! The pure lane (card pure-runs): WARP_SELF_HOSTED=pure compiles a program with parser.warp, the warp passes,
//! analyzer.warp and emitter.warp alone; each program it compiled writes "pure: ran" to WARP_EMIT_REFUSALS
use std::path::PathBuf;

const RAN_PURE: &str = "pure: ran";

/// The program's output and what the lane recorded for it
fn pure_run(name: &str, code: &str) -> (String, String) {
	lane_run("pure", name, code)
}

/// The program's output in the lane WARP_SELF_HOSTED names, and what the lane recorded for it
fn lane_run(lane: &str, name: &str, code: &str) -> (String, String) {
	let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
	let record = scratch.join(format!("pure_{name}.txt"));
	let _ = std::fs::remove_file(&record);
	let run = crate::common::warp_command()
		.env("WARP_SELF_HOSTED", lane)
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

/// WARP_SELF_HOSTED=pure-only: no usual compile behind the lane, a program outside the core fails with its refusal
#[test]
fn pure_only_refuses_without_the_rust_compile() {
	assert!(lane_run("pure-only", "only_core", "1 + 2").0.contains('3'));
	let (output, _) = lane_run("pure-only", "only_refused", "nosuchword 3");
	assert!(output.contains("outside the core: pure: the unknown name nosuchword"), "{output}");
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

/// A call passing the result of a recursive call not known yet, and functions calling each other while one is still
/// read, as parser.warp's expression, atom and continued do
#[test]
fn recursive_results_run_pure() {
	let passed_on =
		"def expression(p, depth) { if depth > 2 { return p }; continued(p, expression(p, depth + 1), depth) }";
	runs_pure(
		"passed_on",
		&format!("{passed_on}; def continued(p, left, depth) := left + p * depth; expression(3, 0)"),
		"12",
	);
	let mutual =
		"def expression(p, depth) { if depth > 2 { return p }; left = atom(p, depth); continued(p, left, depth) }";
	let atom = "def atom(p, depth) := if depth > 1 then p else expression(p, depth + 1)";
	runs_pure(
		"mutual",
		&format!("{mutual}; {atom}; def continued(p, left, depth) := left + p * depth; expression(3, 0)"),
		"12",
	);
}

/// parser.warp reads a name glued to `!` as one word when the source defines it, and a tab between items of a line
/// as a column separator (card parser-reads-2)
#[test]
fn bang_words_and_tab_columns_run_pure() {
	runs_pure(
		"bang_words",
		"def is!(a, b) := a == b\ndef check!(x) := x > 2\ny = 3\n[is!(3, 3), is!(2, 3), check!(5), y != 4]",
		"[yes no yes yes]",
	);
	runs_pure("tab_columns", "row = ['0-1'\t-1]; count(row)", "2");
}
