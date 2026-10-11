// P159 (wiki/mutable.md): `const x=7; x=7` works with a warning (remove the redundant assignment), another value stays
// P130's error; a `let` is fully immutable, every change of it names `var` (user, card let-reassign)
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
	fails_with("let x=\"hello\"; x+=\" world\"; x", "x is let (immutable), cannot change it");
	fails_with("let x = 1; x++; x", "fix: use var x");
	fails_with("let xs = [1 2]; xs#1 = 5; xs", "xs is let (immutable)");
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

// card let-comma: `let a, b = 3, 4` unpacks like `a, b = 3, 4` (Python's reading, assumed; JS would declare a
// without a value and b = 3: notes/open_decisions.md)
#[test]
fn a_declaration_unpacks_comma_values() {
	is!("let a, b = 3, 4; a * b", 12);
	is!("var a, b = 3, 4; a = 5; a * b", 20);
	is!("const a, b = 3, 4; a + b", 7);
	fails_with("const a, b = 3, 4; b = 1", "b is const, cannot assign it again");
}

// card let-reassign (user): `let mut x` is `var x`, with a note naming var
#[test]
fn let_mut_is_var() {
	is!("let mut x = 1; x = 2; x", 2);
	is!("let mutable xs = [1]; xs.add(2); #xs", 2);
}
