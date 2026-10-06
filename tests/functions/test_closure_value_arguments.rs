//! Card closures-untyped: a closure reached as a value (returned, in a list) takes any argument; its call helper
//! `closure_call_n` takes Nodes unless every closure of that arity takes numbers (type_closure_calls)
use crate::is;

#[test]
fn test_a_returned_closure_takes_a_text() {
	is!("make = () => (t => t + \"!\"); a = make(); a(\"x\")", "x!");
	is!("make = () => (t => t + \"!\"); a = make(); a(\"xy\")", "xy!");
	is!("def make(){ def add(t){ t + \"!\" }; function add }; a = make(); a(\"x\")", "x!");
}

#[test]
fn test_a_closure_in_a_list_takes_a_text() {
	is!("fs = [t => t + \"!\"]; fs#1(\"x\")", "x!");
}

#[test]
fn test_a_returned_closure_takes_numbers_and_lists() {
	is!("make = () => (t => t * 2); a = make(); a(4)", 8);
	is!("make = () => (t => count t); a = make(); a([1,2,3])", 3);
}
