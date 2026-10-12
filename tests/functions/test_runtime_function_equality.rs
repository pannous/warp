//! card runtime-function: `f == g` of function values only the run time knows compares their definitions as dump shows
//! them, up to the names of the parameters, and the values they captured
use crate::is;

#[test]
fn a_function_value_equals_itself() {
	is!("xs = [x => x + 1]; xs#1 == xs#1", true);
}

#[test]
fn equal_definitions_are_equal_functions() {
	is!("xs = [x => x + 1, y => y + 1]; xs#1 == xs#2", true);
	is!("f(x) := x + 1; g(y) := y + 1; xs = [&f, &g]; xs#1 == xs#2", true);
}

#[test]
fn different_definitions_are_different_functions() {
	is!("xs = [x => x + 1, z => z * 2]; xs#1 == xs#2", false);
	is!("xs = [x => x + 1, z => z * 2]; xs#1 != xs#2", true);
}

#[test]
fn closures_compare_their_captured_values() {
	is!("make(n) := x => x + n; a = make(1); b = make(1); a == b", true);
	is!("make(n) := x => x + n; a = make(1); c = make(2); a == c", false);
	is!("make(n) := x => x + n; make(1) == make(1)", true);
	is!("make(n) := x => x + n; make(1) == make(2)", false);
}
