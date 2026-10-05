// A task runs its function in a fresh instance, values copied in and out, natively on a thread and in the browser
// through host.js (user, P33): the same results everywhere
use warp::is;

#[test]
fn a_task_gives_the_result_of_its_function() {
	is!("f(x) := x * 2; job = go f(21); await job", 42);
	is!("g(t) := t + \"!\"; job = go g(\"a\"); await job", "a!");
	is!("h(x:float) := x / 2; job = go h(3); await job", 1.5);
	is!("total(xs) := { s=0; for x in xs { s+=x }; s }; job = go total([1,2,3]); await job", 6);
	is!("dbl(xs) := xs.map(x => x*2); job = go dbl([1,2]); await job", warp::ints(vec![2, 4]));
}

#[test]
fn the_failure_of_a_task_is_caught_by_try() {
	is!("f(i) := [1,2,3]#i; job = go f(5); try await job else -1", -1);
	crate::common::fails_with("f(i) := [1,2,3]#i; job = go f(5); await job", "task f: index out of range");
}

/// Two tasks of a second each overlap: they start together (on Workers in the browser, which the test server isolates
/// for shared memory), not one after the other a second apart (user, g-qUvY: prove the overlap, not the wall time)
#[test]
fn two_tasks_overlap() {
	is!("f(ms) := { started = clock(); sleep(ms); started }; a = go f(1000); b = go f(1000); if abs(await a - await b) < 1000 then 2000 else -1", 2000);
}

#[test]
fn a_function_value_crosses_to_a_task() {
	is!("inc = x => x + 1; apply(g, v) := g(v); job = go apply(inc, 3); await job", 4);
	is!("k = 10; add = x => x + k; apply(g, v) := g(v); job = go apply(add, 5); await job", 15);
	is!("mapit(f, xs) := xs.map(f); job = go mapit(x => x + 1, [1, 2]); await job", warp::ints(vec![2, 3]));
}

const SPINNER: &str = "spin(n) := { i = 0; while i < n { i += 1 }; i }; ";

#[test]
fn a_task_pauses_and_resumes() {
	is!(&format!("{SPINNER}job = go spin(300000); pause job; resume job; await job"), 300000);
	// held by the pause, the task is still there to stop after it would have finished
	crate::common::fails_with(&format!("{SPINNER}job = go spin(30000000); pause job; sleep(1500); stop job; await job"), "task stopped");
}
