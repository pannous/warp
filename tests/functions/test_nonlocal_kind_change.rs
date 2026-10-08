// Card nonlocal-assign: a function writing a variable it shares (`nonlocal y`, `global y`) keeps its kind, as P45
// does for a variable of its own: `y = 1; y = "a"` is an error
use crate::common::fails_with;
#[cfg(feature = "native")]
use crate::is;

const KIND_CHANGE: &str = "y was an Int, is given a Text: use another name";

#[test]
fn a_shared_variable_keeps_its_kind() {
	fails_with("outer() := { y = 1; inner() := { nonlocal y; y = \"a\" }; inner(); y }; outer()", KIND_CHANGE);
	fails_with("y = 1; g() := { global y; y = \"a\" }; g(); y", KIND_CHANGE);
	fails_with("g() := { global y = 1; y = \"a\" }; g(); y", KIND_CHANGE);
}

#[test]
#[cfg(feature = "native")]
fn a_shared_variable_takes_values_of_its_kind() {
	is!("outer() := { y = 1; inner() := { nonlocal y; y = 5 }; inner(); y }; outer()", 5);
	is!("y = 1; g() := { global y; y = 7 }; g(); y", 7);
	is!("f() := { y = \"a\"; y }; y = 1; f(); y", 1);
}
