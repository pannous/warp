// A reader that closed its end of the pipe (`warp run x.warp | head -1`) ends warp quietly by SIGPIPE, as it ends
// cat or ls: no panic, no crash card (cards crash-show, show-crashes, crashes-broken)
use std::os::unix::process::ExitStatusExt;
use std::process::{Output, Stdio};

const SIGPIPE: i32 = 13;

/// warp with the given arguments, its stdout (or stderr) a pipe nobody reads any more
fn run_into_closed_pipe(arguments: &[&str], closed_stderr: bool) -> Output {
	let (reader, writer) = std::io::pipe().unwrap();
	drop(reader);
	let mut command = crate::common::warp_command();
	command.args(arguments).env("WARP_CRASH_CARDS", "0").stdin(Stdio::null());
	if closed_stderr {
		command.stdout(Stdio::null()).stderr(writer);
	} else {
		command.stdout(writer).stderr(Stdio::piped());
	}
	command.output().unwrap()
}

#[test]
fn a_value_into_a_closed_pipe_ends_warp_quietly() {
	let output = run_into_closed_pipe(&["eval", "42"], false);
	assert_eq!(output.status.signal(), Some(SIGPIPE), "{output:?}");
	assert!(!String::from_utf8_lossy(&output.stderr).contains("panicked"), "{output:?}");
}

#[test]
fn the_version_into_a_closed_stderr_ends_warp_quietly() {
	let output = run_into_closed_pipe(&["--version"], true);
	assert_eq!(output.status.signal(), Some(SIGPIPE), "{output:?}");
}
