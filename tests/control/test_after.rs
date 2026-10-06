// `after done return x` (wiki/thread.md): a task that gives x once `done` holds; `await job or y` gives y when the task
// fails or was cancelled
use crate::is;

#[test]
fn after_waits_for_its_condition() {
	is!("shared done = false; go { sleep(50); done = true }; last = after done return \"third\"; await last", "third");
	is!("shared n = 0; go { for i in 1 to 5 { n += 1 } }; five = after (n == 5) return n * 2; await five", 10);
}

#[test]
fn a_cancelled_after_gives_the_fallback() {
	is!("shared done = false; last = after done return \"third\"; cancel last; await last or \"no third\"", "no third");
	is!("job = go { sleep(10); 3 }; await job or 0", 3);
}

#[test]
fn after_needs_a_shared_condition() {
	crate::common::fails_with("done = false; last = after done return 1; await last", "shared done");
}
