// `on exit {…}` (card g-3Gdo, notes/system_signals.md): the handler runs once when the program ends, after main
// returns or at `exit(code)`; the exit code stays the program's
#![cfg(all(unix, feature = "native"))]
use crate::common::{printed, warp_command};

fn lines(code: &str) -> Vec<String> {
	printed(code).lines().filter(|line| !line.starts_with('»')).map(str::to_string).collect()
}

#[test]
fn on_exit_runs_when_main_returns() {
	assert_eq!(lines("on exit { print \"bye\" }\nprint \"hello\""), ["hello", "bye"]);
}

#[test]
fn on_exit_runs_at_exit_and_keeps_its_code() {
	assert_eq!(lines("n = 1\non exit { print \"bye \" + n }\nn = 2\nexit(3)\nprint \"never\""), ["bye 2"]);
	let file = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("on_exit_code.wasp");
	std::fs::write(&file, "on exit { print \"bye\" }\nexit(3)\n").expect("write the program");
	let output = warp_command().args(["run", file.to_str().unwrap()]).output().expect("warp runs");
	assert_eq!(String::from_utf8_lossy(&output.stdout), "bye\n");
	assert_eq!(output.status.code(), Some(3));
}

#[test]
fn on_exit_runs_once() {
	assert_eq!(lines("on exit { print \"bye\"; exit(0) }\nprint \"hello\""), ["hello", "bye"]);
}
