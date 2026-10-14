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
		.arg(warp::graphics::paint_window::COMMAND)
		.env(warp::graphics::paint::NO_WINDOW_VARIABLE, "1")
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

// a program that ended well waits for its window only where one opened: under WARP_NO_WINDOW (tests, CI) or without a
// terminal its painting is a PNG and the run ends at once
#[test]
fn a_painting_without_a_window_never_waits() {
	let file = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("painted_headless.warp");
	std::fs::write(&file, "paint([1, 0, 1], 3, 1)\nprint \"painted\"\n").expect("write the program");
	for no_window in [true, false] {
		let mut command = warp_command();
		command.args(["run", file.to_str().unwrap()]).stdout(Stdio::piped()).stderr(Stdio::null());
		if !no_window {
			command.env_remove(warp::graphics::paint::NO_WINDOW_VARIABLE).env("CI", "1");
		}
		let mut run = command.spawn().expect("warp runs");
		let started = Instant::now();
		while run.try_wait().expect("the run's status").is_none() {
			if started.elapsed() > PATIENCE {
				let _ = run.kill();
				panic!("the run waited for a window (WARP_NO_WINDOW {no_window})");
			}
			std::thread::sleep(Duration::from_millis(50));
		}
		let output = run.wait_with_output().expect("the run's output");
		assert!(String::from_utf8_lossy(&output.stdout).contains("painted"), "{output:?}");
	}
}
