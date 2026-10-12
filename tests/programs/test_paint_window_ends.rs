// card exit-key: the paint window's process ends with its program. Its stdin is the program's pipe, so the program
// ending any way (exit(), a failure, ctrl-c, a kill) closes the window; before, `on key { exit() }` left it open and dead.
// No window opens here: the viewer opens none before its first frame
#![cfg(all(unix, feature = "native"))]
use crate::common::warp_command;
use std::process::Stdio;
use std::time::{Duration, Instant};

const PATIENCE: Duration = Duration::from_secs(20);

#[test]
fn the_paint_window_ends_when_its_program_does() {
	let mut viewer = warp_command()
		.arg(warp::paint_window::COMMAND)
		.env(warp::paint::NO_WINDOW_VARIABLE, "1")
		.stdin(Stdio::piped())
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.spawn()
		.expect("the viewer starts");
	drop(viewer.stdin.take()); // the program is gone
	let started = Instant::now();
	let ended = loop {
		if let Some(status) = viewer.try_wait().expect("the viewer's status") {
			break Some(status);
		}
		if started.elapsed() > PATIENCE {
			let _ = viewer.kill();
			let _ = viewer.wait();
			break None;
		}
		std::thread::sleep(Duration::from_millis(50));
	};
	assert!(ended.is_some(), "the viewer outlived its program by {PATIENCE:?}");
}
