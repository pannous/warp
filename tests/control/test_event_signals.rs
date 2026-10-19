// Named event signals (wiki/signal.md, notes/signals.md phase 2, P110): `raise name{data}` runs the `on name {…}`
// handlers of the program, `event` in them is the data; with no handler, raise stays the exception (test_raise.rs)
use crate::is;

#[test]
fn raise_runs_the_handlers_of_its_name() {
	is!("n=0; on alarm {n+=1}; emit alarm; emit alarm; n", 2);
	is!("log=0; on tick {log=log*10+1}; on tick : log=log*10+2; emit tick; log", 12);
}

#[test]
fn the_handler_reads_the_event_data() {
	is!("level=0; on alarm {level=event.level}; emit alarm{level:3}; level", 3);
	is!("why=\"\"; on stop the machine {why=event.reason}; emit stop the machine{reason:\"human\"}; why", "human");
}

#[test]
fn a_raise_inside_a_function_reaches_the_handler() {
	is!("n=0; on alarm {n+=1}; def check(x){ if x>2 {emit alarm}; x }; check(1); check(5); n", 1);
	is!("seen=0; def check(x){ if x>2 {emit too big{value:x}}; x }; on too big {seen=event.value}; check(4); seen", 4);
}

// The async side (tasks forward a raise to the starting thread) finds the exported handler functions

// A function whose body is only a raise keeps its braces (it stays a function, not a getter)
#[test]
fn a_body_of_one_raise_stays_a_block() {
	is!("n=0; on alarm {n+=1}; check() := { emit alarm }; check(); check(); n", 2);
}

// Page events (notes/signals.md phase 7): `on click {…}` is a handler the playground calls; natively nothing raises it
#[test]
fn a_page_event_handler_waits_for_the_page() {
	is!("n=0; on click {n+=1}; n", 0);
}
