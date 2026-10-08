// `x ± r` at run time, an interval with its value that flows through arithmetic and functions (card plus-minus,
// notes/plus_minus.md, decisions P217–P219): worst-case bounds, the ± part to 2 significant digits, the value to the same place
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn bounds_propagate_through_arithmetic() {
	assert_eq!(shown("x = 5 ± 1; y = 2 ± 1; x + y"), "7.0 ± 2.0");
	assert_eq!(shown("x = 5 ± 1; x + 2"), "7.0 ± 1.0");
	assert_eq!(shown("x = 5 ± 1; x * 2"), "10.0 ± 2.0");
	assert_eq!(shown("x = 6 ± 0.3; y = 2 ± 0.2; x / y"), "3.00 ± 0.50");
}

#[test]
fn intervals_are_not_correlated() {
	assert_eq!(shown("x = 5 ± 1; x - x"), "0.0 ± 2.0");
}

#[test]
fn untyped_functions_pass_an_interval_through() {
	assert_eq!(shown("f(x) := x*x; f(3 ± 0.1)"), "9.00 ± 0.61");
}

#[test]
fn approximately_within_the_tolerance() {
	assert_eq!(eval("r = 11.5; 12 ≈ r ± 1"), true);
	assert_eq!(eval("r = 10.5; 12 ≈ r ± 1"), false);
}
