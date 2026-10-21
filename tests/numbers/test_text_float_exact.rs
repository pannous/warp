//! card text-float: a text read as a float at run time is the float of its literal, "+3.5e-2" is 0.035
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_text_read_as_a_float_at_run_time_is_its_literal() {
	assert_eq!(printed("f(t) := t as float; f(\"+3.5e-2\")"), "0.035");
	assert_eq!(printed("f(t) := float(t); f(\"0.035\")"), "0.035");
	assert_eq!(printed("f(t) := float(t); f(\"-12.5e3\")"), "-12500");
	assert_eq!(printed("f(t) := float(t); f(\"1.7e-5\")"), "0.000017");
}

#[test]
fn a_text_of_a_tiny_float_is_no_zero() {
	assert_eq!(printed("f(t) := float(t); f(\"1e-310\")"), "1e-310");
	assert_eq!(printed("f(t) := float(t); f(\"2.5e-320\")"), "2.5e-320");
}

/// card pure-lane-prints: digits beyond what a double holds are read to the nearest float, not rounded digit by digit
/// (parser.warp reads its float literals so, which made the pure lane's 8.33333333332248946124e+03 one ulp off)
#[test]
fn a_text_of_many_digits_is_the_nearest_float() {
	assert_eq!(printed("f(t) := float(t); f(\"8333.33333332248946124\")"), "8333.33333332249");
	assert_eq!(printed("f(t) := float(t); f(\"8.33333333332248946124e+03\")"), "8333.33333332249");
	assert_eq!(printed("f(t) := float(t); f(\"0.1000000000000000055511151231257827\")"), "0.1");
	assert_eq!(printed("f(t) := float(t); f(\"123456789012345678901234567890\")"), "1.2345678901234568e29");
}
