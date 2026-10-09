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

// card let-tuple: let and const take a tuple pattern; each name is bound like a single declaration
#[test]
fn let_and_const_unpack_a_tuple() {
	is!("let (a, b) = (1, 2); a + b", 3);
	is!("let [a, b] = [3, 4]; a * b", 12);
	is!("v = {x: 1, y: 2}; let (names, numbers) = (keys(v), values(v)); count(names) + numbers#2", 4);
	is!("const (a, b) = (5, 6); b - a", 1);
	fails_with("const (a, b) = (5, 6); a = 7", "a is const, cannot assign it again");
}
