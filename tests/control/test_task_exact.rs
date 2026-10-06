// Exact numbers beyond the fixnums (ratios, big integers) cross to a task and back (card g-rH6E): composed from fixnum
// pieces in the receiving instance (tasks.rs Builders), as a run-time block hands them back
#![cfg(feature = "native")] // the browser task path still refuses them
use crate::is;

#[test]
fn a_ratio_crosses_to_a_task_and_back() {
	is!("m(t) := t * 2; job = go m(1/3); await job == 2/3", 1);
	is!("h() := 7/2; job = go h(); z = await job; z * 2", 7);
	is!("f(x) := x; jobs = []; jobs.add(go f(1/3)); jobs.add(go f(2)); sum(await all jobs) == 7/3", 1);
}

#[test]
fn a_big_integer_crosses_to_a_task_and_back() {
	is!("b() := 2^70; job = go b(); await job == 2^70", 1);
	is!("b(x) := x + 1; job = go b(2^70); await job == 2^70 + 1", 1);
	is!("n(x) := 0 - x; job = go n(2^70); await job == 0 - 2^70", 1);
}
