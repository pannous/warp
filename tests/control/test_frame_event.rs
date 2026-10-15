// card frame-event (user TODOs in samples/shader_brush.warp: "on frame { }", "remove artificial sleeps"): `on frame {…}`
// runs its body once per frame of the paint window while that shows, in place of `while window_open { …; sleep 16ms }`;
// the run ends when the window closes. Without a window (headless) the body runs once, as one paint would, and the run
// ends instead of staying
#![cfg(all(unix, feature = "native"))]
use crate::common::warp_command;

#[test]
fn a_frame_handler_runs_once_without_a_window() {
	let file = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("on_frame.warp");
	std::fs::write(&file, "n = 0\non frame { n += 1; print n }\nprint \"ready\"\n").expect("write the program");
	let run =
		warp_command().args(["run", file.to_str().unwrap()]).env("WARP_NO_WINDOW", "1").output().expect("warp runs");
	let said = String::from_utf8_lossy(&run.stdout);
	assert_eq!(said.lines().collect::<Vec<_>>(), ["ready", "1"], "{}", String::from_utf8_lossy(&run.stderr));
}

// card full-screen: shader_brush paints at the view's size, read in the frame handler's body; a system value read there
// is no listener on it, so the headless run still ends after its one frame instead of polling the value forever
#[test]
fn a_frame_handler_reading_the_view_size_runs_once_without_a_window() {
	let file = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("on_frame_view.warp");
	std::fs::write(&file, "on frame { print view_width }\nprint \"ready\"\n").expect("write the program");
	let mut command = warp_command();
	command.args(["run", file.to_str().unwrap()]).env("WARP_NO_WINDOW", "1");
	let run = crate::common::output_within(&mut command, std::time::Duration::from_secs(20));
	let said = String::from_utf8_lossy(&run.stdout);
	assert_eq!(said.lines().collect::<Vec<_>>(), ["ready", "640"], "{}", String::from_utf8_lossy(&run.stderr));
}

#[test]
fn a_frame_handler_leaves_main_running() {
	crate::is!("n = 1; on frame { n = 2 }; n", 1);
}

// card native-keys (user: "on key must listen to native signals … in drawing apps, the operating system"): natively,
// `on key {…}` hears the keys pressed over the paint window, named as the playground page names them. The presses are
// told here as the viewer's input lines tell them (src/graphics/paint_window.rs follow_input): a, released, the up arrow
#[test]
fn on_key_hears_the_paint_windows_keys() {
	for key in ['a' as u32, 0, 0xF700, 0] {
		warp_runtime::system_values::tell_window_key(key as f32);
	}
	crate::is!("keys = \"\"\non key { keys = keys + event.key + \" \" }\nsleep(100 ms)\nkeys", "a ArrowUp ");
}
