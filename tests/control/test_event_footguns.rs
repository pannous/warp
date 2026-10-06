// Footguns of events and signals in other systems and what warp does (wiki/Footguns.md "Events, signals and channels")
use crate::is;

#[test]
fn an_event_raised_before_its_handler_reaches_it() {
	is!("n=0; raise ping; on ping {n+=1}; n", 1);
}

#[test]
fn a_handler_made_in_a_loop_keeps_its_iteration() {
	is!("fs = 0; for i in 1 to 3 { on click { fs += i } }; raise click; fs", 6);
}

#[test]
fn a_handler_reads_the_current_state() {
	is!("x = 1; seen = 0; on ping { seen = x }; x = 2; raise ping; seen", 2);
}

#[test]
fn a_listener_writing_what_it_watches_does_not_loop() {
	is!("x = 0; on change x { x = x + 1 }; x = 1; x", 2);
}

#[test]
fn removing_a_handler_while_dispatching_skips_no_other() {
	is!("n = 0; h = on tick { n += 1; remove h from listeners of tick }; on tick { n += 10 }; raise tick; raise tick; n", 21);
}

// the DOM hands one mutable event object down its listeners
#[test]
fn each_handler_gets_the_event_as_raised() {
	is!("seen = 0; on e { event.x = 2 }; on e { seen = event.x }; raise e{x:1}; seen", 1);
	is!("seen = 0; on e { event.x += 5; seen = event.x }; raise e{x:1}; seen", 6);
}

// Vue 2 missed some in-place changes, React misses all of them
#[test]
fn a_change_through_a_list_method_runs_the_listeners() {
	is!("n = 0; x = [1]; on change x { n += 1 }; x.add(2); n", 1);
	is!("n = 0; xs = [1]; f() := { global xs; xs.add(3) }; on set xs { n += 1 }; f(); n", 1);
}
