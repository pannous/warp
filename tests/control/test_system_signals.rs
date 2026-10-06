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

/// `on every 20 ms {…}` fires while main runs: at its loop starts and during its sleeps
#[test]
fn a_timer_fires_while_main_runs() {
	crate::is!("n=0; on every 10 ms { n += 1 }; while n < 3 { sleep(5 ms) }; n", 3);
}

/// `warp run` keeps a program with a timer after main: the timer goes on firing until the run is stopped
#[test]
fn a_program_with_a_timer_stays_after_main() {
	let file = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("on_every.wasp");
	std::fs::write(&file, "n = 0\non every 20 ms { n += 1; print n }\nprint \"ready\"\n").expect("write the program");
	let mut run = warp_command().args(["run", file.to_str().unwrap()]).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().expect("warp runs");
	let lines: Vec<String> = BufReader::new(run.stdout.take().unwrap()).lines().map_while(Result::ok).take(4).collect();
	run.kill().expect("stop the run");
	run.wait().expect("warp ends");
	assert_eq!(lines, ["ready", "1", "2", "3"]);
}

/// P121: `exit` ends the run, not the process: an in-process eval returns ø
#[test]
fn exit_ends_the_run() {
	crate::is!("x = 1; exit; x = 2", warp::node::Node::Empty);
	crate::is!("def stop(){ exit(3) }; stop(); 7", warp::node::Node::Empty);
}

/// `warp run` exits with the code of `exit(code)`; a timer's handler may end a program that stays (P120, P121)
#[test]
fn exit_gives_the_process_its_code() {
	let run = |name: &str, program: &str| {
		let file = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
		std::fs::write(&file, program).expect("write the program");
		warp_command().args(["run", file.to_str().unwrap()]).stderr(Stdio::null()).output().expect("warp runs")
	};
	let coded = run("exit_code.wasp", "print \"a\"\nexit(3)\nprint \"b\"\n");
	assert_eq!((String::from_utf8_lossy(&coded.stdout).as_ref(), coded.status.code()), ("a\n", Some(3)));
	let ticking = run("exit_timer.wasp", "n = 0\non every 10 ms { n += 1; print n; if n == 2 { exit } }\nprint \"ready\"\n");
	assert_eq!((String::from_utf8_lossy(&ticking.stdout).as_ref(), ticking.status.code()), ("ready\n1\n2\n", Some(0)));
}
