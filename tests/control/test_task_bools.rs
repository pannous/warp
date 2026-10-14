// A bool a task gives stays a bool in the starting program (card bool-crossing): it crosses as 1/0 and is marked
// a bool again where it is awaited
use warp::wasm_emitter::eval;

fn text_of(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn an_awaited_bool_is_a_bool() {
	assert_eq!(text_of("f(x) := x > 2; job = go f(3); await job"), "yes");
	assert_eq!(text_of("f(x) := x > 2; job = go f(1); await job"), "no");
	assert_eq!(eval("f(x) := x > 2; job = go f(3); type(await job)"), "bool");
}

#[test]
fn bools_of_a_job_list_are_bools() {
	assert_eq!(text_of("f(x) := x > 2; await all [go f(3), go f(1)]"), "[yes no]");
	assert_eq!(text_of("f(x) := x > 2; jobs = []; jobs.add(go f(3)); jobs.add(go f(1)); await all jobs"), "[yes no]");
}

#[test]
fn a_bool_given_through_a_value_task_is_a_bool() {
	assert_eq!(text_of("f(x) := x == \"a\"; job = go f(\"a\"); await job"), "yes");
}
