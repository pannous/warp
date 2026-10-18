//! A function whose value is the task its `go` starts returns that task: `v = f(); await v` waits for it and reads its
//! value (lib/sound.warp live_loop returns its voice this way)
use crate::is;

#[test]
fn a_task_a_function_returns_is_awaited() {
	is!("later() := go { sleep 0.05s; 3 }\nv = later()\nawait v", 3);
	is!("later(n) := { doubled = n * 2; go { doubled + 1 } }\nv = later(4)\nawait v", 9);
}
