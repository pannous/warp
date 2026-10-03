// `try X else Y` (todo.md r3-try 2026-10-03): leaving X by return/break/continue restores the try depth, so a later
// error keeps its message; `try X else e => …` names the caught error
use warp::is;
use warp::wasm_emitter::eval;

const LATER_ERROR: &str = " + [1 2]#9";
const LATER_MESSAGE: &str = "index out of range";

fn error_text(code: &str) -> String {
	match eval(code) {
		warp::Node::Error(reason) => format!("{reason:?}"),
		other => panic!("{code}: expected an error, got {other:?}"),
	}
}

fn later_error_is_loud(code: &str) {
	let message = error_text(&format!("{code}{LATER_ERROR}"));
	assert!(message.contains(LATER_MESSAGE), "{code}: {message}");
}

#[test]
fn return_out_of_a_try_restores_the_depth() {
	is!("f(i) := { try { return [10 20]#i } else 0 }; f(1)", 10);
	later_error_is_loud("f(i) := { try { return [10 20]#i } else 0 }; f(1)");
	later_error_is_loud("f(i) := { try { try { return [10 20]#i } else 0 } else 1 }; f(2)");
}

#[test]
fn break_out_of_a_try_restores_the_depth() {
	is!("s=0; for i in [1 2 3] { try { if i==2 {break}; s=s+1 } else 0 }; s", 1);
	later_error_is_loud("s=0; for i in [1 2 3] { try { if i==2 {break}; s=s+1 } else 0 }; s");
	later_error_is_loud("s=0; i=0; while i < 5 { i=i+1; try { if i==3 {break} } else 0 }; s");
}

#[test]
fn continue_out_of_a_try_restores_the_depth() {
	is!("s=0; for i in [1 2 3] { try { if i==2 {continue}; s=s+1 } else 0 }; s", 2);
	later_error_is_loud("s=0; for i in [1 2 3] { try { if i==2 {continue}; s=s+1 } else 0 }; s");
}

#[test]
fn a_jump_inside_a_try_keeps_the_try_catching() {
	// the loop sits inside the try: break leaves the loop, not the try, so the error after it is still caught
	is!("try { for i in [1 2 3] { if i==2 {break} }; [1 2]#9 } else 7", 7);
}

#[test]
fn the_else_branch_names_a_caught_runtime_error() {
	is!("try 1 + [1 2]#5 else e => e", "index out of range");
	is!("try [1 2]#5 else e => e", "index out of range");
	is!("try 1 + [1 2]#5 else e => \"failed: \" + e", "failed: index out of range");
	is!("f(i) := [1 2]#i; try f(7) + 1 else problem => problem", "index out of range");
}

#[test]
fn the_else_branch_names_an_error_value() {
	is!("try error(\"boom\") else e => e", "boom");
}

#[test]
fn a_named_else_is_skipped_without_an_error() {
	is!("try 1 + [1 2]#2 else e => 0", 3);
}

#[test]
fn bare_try_else_still_works() {
	is!("try 1 + [1 2]#5 else 0", 0);
	is!("try [1 2]#5 else 4", 4);
}
