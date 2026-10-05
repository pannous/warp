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
