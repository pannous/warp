// card await-warns: `await any` names the wait itself, so reading the winner never warns that a task "is used as its
// value", also when a racer never blocks. The slow racer sleeps a full second: starting a task takes tens of ms under
// test load, so a 30 ms racer sometimes finished before the quick one had started (card control-races)
use crate::common::warnings_of;
use warp::Node;

#[test]
fn await_any_with_a_racer_that_never_blocks_does_not_warn() {
	let (value, warnings) =
		warnings_of("slow() := { sleep(1 s); 10 }; quick() := { 20 }; r = await any [go slow(), go quick()]; r");
	assert_eq!(value, Node::from(20), "{warnings:?}");
	assert!(warnings.iter().all(|warning| !warning.contains("used as its value")), "{warnings:?}");
}
