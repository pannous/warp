#![cfg(feature = "native")]

use crate::is;

#[test]
fn queued_and_nested_tasks_finish_on_the_fixed_pool() {
	let workers = warp::host::tasks::worker_count() as i64;
	let tasks = workers * 2 + 1;
	let program = format!(
		"shared total = 0; inner(x) := x; outer(x) := {{ value = await go inner(x); total += value; 0 }}; jobs = []; for i in 0..{tasks} {{ jobs.add(go outer(i)) }}; await all jobs; total"
	);
	is!(&program, tasks * (tasks - 1) / 2);
}
