//! Card inside-go: in the browser `midi_on` / `midi_send` inside a `go` task reach the page as the program's own do:
//! the task Worker relays sendMidi to the program's Worker (task-worker.js relay, host-tasks.js RELAYED), which posts
//! it to the page's Web MIDI. Both ends' code under node, the program's Worker recording what arrives
use std::process::Command;

const TASK_WORKER: &str = include_str!("../../web/playground/task-worker.js");
const HOST_TASKS: &str = include_str!("../../web/playground/host-tasks.js");
/// the task Worker's port to the program's Worker, which keeps what the task relays; the page's sendMidi records it
const TWO_WORKERS: &str = r#"
const relayedOnTheProgramsWorker = [];
let program = { postMessage: ({ relay }) => relayedOnTheProgramsWorker.push(relay) };
const arrived = [];
"#;

fn source_between(file: &'static str, start: &str, end: &str) -> &'static str {
	let from = file.find(start).expect("the file has the code");
	let to = from + file[from..].find(end).expect("its end") + end.len();
	&file[from..to]
}

#[test]
fn a_task_relays_its_midi_messages_to_the_page() {
	let relayed_words = source_between(HOST_TASKS, "const RELAYED", "\n}\n");
	let task_relay = source_between(TASK_WORKER, "const relay =", "relay(method, ...values);\n");
	let program = format!(
		"globalThis.self = globalThis;\n{relayed_words}\n{TWO_WORKERS}\n{task_relay}\n\
		 self.sendMidi(\"\", [144, 60, 100], 1.5);\n\
		 self.sendMidi = (...values) => arrived.push(values);\n\
		 relayedOnTheProgramsWorker.forEach(relayed);\n\
		 console.log(JSON.stringify(arrived));"
	);
	let output = Command::new("node").arg("-e").arg(program).output().expect("node runs");
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), r#"[["",[144,60,100],1.5]]"#);
}
