//! Reflection words as dot forms (card reflection step 1, notes/reflection.md): `f.effects` is `effects of f`,
//! `e.listeners` is `listeners of e`; a real field of that name wins (card listeners-tick: an unknown name is loud)
use crate::common::fails_with;
use crate::is;
use warp::ints;
use warp::Node;

#[test]
fn a_functions_effects_as_a_dot_form() {
	is!("f(x) := print x\nf.effects", Node::Symbol("IO".into()));
}

#[test]
fn an_events_listeners_as_a_dot_form() {
	is!("on alarm {1}; on alarm {2}; alarm.listeners", ints(vec![1, 2]));
	is!("on tick {1}; count tick.listeners", 1);
	is!("x = 0; on set x {print value}; count x.listeners", 1);
}

#[test]
fn a_real_field_named_listeners_wins() {
	is!("p = {listeners: 7}; p.listeners", 7);
}

#[test]
fn listeners_of_a_name_nothing_listens_to_is_loud() {
	fails_with("listeners of alarm", "nothing listens to alarm");
}
