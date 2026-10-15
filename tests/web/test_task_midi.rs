//! Card inside-go: in the browser `midi_on` / `midi_send` inside a `go` task reach the page as the program's own do.
//! The task Worker posts them to the page itself (task-worker.js, host-tasks.js addPageSoundWords; card playground-live:
//! the program's Worker, waiting for the task in Atomics.wait, could pass nothing on in time). Both ends' code under
//! node, the page recording what arrives
use std::process::Command;

const TASK_WORKER: &str = include_str!("../../web/playground/task-worker.js");
const HOST_TASKS: &str = include_str!("../../web/playground/host-tasks.js");
/// the task Worker's own postMessage goes to the page, which keeps what arrives
const PAGE: &str = r#"
const arrived = [];
self.postMessage = message => arrived.push(message);
self.voiceStopped = () => {};
"#;

fn source_between(file: &'static str, start: &str, end: &str) -> &'static str {
	let from = file.find(start).expect("the file has the code");
	let to = from + file[from..].find(end).expect("its end") + end.len();
	&file[from..to]
}

#[test]
fn a_task_relays_its_midi_messages_to_the_page() {
	let page_words = source_between(HOST_TASKS, "let soundHandles", "\n}\n");
	let task_words = source_between(TASK_WORKER, "addPageSoundWords(message", ";");
	let program = format!(
		"globalThis.self = globalThis;\n{PAGE}\n{page_words}\n{task_words}\n\
		 self.sendMidi(\"\", [144, 60, 100], 1.5);\n\
		 console.log(JSON.stringify(arrived));"
	);
	let output = Command::new("node").arg("-e").arg(program).output().expect("node runs");
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	assert_eq!(
		String::from_utf8_lossy(&output.stdout).trim(),
		r#"[{"type":"midi","port":"","message":[144,60,100],"at":1.5}]"#
	);
}
