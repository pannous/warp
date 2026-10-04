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
