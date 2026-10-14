// card gap-feeding (wiki/call.md): a phrase call that leaves out `from _` takes the variable named from for that slot
use crate::common::fails_with;
use crate::is;

const INTEGRATE: &str = "to integrate a function from a to b: b - a\n";

#[ignore = "cleanup-edge: cleanup-park-phrases: phrase forms parked until card prepositions-call"]
#[test]
fn a_left_out_slot_takes_the_variable_named_like_its_word() {
	is!(&format!("{INTEGRATE}from = 1\nto = 3\nintegrate sin"), 2);
	is!(&format!("{INTEGRATE}to = 5\nintegrate sin from 2"), 3);
}

#[ignore = "cleanup-edge: cleanup-park-phrases: phrase forms parked until card prepositions-call"]
#[test]
fn written_slots_win() {
	is!(&format!("{INTEGRATE}from = 1\nto = 3\nintegrate sin from 0 to 10"), 10);
}

#[ignore = "cleanup-edge: cleanup-park-phrases: phrase forms parked until card prepositions-call"]
#[test]
fn every_left_out_slot_needs_its_variable() {
	// all or nothing: a partly fed call could match a shorter overload's pattern
	fails_with(&format!("{INTEGRATE}from = 1\nintegrate sin"), "needs a value for parameter a");
}

#[test]
fn a_plain_parameter_is_never_fed() {
	// the parameter's own name is no slot word (test_named_arguments pins `x=7; f(x):=x*x; f()` as the error)
	fails_with("x = 7\nf(x) := x * x\nf()", "needs a value for parameter x");
}
