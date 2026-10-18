//! Card function-returning: a function returning a number on one path and a list or a text on another returns a Node
//! of run-time kind, which keeps whichever value the run gives (probes/analyzer/mixed_results.warp)
use warp::wasm_emitter::eval;

fn text_of(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_number_or_a_list_is_known_at_run_time() {
	let counted = "def counted(n) { if n > 0 { return n }; [1] }\n";
	assert_eq!(text_of(&format!("{counted}counted(1) is number")), "yes");
	assert_eq!(text_of(&format!("{counted}counted(0) == [1]")), "yes");
	assert_eq!(text_of("def halved(n) { if n > 0 { return 2.5 }; [1] }\nhalved(1) + 1"), "3.5");
}

#[test]
fn a_number_or_a_text_is_known_at_run_time() {
	assert_eq!(text_of("def named(n) { if n > 0 { return \"x\" }; 2 }\nnamed(0) + 1"), "3");
}
