// `job = go f(x)` starts a task, `await job` waits for its value (wiki/async.md); one thread: the task ends where it starts
use warp::is;

#[test]
fn await_gives_the_value_of_a_task() {
	is!("f(x):=x*2; job = go f(3); await job", 6);
	is!("f(x):=x*2; job = go f(3); job + 1", 7); // a task auto-casts to its value
}

#[test]
fn a_program_may_define_its_own_go() {
	is!("go(x) := x + 1; go(2)", 3);
}
