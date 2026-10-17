//! card list-methods, decision (a): `dir` of a value lists its type's methods as Python's dir([1, 2]) does, the words of
//! the module named after the type; a map still lists its keys, a number nothing
use crate::is;

#[test]
fn dir_of_a_value_lists_its_types_methods() {
	is!("xs=[3 4]; dir(xs) == dir(list)", true);
	is!("f(v) := dir(v); f([1]) == dir(list)", true);
}
