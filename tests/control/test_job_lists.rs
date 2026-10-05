// P47 (user, 2026-10-05: "jobs are asynchronous by definition; awaiting one must not affect the others"): a job added to
// a list keeps running on its own; only a use that needs a value awaits it; `await all jobs` gives every result
use warp::is;

#[test]
fn a_list_of_jobs_gives_their_results_when_used() {
	let jobs = "f(x) := x*2; jobs = []; for i in 1 to 3 { jobs.add(go f(i)) }; ";
	is!(&format!("{jobs}await all jobs"), warp::ints(vec![2, 4, 6]));
	is!(&format!("{jobs}jobs"), warp::ints(vec![2, 4, 6]));
	is!(&format!("{jobs}await jobs#2"), 4);
	is!(&format!("{jobs}jobs#2 + 1"), 5);
	is!("f(x) := x*2; a = go f(1); b = go f(2); await all [a, b]", warp::ints(vec![2, 4]));
}

// the jobs start together, not a second apart one after the other (user, g-qUvY: prove the overlap, not the wall time)
#[test]
fn jobs_in_a_list_run_at_the_same_time() {
	let jobs = "f(ms) := { started = clock(); sleep(ms); started }; jobs = []; for i in 1 to 3 { jobs.add(go f(1000)) }; ";
	is!(&format!("{jobs}starts = await all jobs; if abs(starts#3 - starts#1) < 1000 then 3000 else -1"), 3000);
}

#[test]
fn counting_jobs_does_not_await_them() {
	// the count is there at once: well before the one-second jobs end (clock() is in milliseconds)
	let jobs = "f(ms) := { sleep(ms); ms }; jobs = []; for i in 1 to 3 { jobs.add(go f(1000)) }; ";
	is!(&format!("{jobs}t0 = clock(); n = count(jobs) + #jobs + jobs.size; t1 = clock(); if t1 - t0 < 500 then n else -1"), 9);
	is!(&format!("{jobs}count(jobs) + sum(await all jobs)"), 3003);
}

// the results of awaited jobs are Nodes of run-time kind: an `if` choosing one (as max and min do) keeps that kind
// instead of typing it text, so arithmetic on the choice works
#[test]
fn the_extremes_of_job_results_are_numbers() {
	let jobs = "f(x) := x*2; jobs = []; for i in 1 to 3 { jobs.add(go f(i)) }; results = await all jobs; ";
	is!(&format!("{jobs}max(results) - min(results)"), 4);
	is!(&format!("{jobs}(if count(results) == 0 then 0 else results#1) + 1"), 3);
}
