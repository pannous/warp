//! Card sleep-run-limit: the playground stops a run after RUN_TIMEOUT_MS of running, not of sleeping (samples/music.warp
//! plays in the background behind sleep(10s)); playground.js's timer functions under node with a fake clock
use std::process::Command;

const PLAYGROUND: &str = include_str!("../../web/playground/playground.js");
/// a run that sleeps 10 s after 1 s and runs 5 s more: stopped only when its running time passes the limit
const FAKE_CLOCK_RUN: &str = r#"
let now = 0, timer;
Date.now = () => now;
const setTimeout = (stop, milliseconds) => (timer = { stop, at: now + milliseconds });
const clearTimeout = () => (timer = undefined);
let stopped;
const stopRun = (run, message) => (stopped = `${now}: ${message}`);
const run = {};
const advance = milliseconds => { now += milliseconds; if (timer && now >= timer.at) timer.stop(); };
stopAfterTimeout(run);
advance(1000);
slept(run, 10000);
advance(10000 + 5000);
const early = stopped;
advance(4000);
console.log(JSON.stringify([early ?? "running", stopped]));
"#;

fn function_source(start: &str, end: &str) -> &'static str {
	let from = PLAYGROUND.find(start).expect("playground.js has the function");
	let to = from + PLAYGROUND[from..].find(end).expect("its end") + end.len();
	&PLAYGROUND[from..to]
}

#[test]
fn sleeping_does_not_count_toward_the_run_limit() {
	let timers = function_source("function stopAfterTimeout", "+ milliseconds);\n");
	let program = format!("const RUN_TIMEOUT_MS = 10000;\n{timers}\n{FAKE_CLOCK_RUN}");
	let output = Command::new("node").arg("-e").arg(program).output().expect("node runs");
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), r#"["running","20000: stopped after 10 s: the program may not terminate"]"#);
}
