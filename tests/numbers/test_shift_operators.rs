use warp::*;

use crate::common;

const FLOAT_IN_EXACT_CONTEXT: &str = "is a float where an exact Int is expected";

#[test]
fn test_shift_left_multiplies_by_a_power_of_two() {
	is!("2 << 1", 4);
	is!("1 << 10", 1024);
	is!("-3 << 2", -12);
	is!("5 << 0", 5);
}

#[test]
fn test_shift_right_floors_by_a_power_of_two() {
	is!("8 >> 1", 4);
	is!("1024 >> 3", 128);
	is!("7 >> 1", 3);
	is!("-8 >> 1", -4);
	is!("-7 >> 1", -4);
	is!("5 >> 0", 5);
}

#[test]
fn test_shifts_of_big_exact_ints() {
	is!("1 << 100 == 2^100", true);
	is!("(2^100) >> 98", 4);
	is!("(1 << 200) >> 199", 2);
}

#[test]
fn test_shift_precedence_and_variables() {
	is!("1 << 3 + 1", 16);
	is!("x=3;1 << x", 8);
	is!("x=3;x<<1", 6);
	is!("1 << 2 < 5", true);
	is!("f(n) := 1 << n; f(4)", 16);
}

#[test]
fn test_shift_of_a_float_is_refused() {
	for operator in ["<<", ">>"] {
		common::fails_with(&format!("f(x:float) := x {operator} 1; f(2.5)"), FLOAT_IN_EXACT_CONTEXT);
		common::fails_with(&format!("f(x:float) := 1 {operator} x; f(1.0)"), FLOAT_IN_EXACT_CONTEXT);
	}
}

#[test]
fn test_shift_of_a_non_integer_or_by_a_bad_count_is_an_error() {
	common::fails_with("1 << -1", "shift");
	common::fails_with("1 >> -1", "shift");
	common::fails_with("1 << 1000000", "shift");
	common::fails_with("2.5 << 1", "shift");
	common::fails_with("1 << 0.5", "shift");
}

#[test]
fn test_angle_brackets_still_delimit_generics() {
	is!("1 < 2", true);
	is!("2 > 1", true);
	let generic = parse("x:list<int> = [1 2 3]");
	assert!(!format!("{generic:?}").contains("Shl"), "{generic:?}");
	let nested = parse("x:list<list<int>> = [[1] [2]]");
	assert!(!format!("{nested:?}").contains("Shr"), "{nested:?}");
}
