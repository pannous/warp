// Card glued-loop: a block word glued to its block reads as when spaced, `loop{…}` as `loop {…}` and `do{…}` as
// `do {…}`, no tags `loop:{…}`, `do:{…}` (as `go{…}`, card error-beep)
use crate::is;

#[test]
fn a_glued_loop_runs_its_block() {
	is!("i = 0; loop{ i = i + 1; if i > 2 { break } }; i", 3);
	is!("i = 0; loop { i = i + 1; if i > 2 { break } }; i", 3);
}

#[test]
fn a_glued_do_runs_its_block() {
	is!("x = do{ 5 }; x", 5);
	is!("x = do { 5 }; x", 5);
	is!("i = 0; do{ i = i + 1 } while i < 3; i", 3);
	is!("i = 0; do { i = i + 1 } while i < 3; i", 3);
}
