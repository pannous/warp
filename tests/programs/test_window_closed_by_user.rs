// card escape-window: the user ending the paint window (Escape, closing it, Cmd-Q) ends its program and its sounds; before,
// samples/gpu_visualizer_warp.warp kept playing (and opened a new window with its next frame) until ctrl-c. The viewer
// says so with a `closed` line (src/graphics/paint_window.rs), which the program reads among its input lines
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
