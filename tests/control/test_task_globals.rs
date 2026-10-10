//! Card error-undefined (user's samples/music.warp: `play C4` in a go block fell silent): a task's instance gets the
//! program's globals as they are, an exact number too (2.5, 261.63, 10^30), whose Int is a handle into the instance
//! that made it, not a value of its own
use crate::is;

#[test]
fn a_task_reads_an_exact_global() {
	is!("global g = 2.5; f() := g * 2; await go f()", 5);
	is!("global big = 10^30; f() := big + 1 - big; await go f()", 1);
}

#[test]
fn a_go_block_plays_notes() {
	is!("use sound; job = go { C4 * 100 }; await job", 26163);
}
