// `exit 3` in a handler's braces ends the run with code 3, not 0: only a lone `exit` there means exit(0) (card key-app)
#![cfg(all(unix, feature = "native"))]
use crate::common::warp_command;
use std::process::Stdio;

fn exit_code(name: &str, program: &str) -> Option<i32> {
	let file = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
	std::fs::write(&file, program).expect("write the program");
	warp_command()
		.args(["run", file.to_str().unwrap()])
		.stderr(Stdio::null())
		.output()
		.expect("warp runs")
		.status
		.code()
}

#[test]
fn exit_with_a_code_in_a_handler_body() {
	assert_eq!(exit_code("handler_exit_code.warp", "on alarm { exit 3 }\nemit alarm\n"), Some(3));
	assert_eq!(exit_code("handler_bare_exit.warp", "on alarm { exit }\nemit alarm\nexit 5\n"), Some(0));
}
