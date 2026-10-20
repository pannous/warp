// card operator-leftover (#30, "Only `is` tests types"): `==` with a class or enum the program declares compares a value
// with the type, as with a built-in type word; only `is` tests the type.
use crate::is;

#[test]
fn equals_with_a_declared_class_does_not_type_test() {
	is!("class P {x:int}; p = P{x:1}; p == P", 0);
	is!("class P {x:int}; p = P{x:1}; p != P", 1);
	is!("class P {x:int}; p = P{x:1}; p is P", 1);
}

#[test]
fn equals_with_a_declared_enum_does_not_type_test() {
	is!("enum C {A, B}; A == C", 0);
	is!("enum C {A, B}; A is C", 1);
}
