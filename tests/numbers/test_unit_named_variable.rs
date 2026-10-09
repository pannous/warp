// card loop-variable: a variable named like a unit is the variable, not the unit: `for g in 0 to 2 { print g }`
use crate::is;

#[test]
fn a_loop_variable_named_like_a_unit() {
	is!("sum = 0; for g in 1 to 3 { sum += g }; sum", 6);
	is!("sum = 0; for m in [2 3] { sum += m * 10 }; sum", 50);
}

#[test]
fn an_assigned_variable_named_like_a_unit() {
	is!("g = 5; g + 1", 6);
	is!("m = 3; m * 2", 6);
}

#[cfg(feature = "native")]
#[test]
fn a_printed_loop_variable_named_like_a_unit() {
	assert_eq!(crate::common::printed("for g in 0 to 2 { print g }"), "0\n1\n2\n");
}
