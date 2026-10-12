// card window-loop: `while window_open { … }` before any paint opened the window never ran (window_open was false until
// the first frame). Reading window_open asks the warp binary's opener (src/paint.rs allow_windows) once to open the
// window; headless there is no opener and the loop still never starts
use std::sync::atomic::{AtomicUsize, Ordering};
use warp_runtime::host_words::WINDOW_OPEN;
use warp_runtime::system_values::{read, tell_window_opener};

static OPENED: AtomicUsize = AtomicUsize::new(0);

#[test]
fn reading_window_open_opens_the_window_once() {
	// an opener that opens nothing: window_open stays false for the other tests of this process
	tell_window_opener(|| {
		OPENED.fetch_add(1, Ordering::Relaxed);
	});
	assert_eq!(read(WINDOW_OPEN), Ok(0));
	assert_eq!(read(WINDOW_OPEN), Ok(0));
	assert_eq!(OPENED.load(Ordering::Relaxed), 1, "asked once, not at every reading");
}
