// card event-handlers (user 2026-10-06: the listeners of alarm as a list to delete from arbitrarily): `listeners of
// alarm` is the list of its handlers still listening, each by its place among alarm's handlers (1, 2 …, a named
// handler h is its place too), and any of them is removed by it, also an unnamed one
use crate::common::fails_with;
use crate::is;
use warp::ints;

#[test]
fn the_listeners_of_an_event_are_a_list() {
	is!("on alarm {1}; on alarm {2}; listeners of alarm", ints(vec![1, 2]));
	is!("on alarm {1}; on alarm {2}; on alarm {3}; remove 2 from listeners of alarm; listeners of alarm", ints(vec![1, 3]));
	is!("on alarm {1}; h = on alarm {2}; raise alarm; h", 2);
	is!("on alarm {1}; on alarm {2}; remove 1 from listeners of alarm; count listeners of alarm", 1);
}

#[test]
fn any_handler_is_removed_by_its_place() {
	is!("n = 0; on alarm {n += 1}; on alarm {n += 10}; remove (listeners of alarm)#1 from listeners of alarm; raise alarm; n", 10);
	is!("n = 0; on alarm {n += 1}; on alarm {n += 10}; listeners of alarm -= 2; raise alarm; n", 1);
	is!("n = 0; on alarm {n += 1}; h = on alarm {n += 10}; for l in listeners of alarm { remove l from listeners of alarm }; raise alarm; n", 0);
	is!("n = 0; on alarm {n += 1}; remove 1 from listeners of alarm; remove 1 from listeners of alarm; raise alarm; n", 0);
}

// wiki Footguns "Removing a listener that does not exist"
#[test]
fn removing_an_unknown_listener_is_a_compile_error() {
	fails_with("on tick {1}; remove h from listeners of tick; raise tick", "h is no named listener of tick");
	fails_with("h = on alarm {1}; on tick {2}; remove h from listeners of tick; raise tick", "h is no named listener of tick");
}
