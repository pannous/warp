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
	let handover = source_between(
		PLAYGROUND,
		"function liveCycled",
		"finish(run, stoppedReport(run, LIVE_HANDED_OVER, false));\n}\n",
	);
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
		const live = playsLive({ id: 1 });
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

/// decision deleted-loop: the rerun names its live loops as it starts (its module's LIVE_LOOPS_SECTION); the old run's
/// loops of the rest fade out, page timers set by hand and sounds whose gain notes its ramps
fn deleted_loop_program(scenario: &str) -> String {
	let fade_out = source_between(PLAYGROUND, "function fadeOut", "\n}\n");
	let timers = r#"
		const timers = [], cleared = [];
		globalThis.setTimeout = (run, ms) => timers.push({ run, ms });
		globalThis.clearTimeout = id => id && cleared.push(id);
		const ramped = source => Object.assign(source, { gainNode: { gain: { value: 1, ramps: [],
			setValueAtTime(value, at) { this.ramps.push(["set", value, at]); },
			linearRampToValueAtTime(value, at) { this.ramps.push(["ramp", value, at]); } } } });
	"#;
	page_program(&format!("{fade_out}\n{timers}\n{scenario}"))
}

#[test]
fn a_deleted_loop_fades_out_as_soon_as_the_rerun_names_its_loops() {
	let scenario = r#"
		liveCycled(worker, "drums", 10);
		liveCycled(worker, "bass", 10);
		const old = worker;
		sounding = [sound(old, "drums", 13, 15), sound(old, "bass", 13, 15), sound(old, "bass", 17, 19)].map(ramped);
		fakeNow = audio.currentTime = 12;
		handOver({ id: 1 });
		audio.currentTime = 12.5;
		liveLoopsNamed(["drums"]);
		const fading = { terminated: [...terminated], drums: sounding[0].gainNode.gain.ramps.length };
		liveCycled(worker, "drums", 14);
		console.log(JSON.stringify({ fading, bass: sounding[1].gainNode.gain.ramps, stopped: sounding.map(s => s.stopped),
			terminated, cleared, retired: retiredWorkers.size }));
	"#;
	assert_eq!(
		node_output(&deleted_loop_program(scenario)),
		r#"{"fading":{"terminated":[],"drums":0},"bass":[["set",1,12.5],["ramp",0,13.5]],"stopped":[14,13.5,13.5],"terminated":["old"],"cleared":[1,1],"retired":0}"#
	);
}

#[test]
fn a_kept_loop_with_a_long_period_is_not_faded() {
	let scenario = r#"
		liveCycled(worker, "drums", 4);
		liveCycled(worker, "drums", 10);
		const old = worker;
		sounding = [sound(old, "drums", 10, 16), sound(old, "drums", 16, 22)].map(ramped);
		fakeNow = audio.currentTime = 12;
		handOver({ id: 1 });
		liveLoopsNamed(["drums"]);
		fakeNow = audio.currentTime = 15.5;
		const waiting = { terminated: [...terminated], ramps: sounding.map(s => s.gainNode.gain.ramps.length) };
		liveCycled(worker, "drums", 16);
		console.log(JSON.stringify({ waiting, stopped: sounding.map(s => s.stopped), terminated, cleared }));
	"#;
	assert_eq!(
		node_output(&deleted_loop_program(scenario)),
		r#"{"waiting":{"terminated":[],"ramps":[0,0]},"stopped":[null,16],"terminated":["old"],"cleared":[1,1]}"#
	);
}

/// LIVE_LOOPS_SECTION lists the program's live loops by name, which the playground's rerun reports (host-tasks.js)
#[test]
fn the_module_names_its_live_loops() {
	let program = "use sound\nlive_loop drums 2 { play(C4, 0.1s) }\nlive_loop bass 2 { play(C2, 0.1s) }\nawait drums\nawait bass";
	let module = warp::pipeline::compile(program).expect("compiles");
	let names = wasmparser::Parser::new(0).parse_all(&module.bytes).find_map(|payload| match payload.unwrap() {
		wasmparser::Payload::CustomSection(section) if section.name() == warp::host::LIVE_LOOPS_SECTION => {
			Some(String::from_utf8(section.data().to_vec()).unwrap())
		}
		_ => None,
	});
	assert_eq!(names.as_deref(), Some("bass\ndrums"));
}
