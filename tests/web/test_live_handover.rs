//! Card playground-live: a rerun while live loops play hands them over. The page retires the old run's Worker instead of
//! stopping it (playground.js handOver); a loop of the new run starts on the grid of the old one's cycles
//! (host-tasks.js liveCycleStarted) and the old loop of its name gives way at that cycle (liveCycled). The code of both
//! ends under node, the page's sounds and Workers as records
use super::test_task_midi::source_between;
use std::process::Command;

const HOST_TASKS: &str = include_str!("../../web/playground/host-tasks.js");
const PLAYGROUND: &str = include_str!("../../web/playground/playground.js");

/// what the page's code reads: a clock set by hand, sounds that note when they are stopped, Workers that note their end
const PAGE: &str = r#"
let fakeNow = 0, clockOffset = 0, sounding = [], liveHandover = {};
const audio = { currentTime: 0 };
const soundClock = () => fakeNow;
const terminated = [], finished = [];
const madeWorker = name => ({ name, liveLoops: new Map(), terminate() { terminated.push(name); } });
let worker = Object.assign(madeWorker("old"), { runId: 1 });
const startWorker = () => { worker = madeWorker("new"); };
const finish = (run, report) => finished.push(report.value);
const stoppedReport = (run, message) => ({ value: message });
const sound = (from, loop, startsAt, endsAt) => ({ from, loop, startsAt, endsAt, stopped: null, stop(when) { this.stopped = when ?? "now"; } });
"#;

fn node_output(program: &str) -> String {
	let output = Command::new("node").arg("-e").arg(program).output().expect("node runs");
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// the page with its handover code, then the scenario
fn page_program(scenario: &str) -> String {
	let constants = source_between(PLAYGROUND, "const LIVE_HANDED_OVER", "const LIVE_RETIRE_SECONDS = 4;\n");
	let retired = source_between(PLAYGROUND, "const retiredWorkers", ";");
	let handover = source_between(PLAYGROUND, "function liveCycled", "finish(run, stoppedReport(run, LIVE_HANDED_OVER, false));\n}\n");
	format!("{PAGE}\n{constants}\n{retired}\n{handover}\n{scenario}")
}

#[test]
fn a_new_runs_loop_takes_over_at_the_old_loops_cycle() {
	let scenario = r#"
		liveCycled(worker, "drums", 10);
		liveCycled(worker, "drums", 12);
		const old = worker;
		sounding = [sound(old, "drums", 12, 14), sound(old, "drums", 14, 16), sound(old, undefined, 13, 20)];
		fakeNow = 13;
		const live = isLive({ id: 1 });
		handOver({ id: 1 });
		const handedOver = JSON.parse(JSON.stringify(liveHandover));
		liveCycled(worker, "drums", 14);
		console.log(JSON.stringify({ live, handedOver, stopped: sounding.map(s => s.stopped), terminated, finished, retired: retiredWorkers.size }));
	"#;
	assert_eq!(
		node_output(&page_program(scenario)),
		r#"{"live":true,"handedOver":{"drums":{"start":12,"period":2}},"stopped":[null,14,"now"],"terminated":["old"],"finished":["its live loops play on into the next run"],"retired":0}"#
	);
}

#[test]
fn a_loop_the_new_run_drops_ends_at_a_cycle_after_a_while() {
	let scenario = r#"
		liveCycled(worker, "drums", 10);
		const old = worker;
		sounding = [sound(old, "drums", 13, 15), sound(old, "drums", 17, 19)];
		fakeNow = 12;
		handOver({ id: 1 });
		fakeNow = 13.5;
		liveCycled(old, "drums", 14);
		const early = terminated.length;
		fakeNow = 16.5;
		liveCycled(old, "drums", 18);
		console.log(JSON.stringify({ early, stopped: sounding.map(s => s.stopped), terminated }));
	"#;
	assert_eq!(node_output(&page_program(scenario)), r#"{"early":0,"stopped":[null,18],"terminated":["old"]}"#);
}

#[test]
fn a_handed_over_loop_starts_on_the_old_runs_grid() {
	let voices = source_between(HOST_TASKS, "let voice = {};", "* period : Math.max(start, moment);\n");
	let scenario = r#"
		const said = [];
		self.liveCycled = (name, at) => said.push([name, Math.round((at - now) * 10) / 10]);
		const now = soundClock();
		self.liveHandover = { drums: { start: now - 3.5, period: 2 } };
		self.liveCycleStarted("drums");
		self.voiceSounded(2);
		self.liveCycleStarted("drums");
		const loop = self.currentVoice().loop;
		self.joinVoice();
		self.liveCycleStarted("bass");
		console.log(JSON.stringify({ said, loop }));
	"#;
	let program = format!("globalThis.self = globalThis;\n{voices}\n{scenario}");
	assert_eq!(node_output(&program), r#"{"said":[["drums",0.5],["drums",2.5],["bass",0]],"loop":"drums"}"#);
}
