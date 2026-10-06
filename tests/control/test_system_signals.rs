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
	let (rest, status) = interrupted_run("on_interrupt.wasp", PROGRAM);
	assert_eq!(rest, ["interrupted", "done"]);
	assert!(status.success(), "the handler ended the loop, the run ends normally: {status}");
}

/// A ctrl-c during sleep(…) ends the sleep and runs the handler at once
#[test]
fn on_interrupt_cuts_a_sleep_short() {
	let started = std::time::Instant::now();
	let (rest, status) = interrupted_run("on_interrupt_sleep.wasp", "on interrupt { print \"interrupted\" }\nprint \"ready\"\nsleep(20000)\nprint \"done\"\n");
	assert_eq!(rest, ["interrupted", "done"]);
	assert!(status.success(), "{status}");
	assert!(started.elapsed().as_secs() < 15, "the sleep of 20 s was cut short: {:?}", started.elapsed());
}

/// Runs the program, sends SIGINT once it printed "ready", and gives the lines it printed after that and its status
fn interrupted_run(name: &str, program: &str) -> (Vec<String>, std::process::ExitStatus) {
	let file = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
	std::fs::write(&file, program).expect("write the program");
	let mut run = warp_command().args(["run", file.to_str().unwrap()]).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().expect("warp runs");
	let mut lines = BufReader::new(run.stdout.take().unwrap()).lines();
	assert_eq!(lines.next().and_then(Result::ok).as_deref(), Some("ready"), "the program started");
	let sent = std::process::Command::new("kill").args(["-INT", &run.id().to_string()]).status().expect("kill runs");
	assert!(sent.success());
	let rest: Vec<String> = lines.map_while(Result::ok).collect();
	(rest, run.wait().expect("warp ends"))
}
