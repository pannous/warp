// A timer handler slower than its period skips the turns that come due while it runs: its loop's signal poll used to
// call it again inside itself, nesting without end (card timer-handler, probes/signals/slow_handler.warp)
use crate::is;

#[test]
fn a_handler_slower_than_its_period_does_not_call_itself() {
	is!("n = 0\non every 0.001 seconds { for i in 1 to 20000 { x = i % 7 }; n += 1 }\nfor i in 1 to 100000 { y = i % 7 }\nn > 0", true);
}
