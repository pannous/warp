// Named event signals (wiki/signal.md, notes/signals.md phase 2, P110): `raise name{data}` runs the `on name {…}`
// handlers of the program, `event` in them is the data; with no handler, raise stays the exception (test_raise.rs)
use crate::is;

#[test]
fn raise_runs_the_handlers_of_its_name() {
	is!("n=0; on alarm {n+=1}; raise alarm; raise alarm; n", 2);
	is!("log=0; on tick {log=log*10+1}; on tick : log=log*10+2; raise tick; log", 12);
}

#[test]
fn the_handler_reads_the_event_data() {
	is!("level=0; on alarm {level=event.level}; raise alarm{level:3}; level", 3);
	is!("why=\"\"; on stop the machine {why=event.reason}; raise stop the machine{reason:\"human\"}; why", "human");
}

#[test]
fn a_raise_inside_a_function_reaches_the_handler() {
	is!("n=0; on alarm {n+=1}; def check(x){ if x>2 {raise alarm}; x }; check(1); check(5); n", 1);
	is!("seen=0; def check(x){ if x>2 {raise too big{value:x}}; x }; on too big {seen=event.value}; check(4); seen", 4);
}
