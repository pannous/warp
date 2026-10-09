// Math words over an interval `x ± r` at run time (card plus-minus-playground, decision P217): a function maps the
// endpoints, an extremum or a pole inside the interval is its bound (notes/plus_minus.md)
use warp::wasm_emitter::eval;

#[test]
fn a_math_word_maps_the_endpoints() {
	// √3..√5
	assert_eq!(eval("x = 4 ± 1; str(sqrt(x))"), "2.00 ± 0.27");
	assert_eq!(eval("x = 4 ± 1; str(√x)"), "2.00 ± 0.27");
	assert_eq!(eval("f(y) := sqrt(y); str(f(4 ± 1))"), "2.00 ± 0.27");
	assert_eq!(eval("x = 0 ± 1; y = exp(x); y.low"), (-1f64).exp());
	assert_eq!(eval("x = 0 ± 1; y = exp(x); y.high"), 1f64.exp());
}

#[test]
fn an_extremum_inside_the_interval_is_its_bound() {
	// π/2 lies in 1.3..1.7
	assert_eq!(eval("x = 1.5 ± 0.2; y = sin(x); y.high"), 1.0);
	assert_eq!(eval("x = 1.5 ± 0.2; y = sin(x); y.low"), 1.3f64.sin());
	assert_eq!(eval("x = 0 ± 0.5; y = cos(x); y.high"), 1.0);
	assert_eq!(eval("x = 0 ± 0.5; y = cos(x); y.low"), 0.5f64.cos());
	assert_eq!(eval("x = 1.5 ± 0.2; y = tan(x); y.high"), f64::INFINITY);
	assert_eq!(eval("x = 1.5 ± 0.2; y = tan(x); y.low"), f64::NEG_INFINITY);
	assert_eq!(eval("x = 0 ± 1; y = cosh(x); y.low"), 1.0);
	assert_eq!(eval("x = 0.5 ± 1; y = abs(x); y.low"), 0.0);
	assert_eq!(eval("x = 0.5 ± 1; y = abs(x); y.high"), 1.5);
}

#[test]
fn a_plain_number_still_takes_the_math_word() {
	assert_eq!(eval("x = 1 ± 0.1; f(y) := sqrt(y); f(4.0)"), 2.0);
	assert_eq!(eval("x = 1 ± 0.1; f(y) := sin(y); f(0.5)"), 0.5f64.sin());
	assert_eq!(eval("f(y) := sin(y); f(0.5)"), 0.5f64.sin());
}
