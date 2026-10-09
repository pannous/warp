//! card g_oOJs: `dir(task)` of a task variable was "undefined function: dir", and its read waited for the task. It lists
//! what a task takes: `await task`, the controls (`stop task`, `task.pause()`) and the event `once task finishes`
use crate::is;

const TASK: &str = "f(id) := { sleep(10 ms); id }\ntask = go f(7)\n";
const TASK_WORDS: [&str; 6] = ["await", "stop", "pause", "cancel", "resume", "finishes"];

#[test]
fn dir_of_a_task_lists_its_words() {
	is!(&format!("{TASK}dir(task)"), warp::texts(TASK_WORDS.to_vec()));
}

#[test]
fn the_task_still_runs_after_its_dir() {
	is!(&format!("{TASK}names = dir(task)\nawait task"), 7);
}
