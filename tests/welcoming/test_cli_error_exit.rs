// card cli-error-exit: a run ending in an uncaught error exits with status 1, so scripts and CI see it failed; a
// program file's Int stays its exit status, inline code's value is only printed
#![cfg(feature = "native")]

fn exit_status(arguments: &[&str]) -> Option<i32> {
	crate::common::warp_command().arg("--no-ask").args(arguments).output().unwrap().status.code()
}

fn inline_status(code: &str) -> Option<i32> {
	exit_status(&["eval", code])
}

/// The exit status of `warp <file>` running `code`
fn file_status(name: &str, code: &str) -> Option<i32> {
	let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("cli_error_exit_{name}.warp"));
	std::fs::write(&path, code).unwrap();
	exit_status(&["run", &path.to_string_lossy()])
}

#[test]
fn an_uncaught_error_exits_with_status_1() {
	assert_eq!(inline_status("raise \"boom\""), Some(1));
	assert_eq!(inline_status("use json\nparse_json(\"not json\")\n2"), Some(1));
	assert_eq!(inline_status("[1 2]#5"), Some(1));
	assert_eq!(file_status("raise", "raise \"boom\""), Some(1));
}

#[test]
fn a_caught_error_and_a_value_keep_their_status() {
	assert_eq!(inline_status("try { raise \"boom\" } catch { 0 }"), Some(0));
	assert_eq!(inline_status("\"fine\""), Some(0));
	assert_eq!(inline_status("3"), Some(0));
	assert_eq!(file_status("three", "3"), Some(3));
}

// a float result is a measurement, not a status: samples/visualizer.warp ends in its loop's elapsed seconds
#[test]
fn a_program_ending_in_a_float_exits_with_status_0() {
	assert_eq!(file_status("float", "4.5"), Some(0));
	assert_eq!(file_status("float_loop", "elapsed = 0.0\nwhile elapsed < 4 { elapsed = elapsed + 1.25 }"), Some(0));
}
