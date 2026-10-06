// Errors that were raw engine traps are named runtime errors a `try` catches: indexing a number, a missing field
use crate::common::fails_with;
use crate::is;

#[test]
fn indexing_a_number_is_not_a_list() {
	fails_with("x=3; x#1", "not a list");
	is!("def f(m){ m#1 }; try f(3) else 7", 7);
}

#[test]
fn a_missing_field_is_caught() {
	is!("def f(m){ m.a }; try f(3) else 7", 7);
	is!("def f(m){ m.a }; try f({a:2}) else 7", 2);
}
