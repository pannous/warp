// P47 (user, 2026-10-05: "jobs are asynchronous by definition; awaiting one must not affect the others"): a job added to
// a list keeps running on its own; only a use that needs a value awaits it; `await all jobs` gives every result
use std::time::{Duration, Instant};
use warp::is;

const OVERLAPPING_LIMIT: Duration = Duration::from_millis(1800);

#[test]
fn a_list_of_jobs_gives_their_results_when_used() {
	let jobs = "f(x) := x*2; jobs = []; for i in 1 to 3 { jobs.add(go f(i)) }; ";
	is!(&format!("{jobs}await all jobs"), warp::ints(vec![2, 4, 6]));
	is!(&format!("{jobs}jobs"), warp::ints(vec![2, 4, 6]));
	is!(&format!("{jobs}await jobs#2"), 4);
	is!(&format!("{jobs}jobs#2 + 1"), 5);
	is!("f(x) := x*2; a = go f(1); b = go f(2); await all [a, b]", warp::ints(vec![2, 4]));
}

#[test]
fn jobs_in_a_list_run_at_the_same_time() {
	let started = Instant::now();
	is!("f(ms) := { sleep(ms); ms }; jobs = []; for i in 1 to 3 { jobs.add(go f(1000)) }; sum(await all jobs)", 3000);
	assert!(started.elapsed() < OVERLAPPING_LIMIT, "three one-second jobs took {:?}", started.elapsed());
}
