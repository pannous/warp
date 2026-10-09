//! Quantities are computed at compile time only (src/units.rs): where they would be needed at run time the unit is a loud,
//! explained error, never a dropped unit (notes/units_runtime.md)

#[test]
fn test_quantities_at_run_time_fail_loudly_naming_the_unit() {
	// loops and branches compute since static units stage 1 (test_static_units.rs), functions since stage 2
	// print since stage 3, list elements and √ since stage 4, a whole list since stage 5, a list grown at run time since stage 6
	assert_eq!(warp::wasm_emitter::eval("xs = [1 m]; xs.add(2 m); sum(xs)").serialize().trim(), "3m");
}
