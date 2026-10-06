// System signals (notes/system_signals.md): `on interrupt {…}` runs at the next loop start after a ctrl-c (SIGINT)
// instead of ending the run; a real `warp run` gets a real SIGINT
#![cfg(all(unix, feature = "native"))]
use crate::common::warp_command;
use std::io::{BufRead, BufReader};
use std::process::Stdio;

const PROGRAM: &str = "stop = false
on interrupt { print \"interrupted\"; stop = true }
i = 0
while not stop {
	if i == 1 { print \"ready\" }
	i += 1
	sleep(10)
}
print \"done\"
";

#[test]
fn on_interrupt_runs_at_the_next_loop_start() {
	let file = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("on_interrupt.wasp");
	std::fs::write(&file, PROGRAM).expect("write the program");
	let mut run = warp_command().args(["run", file.to_str().unwrap()]).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().expect("warp runs");
	let mut lines = BufReader::new(run.stdout.take().unwrap()).lines();
	assert_eq!(lines.next().and_then(Result::ok).as_deref(), Some("ready"), "the loop started");
	let sent = std::process::Command::new("kill").args(["-INT", &run.id().to_string()]).status().expect("kill runs");
	assert!(sent.success());
	let rest: Vec<String> = lines.map_while(Result::ok).collect();
	let status = run.wait().expect("warp ends");
	assert_eq!(rest, ["interrupted", "done"]);
	assert!(status.success(), "the handler ended the loop, the run ends normally: {status}");
}
