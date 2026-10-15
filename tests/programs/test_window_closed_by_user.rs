// card escape-window: the user ending the paint window (closing it, Cmd-Q) ends its program and its sounds; before,
// samples/gpu_visualizer_warp.warp kept playing (and opened a new window with its next frame) until ctrl-c. The viewer
// says so with a `closed` line (src/graphics/paint_window.rs), which the program reads among its input lines. Escape
// ends the run likewise, unless the program names that key: then it is the program's, as on a page, and its `exit`
// stops the sounds instead of waiting for them
#![cfg(feature = "native")]
use warp::graphics::paint_window::follow_input;

#[test]
fn the_closed_line_ends_the_run() {
	assert!(follow_input("size 400 300\ninput 1 2 0 0\nclosed\nsize 1 1\n".as_bytes()));
}

#[test]
fn a_viewer_ending_without_it_leaves_the_run_alone() {
	assert!(!follow_input("size 400 300\ninput 1 2 0 0\n".as_bytes()));
}

/// Whether the lowered program tells the runtime at its start that Escape is its key
fn keeps_escape(code: &str) -> bool {
	warp::pipeline::lower(code).unwrap().serialize().contains(warp_runtime::host_words::ESCAPE_KEPT)
}

#[test]
fn a_program_naming_escape_keeps_it() {
	assert!(keeps_escape("on key { if key == ESC { exit } }"));
	assert!(keeps_escape("while held_key != ESCAPE { sleep(0.1) }"));
	assert!(!keeps_escape("on key { if key == UP { print 1 } }"));
	assert!(!keeps_escape("print 1"));
}
