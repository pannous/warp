//! card constant-quantity: spaced, a number and a name stay a list (decisions.md: juxtaposition only glued, `3 x` stays
//! a list), so `2 g0` of a quantity g0 says how to multiply instead of failing on its unit; `2g0` and `2 * g0` multiply
use crate::common::fails_with;
use warp::wasm_emitter::eval;

#[test]
fn a_spaced_number_before_a_quantity_says_how_to_multiply() {
	fails_with("g0 := 9.81 m/s²\n2 g0", "write 2g0 or 2 * g0");
	fails_with("q = 2 m\n3 q", "write 3q or 3 * q");
	assert_eq!(eval("g0 := 9.81 m/s²\n2g0").serialize().trim(), "19.62m/s²");
	assert_eq!(eval("q := 2 m\n3 * q").serialize().trim(), "6m");
}
