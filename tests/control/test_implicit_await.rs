// A task used as its value waits for it where it is read (user, 2026-10-06: "using a task as a value blocks silently:
// we should definitely give a strong warning"): warned, unless the program says `await`
use crate::common::fails_with;
use crate::is;
use warp::diagnostic::{with_warning_mode, WarningMode};

const SLOW: &str = "f(x) := { sleep(10 ms); x * 2 }; ";

#[test]
fn a_task_read_as_its_value_warns_that_it_waits() {
	with_warning_mode(WarningMode::Error, || fails_with(&format!("{SLOW}job = go f(3); job + 1"), "waits"));
	with_warning_mode(WarningMode::Error, || fails_with(&format!("{SLOW}jobs = []; jobs.add(go f(1)); starts = jobs; starts"), "waits"));
	is!(&format!("{SLOW}job = go f(3); job + 1"), 7);
}

#[test]
fn an_awaited_task_does_not_warn() {
	with_warning_mode(WarningMode::Error, || is!(&format!("{SLOW}job = go f(3); await job + 1"), 7));
	with_warning_mode(WarningMode::Error, || is!(&format!("{SLOW}jobs = []; jobs.add(go f(1)); await all jobs"), warp::ints(vec![2])));
	with_warning_mode(WarningMode::Error, || is!(&format!("{SLOW}job = go f(3); try await job else 0"), 6));
}
