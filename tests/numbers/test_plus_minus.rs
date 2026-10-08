// `x ± σ` at run time, a value with uncertainty that flows through arithmetic and functions like Julia's Measurements
// (card plus-minus, notes/plus_minus.md): linear propagation, σ to 2 significant digits, the value to the same place
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn uncertainty_propagates_through_arithmetic() {
	assert_eq!(shown("x = 5 ± 1; y = 2 ± 1; x + y"), "7.0 ± 1.4");
	assert_eq!(shown("x = 5 ± 1; x + 2"), "7.0 ± 1.0");
	assert_eq!(shown("x = 5 ± 1; x * 2"), "10.0 ± 2.0");
	assert_eq!(shown("x = 6 ± 0.3; y = 2 ± 0.2; x / y"), "3.00 ± 0.34");
}

#[test]
fn one_source_is_correlated_with_itself() {
	assert_eq!(shown("x = 5 ± 1; x - x"), "0 ± 0");
	assert_eq!(shown("f(x) := x*x; f(3 ± 0.1)"), "9.00 ± 0.60");
}

#[test]
fn approximately_within_the_uncertainty() {
	assert_eq!(eval("r = 11.5; 12 ≈ r ± 1"), true);
	assert_eq!(eval("r = 10.5; 12 ≈ r ± 1"), false);
}
