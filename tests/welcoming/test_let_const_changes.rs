// P159 (wiki/mutable.md): `const x=7; x=7` works with a warning (remove the redundant assignment), another value stays
// P130's error; a `let` variable may change, with a got-it note teaching `var`
use crate::common::fails_with;
use crate::is;
use warp::ints;

#[test]
fn a_constant_assigned_its_own_value_again() {
	is!("const x=7; x=7; x", 7);
	is!("const xs=[1,2,3]; xs=[1,2,3]; xs", ints(vec![1, 2, 3]));
	fails_with("const x=7; x=8", "x is const, cannot assign it again");
}

#[test]
fn a_let_variable_changes() {
	is!("let x=\"hello\"; x+=\" world\"; x", "hello world");
	is!("let x = 1; x++; x", 2);
	is!("let xs = [1 2]; xs#1 = 5; xs", ints(vec![5, 2]));
}
