// `go f(x)` runs on its own thread (user, P33; notes/threads.md step 1): a function taking and giving Ints runs in a new
// instance of the module, its arguments and result copied; any other function runs where it is started
#![cfg(feature = "native")] // threads of the wasmtime runner
use std::time::{Duration, Instant};
use warp::is;

const SLEEPER: &str = "f(ms) := { sleep(ms); ms }; ";
/// Three tasks of a second each: overlapping they take about one second, one after the other three
const OVERLAPPING_LIMIT: Duration = Duration::from_millis(2500);

#[test]
fn tasks_run_at_the_same_time() {
	let started = Instant::now();
	is!(&format!("{SLEEPER}a = go f(1000); b = go f(1000); c = go f(1000); a + b + c"), 3000);
	assert!(started.elapsed() < OVERLAPPING_LIMIT, "three one-second tasks took {:?}", started.elapsed());
}

#[test]
fn await_gives_the_result_of_the_thread() {
	is!("f(x) := x * 2; job = go f(21); await job", 42);
	is!("add(a, b) := a + b; job = go add(40, 2); job + 0", 42);
}

#[test]
fn an_error_in_a_task_is_the_error_of_await() {
	crate::common::fails_with("f(i) := [1,2,3]#i; job = go f(5); await job", "task f: index out of range");
}

#[test]
fn a_task_of_other_values_runs_where_it_starts() {
	is!("g(t) := t + \"!\"; job = go g(\"a\"); await job", "a!");
}

#[test]
fn a_task_nobody_awaits_finishes_before_the_program_ends() {
	assert!(crate::common::printed("f(x) := { sleep(200); puti(x); x }; go f(7); 1").starts_with('7'));
}

#[test]
fn await_binds_its_task_inside_an_expression() {
	is!(&format!("{SLEEPER}a = go f(1); b = go f(2); await a + await b"), 3);
	is!(&format!("{SLEEPER}a = go f(5); x = await a * 2; x"), 10);
}

const SPINNER: &str = "spin(n) := { i = 0; while i < n { i += 1 }; i }; ";

#[test]
fn stop_ends_a_running_task() {
	let started = Instant::now();
	crate::common::fails_with(&format!("{SPINNER}job = go spin(100000000000); stop job; await job"), "task spin: task stopped");
	assert!(started.elapsed() < OVERLAPPING_LIMIT, "stopping took {:?}", started.elapsed());
}

#[test]
fn a_paused_task_waits_for_resume() {
	is!(&format!("{SPINNER}job = go spin(300000); pause job; resume job; await job"), 300000);
	is!(&format!("{SPINNER}job = go spin(300000); job.pause(); job.resume(); job"), 300000);
	// held by the pause, the task is still there to stop after it would have finished
	crate::common::fails_with(&format!("{SPINNER}job = go spin(30000000); pause job; sleep(1500); stop job; await job"), "task stopped");
}
