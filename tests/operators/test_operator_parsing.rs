//! Which operator binds first: precedence, associativity, prefix/suffix and glyph forms, as the values show them
//! (condensed from probe_operators.rs, probe_precedence.rs and probe_increment.rs)
use crate::is;

#[test]
fn test_arithmetic_precedence() {
	is!("1 + 2 * 3", 7);
	is!("1 + 2 == 3", true);
	is!("-5 + 3", -2);
}

#[test]
fn test_prefix_and_suffix() {
	is!("x=3; -x", -3);
	is!("x=3; x²", 9);
}

#[test]
fn test_unicode_glyphs() {
	is!("2 ≤ 3", true);
	is!("x = 2; y = 3; x × y", 6);
}

#[test]
fn test_power_right_assoc() {
	is!("2^3^2", 512);
}

#[test]
fn test_assignment_right_assoc() {
	is!("a = b = 3; a + b", 6);
}

#[test]
fn test_range_and_index() {
	is!("count(1..10)", 9);
	is!("xs=[5,6,7]; xs#3", 7);
}

#[test]
fn test_division_type_upgrade() {
	is!("1/2", 0.5);
	is!("3/2", 1.5);
	is!("-1/2", -0.5);
	is!("10/4", 2.5);
	is!("-1/6", -1.0 / 6.0);
}

#[test]
fn typed_name_binds_before_assignment() {
	// (a:int)=7, not a:(int=7)
	is!("a:int=7; a", 7);
}

#[test]
fn increment_suffix_and_while_condition_spacing() {
	is!("i=0; while(i<9){i++}; i", 9);
	is!("i=0; while (i<9) {i++}; i", 9);
}
