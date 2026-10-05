//! Quantities are computed at compile time only (src/units.rs): where they would be needed at run time the unit is a loud,
//! explained error, never a dropped unit (notes/units_runtime.md)
use crate::common::fails_with;

#[test]
fn test_quantities_at_run_time_fail_loudly_naming_the_unit() {
	for code in ["speed(d, t) := d/t; speed(10 km, 2 h)", "total = 0 m; for i in 1..3 { total += 5 m }; total", "xs = [1 m, 2 m]; xs#1 + xs#2",
		"c = 1; if c { 5 m } else { 3 m }", "x = 2 km; print x", "sqrt(4 m)"] {
		fails_with(code, "quantities compute only in constant expressions");
	}
}
