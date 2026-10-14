// card operator-shall (user 2026-10-13): `&` and `|` are polymorphic, bitwise on two whole numbers and the logical
// and/or on truth values; `&&`, `||`, `and` and `or` stay logical on numbers too (2 and 3 is 3)
use crate::common;
use crate::is;

#[test]
fn ampersand_and_bar_combine_the_bits_of_whole_numbers() {
	is!("12 & 10", 8);
	is!("12 | 10", 14);
	is!("n = 624485; n & 127", 101);
	is!("n = 101; n | 128", 229);
	is!("-1 & 255", 255);
	is!("-8 | 3", -5);
	is!("x = 12; x & 10 | 1", 9);
}

#[test]
fn ampersand_and_bar_of_typed_and_inferred_ints() {
	is!("f(a:int, b:int) := a & b; f(12, 10)", 8);
	is!("mask(n:int) := n & 255; mask(256 + 7)", 7);
	is!("low(n:int) := n | 1; low(4) + 1", 6);
	is!("x = 3 * 4; y = x & 10; y + 1", 9);
}

// user 2026-10-13: any two whole numbers, whatever gives them: a call or a prefix operation after `|` too
#[test]
fn ampersand_and_bar_of_calls_and_operations() {
	is!("f(x:int) := x * 2; n = 1; n | f(4)", 9);
	is!("f(x:int) := x * 2; f(6) & f(3)", 4);
	is!("n = 1; n | abs -6", 7);
	is!("n = 2; m = 4; n | m", 6);
	is!("n = 6; m = 3; n & m", 2);
}

#[test]
fn ampersand_and_bar_of_truth_values_are_logical() {
	is!("yes & no", false);
	is!("yes | no", true);
	is!("a = yes; b = no; a & b", false);
	is!("a = yes; b = no; a | b", true);
	is!("(1 < 2) & (3 < 4)", true);
}

#[test]
fn double_and_word_forms_stay_logical_on_numbers() {
	is!("12 && 10", 10);
	is!("12 and 10", 10);
	is!("0 || 5", 5);
	is!("0 or 5", 5);
}

// user 2026-10-13: `if 8 & 4 {…}` tests the bits 8 and 4 share (none), not whether both are truthy: a warning names
// both readings
#[test]
fn bits_as_a_condition_warn() {
	common::warns_with("if 8 & 4 { 1 } else { 2 }", 2, "combines the bits");
	common::warns_with("x = 8; while x & 12 { x = x - 1 }; x", 3, "combines the bits");
	for unambiguous in
		["if (8 & 4) != 0 { 1 } else { 2 }", "if 8 and 4 { 1 } else { 2 }", "if yes & no { 1 } else { 2 }"]
	{
		let (_, warnings) = common::warnings_of(unambiguous);
		assert!(warnings.iter().all(|warning| !warning.contains("combines the bits")), "{unambiguous}: {warnings:?}");
	}
}
