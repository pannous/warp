// The ctrl-c rule of `on interrupt` on every platform (Unix SIGINT, Windows SetConsoleCtrlHandler, issue #25): a
// ctrl-c sets a flag the handler takes, a second one within a second or before the handler ran ends the run
#![cfg(feature = "native")]
use warp_runtime::system_signals::ctrl_c_ends_run;

#[test]
fn a_ctrl_c_reaches_the_handler_unless_one_is_pending_or_just_came() {
	assert!(!ctrl_c_ends_run(false, 0, 5_000), "the first ctrl-c sets the flag");
	assert!(ctrl_c_ends_run(true, 0, 5_000), "the one before was not taken yet");
	assert!(ctrl_c_ends_run(false, 4_500, 5_000), "a second ctrl-c within a second");
	assert!(!ctrl_c_ends_run(false, 3_000, 5_000), "taken, and two seconds ago");
}
