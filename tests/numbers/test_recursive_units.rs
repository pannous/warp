//! A recursive function with quantities specialises like any other: its result signature comes from the base case and
//! must agree with what the recursive branch gives (card static-units)
use crate::common::fails_with;
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize().trim().to_string()
}

#[test]
fn a_recursive_function_takes_quantities() {
	assert_eq!(shown("fact(n, x) := if n < 1 { x } else { fact(n - 1, x + 1 m) }; fact(3, 0 m)"), "3m");
	assert_eq!(shown("fact(n, d) := if n < 1 { d } else { fact(n - 1, d + 1 m) }; fact(3, 0 m)"), "3m");
	assert_eq!(shown("walk(n, x) := if n > 0 { walk(n - 1, x + 2 m) } else { x }; walk(2, 1 m)"), "5m");
	assert_eq!(shown("total(n) := if n < 1 { 0 km } else { 500 m + total(n - 1) }; total(3)"), "1500m");
}

#[test]
fn a_recursive_function_keeps_its_dimensions() {
	fails_with("f(n, x) := if n < 1 { x } else { f(n - 1, x + 1 s) }; f(2, 0 m)", "DimensionError");
	// the result's units would depend on n
	fails_with("p(n, x) := if n < 1 { 1 } else { x * p(n - 1, x) }; p(2, 3 m)", "DimensionError");
}
