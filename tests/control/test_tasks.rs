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

#[test]
fn a_finished_tasks_handler_runs_and_a_stop_handler_never() {
	is!("job = go 3; once job finishes: x = 7; x", 7);
	is!("download(u) := u + 1; job = go download(1); once the download finishes: y = 5; y + job", 7);
	is!("job = go 3; once job pauses: print \"held\"; await job", 3);
	is!("download(u) := u; job = go download(2); on download.stop : print \"cancelled\"; job", 2);
}

#[test]
fn stopping_a_finished_task_warns_and_changes_nothing() {
	is!("job = go 3; stop job; job", 3);
	is!("job = go 3; job.stop(); job", 3);
}
