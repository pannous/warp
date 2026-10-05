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

#[test]
fn texts_floats_characters_and_lists_cross_to_a_task() {
	is!("g(t) := t + \"!\"; job = go g(\"a\"); await job", "a!");
	is!("g(t) := t + \"!\"; a = go g(\"x\"); b = go g(\"y\"); await a + await b", "x!y!");
	is!("p(t, n) := t * n; job = go p(\"ab\", 3); await job", "ababab");
	is!("h(x:float) := x / 2; job = go h(3); await job", 1.5);
	is!("q(t) := [t, t]; job = go q(2); await job", warp::ints(vec![2, 2]));
}

#[test]
fn an_exact_number_crosses_to_a_task() {
	is!("m(t) := t + 1; job = go m(2.5); await job", 3.5);
}

#[test]
fn a_task_of_a_list_parameter_runs_where_it_starts() {
	is!("k(xs) := count(xs); job = go k([1,2,3]); await job", 3);
}

#[test]
fn try_catches_the_failure_of_a_task() {
	is!("f(i) := [1,2,3]#i; job = go f(5); try await job else -1", -1);
	is!(&format!("{SPINNER}job = go spin(100000000000); stop job; try await job else -1"), -1);
	is!("g(t) := t + \"!\"; job = go g(\"a\"); try await job else \"failed\"", "a!");
}

#[test]
fn lists_cross_to_a_task() {
	is!("total(xs) := { s=0; for x in xs { s+=x }; s }; job = go total([1,2,3]); await job", 6);
	is!("sum2(xs) := { s=0; for i in 0..#xs { s+=xs#(i+1) }; s }; job = go sum2([1,2,3]); await job", 6);
	is!("dbl(xs) := xs.map(x => x*2); job = go dbl([1,2]); await job", warp::ints(vec![2, 4]));
	is!("f(xs, n) := xs#1 * n; job = go f([3,4], 2); await job", 6);
	is!("total(xs) := { s=0; for x in xs { s+=x }; s }; big=(0..20000).map(x=>x); a = go total(big); b = go total(big); await a + await b", 399980000);
}

#[test]
fn a_finish_handler_runs_after_the_statement_that_saw_the_task_finish() {
	is!(&format!("{SLEEPER}job = go f(300); x = 0; once job finishes: x = 7; sleep(600); x"), 7);
	// not finished yet when y is set; the end of the block waits for the task and runs the handler
	is!(&format!("{SLEEPER}job = go f(1000); x = 0; once job finishes: x = 7; y = x; y"), 0);
	is!(&format!("{SLEEPER}job = go f(200); x = 0; once job finishes: x = 7; z = 1; x"), 7);
	is!(&format!("{SLEEPER}job = go f(100); done = 0; once job finishes: done = done + 1; for i in 1 to 3 {{ sleep(100) }}; done"), 1);
}

#[test]
fn a_stop_handler_runs_once_the_task_stopped() {
	is!(&format!("{SPINNER}job = go spin(100000000000); stopped = 0; on job.stop: stopped = 1; stop job; sleep(300); stopped"), 1);
}

#[test]
fn function_values_cross_to_a_task() {
	is!("inc = x => x + 1; apply(g, v) := g(v); job = go apply(inc, 3); await job", 4);
	is!("k = 10; add = x => x + k; apply(g, v) := g(v); job = go apply(add, 5); await job", 15);
	is!("twice(f, v) := f(f(v)); job = go twice(x => x * 3, 2); await job", 18);
	is!("mapit(f, xs) := xs.map(f); job = go mapit(x => x + 1, [1, 2]); await job", warp::ints(vec![2, 3]));
}
