// Run-time `!` (wiki/charged.md §5, notes/runtime_eval.md, package 4). Step 0, user 2026-10-05: "give a compiler
// warning if the code contains unresolved symbol arithmetic": a block over names the program defines nowhere.
use warp::diagnostic::take_warnings;

fn warnings_of(code: &str) -> Vec<String> {
	take_warnings();
	warp::wasm_emitter::eval(code);
	take_warnings().into_iter().map(|warning| warning.message).collect()
}

#[test]
fn a_block_over_names_defined_nowhere_warns() {
	let warnings = warnings_of("x : a+b; x");
	assert!(warnings.iter().any(|message| message.contains("x keeps a+b over a, b, defined nowhere in the program")), "{warnings:?}");
	assert!(warnings_of("x : a*2 > limit; x").iter().any(|message| message.contains("over a, limit")));
}

#[test]
fn a_block_over_defined_names_does_not_warn() {
	assert!(warnings_of("a=1; x : a+b; b=2; x!").iter().all(|message| !message.contains("defined nowhere")));
	assert!(warnings_of("x : a+1; a=5; x!").iter().all(|message| !message.contains("defined nowhere")));
}

// Step 2: a block known only at run time runs through the host (run_block), natively
#[test]
fn interpret_runs_a_block_known_only_at_run_time() {
	warp::is!("xs = [data a+1, data a*2]; a = 5; interpret(xs#2)", 10);
	warp::is!("y = data 6*7; interpret y", 42);
	warp::is!("n=3; t=\"ab\"; ys = [data n+1, data t+\"c\"]; interpret(ys#2)", "abc");
}

#[test]
fn interpret_of_a_constant_block_runs_it_where_written() {
	warp::is!("a=1; x : a+b; b=2; interpret x", 3);
}

#[test]
fn a_run_time_block_reads_the_values_where_it_runs() {
	warp::is!("xs = [data a, data 0]; a = 1; r = interpret(xs#1); a = 2; r", 1);
}

#[test]
fn a_failing_run_time_block_names_the_block() {
	crate::common::fails_with("y = data q+1; interpret y", "the block q+1 failed: undefined variable: q");
}
