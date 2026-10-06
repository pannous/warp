// A program the standalone runtime cannot carry says so in a few words (card g-13Vk, user: 'Change message to simply
// "tasks need runtime."'): its tasks, shared arrays, run-time blocks or fetch and files by name, any other import as itself
#![cfg(feature = "native")]
use std::path::PathBuf;

fn build_errors(name: &str, source: &str) -> String {
	let source_path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}.warp"));
	std::fs::write(&source_path, source).unwrap();
	let build = crate::common::warp_command().env("WARP_RUNTIME_STUB", crate::common::runtime_stub()).args(["build", "--exe"]).arg(&source_path).output().unwrap();
	assert!(!build.status.success());
	String::from_utf8_lossy(&build.stderr).into_owned()
}

#[test]
fn a_program_with_tasks_says_tasks_need_runtime() {
	let errors = build_errors("standalone_tasks", "twice(x) := x * 2\njob = go twice(21)\nawait job");
	assert!(errors.contains("tasks need runtime."), "{errors}");
	assert!(!errors.contains("task_spawn"), "{errors}");
}

#[test]
fn fetch_says_fetch_and_files_need_runtime() {
	let errors = build_errors("standalone_fetch_needs", "fetch \"https://example.com\"");
	assert!(errors.contains("fetch and files need runtime."), "{errors}");
}
