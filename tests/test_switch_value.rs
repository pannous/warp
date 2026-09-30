//! A switch is an expression: bind it, or put it in parentheses to compute with it

use warp::is;

#[test]
fn a_switch_can_be_bound() {
	is!("y=switch 2 {1: 10 2: 20}; y+1", 21);
	is!("c=1; y=match c {1: 10 default: 0}; y", 10);
}

#[test]
fn a_parenthesized_switch_takes_part_in_arithmetic() {
	is!("(switch 1 {1: 10 2: 20}) + 1", 11);
}
