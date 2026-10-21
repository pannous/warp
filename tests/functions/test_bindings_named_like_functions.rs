//! A function's local named like one of the program's functions is the local wherever it is bound: for reads, calls
//! on it, slices and type positions; a call of it holding no function is an error naming both (cards local-named-like)
use crate::is;

#[test]
fn a_local_list_is_tested_for_membership() {
	is!("arity(s: text): int := 1; f() := { arity = [3]; arity in [[3]] }; f()", true);
}

#[test]
fn a_local_list_is_sliced() {
	is!("items(p, closer, bracket) := 1; f() := { items = [1, 2, 3]; items[1..] }; f()", [2, 3]);
}

#[test]
fn a_local_list_stays_a_value_beside_a_function_of_one_value() {
	is!("g(x) := x * 2; f() := { g = [3]; g }; f()", [3]);
	is!("g(x) := x * 2; f() := { g = [3, 4]; #g }; f()", 2);
}

#[test]
fn a_local_parameter_and_loop_variable_win() {
	is!("g(x) := x * 2; f(g) := g in [1, 2]; f(2)", true);
	is!("g(x) := x * 2; f() := { n = 0; for g in [[1], [2, 3]] { n += #g }; n }; f()", 3);
}

#[test]
fn a_local_function_value_is_called() {
	is!("g(x) := x * 2; f() := { g = x => x + 1; g(3) }; f()", 4);
}

#[test]
fn a_call_of_a_local_holding_a_plain_value_names_both() {
	crate::common::fails_with("g(x) := x * 2; f() := { g = 5; g(3) }; f()", "calls the local g = 5");
}

#[test]
fn the_function_is_still_called_outside_the_local_scope() {
	is!("g(x) := x * 2; f() := { g = [3]; #g }; f() + g(4)", 9);
}
