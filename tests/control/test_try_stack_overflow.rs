// `try f(args) else Y` of a user function catches the engine's stack overflow ("call stack exhausted"): the call runs
// through the host (guarded_call, natively and in the browser). Running out of fuel stays uncatchable (the runaway guard).
use crate::common::fails_with;
use warp::is;

const DEEP: &str = "down(n) := if n == 0 then 0 else 1 + down(n - 1); ";

#[test]
fn try_catches_the_stack_overflow_of_a_call() {
	is!(&format!("{DEEP}try down(1000000) else 7"), 7);
	is!(&format!("{DEEP}try down(10) else 7"), 10);
}

#[test]
fn the_instance_keeps_working_after_a_caught_overflow() {
	is!(&format!("{DEEP}r = try down(1000000) else 100; r + down(5)"), 105);
	// a global the aborted call wrote keeps what it wrote: nothing is rolled back
	is!("global calls = 0; down(n) := { calls = calls + 1; if n == 0 then 0 else 1 + down(n - 1) }; r = try down(1000000) else 0; calls > 1000", 1);
}

#[test]
fn a_try_without_a_direct_call_keeps_todays_behaviour() {
	fails_with(&format!("{DEEP}try 1 + down(1000000) else 7"), "call stack exhausted");
	is!("try [1 2]#5 else 7", 7);
}

#[test]
#[cfg(feature = "native")]
fn untrusted_code_may_guard_a_call() {
	assert_eq!(warp::wasm_emitter::eval_untrusted("f(x) := x * 2; try f(3) else 7"), warp::Node::int(6));
}


#[test]
fn a_guarded_call_of_no_arguments() {
	is!("f() := 5; try f() else 7", 5);
	is!("f() := [1 2]#5; try f() else 7", 7);
}
