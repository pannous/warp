// A Gaussian ± through the explicit form `5 ± 1σ`: one standard deviation, independent errors add in quadrature
// (card plus-minus-gaussian, decision P217: a bare `5 ± 1` stays an interval; notes/plus_minus.md)
use crate::common::fails_with;
use warp::wasm_emitter::eval;

#[test]
fn independent_errors_add_in_quadrature() {
	assert_eq!(eval("x = 5 ± 1σ; y = 2 ± 1σ; str(x + y)"), "7.0 ± 1.4σ");
	assert_eq!(eval("x = 5 ± 1σ; y = 2 ± 1σ; str(x - y)"), "3.0 ± 1.4σ");
	assert_eq!(eval("x = 6 ± 0.3σ; y = 2 ± 0.2σ; str(x * y)"), "12.0 ± 1.3σ");
	assert_eq!(eval("x = 6 ± 0.3σ; y = 2 ± 0.2σ; str(x / y)"), "3.00 ± 0.34σ");
}

#[test]
fn a_number_scales_the_deviation() {
	assert_eq!(eval("x = 5 ± 1σ; str(x * 2)"), "10.0 ± 2.0σ");
	assert_eq!(eval("x = 5 ± 1σ; str(x + 1)"), "6.0 ± 1.0σ");
	assert_eq!(eval("x = 5 ± 1σ; str(10 - x)"), "5.0 ± 1.0σ");
}

#[test]
fn a_math_word_scales_the_deviation_by_its_slope() {
	assert_eq!(eval("x = 4 ± 0.4σ; str(√x)"), "2.00 ± 0.10σ");
	assert_eq!(eval("x = 0 ± 0.1σ; str(sin(x))"), "0.00 ± 0.10σ");
	assert_eq!(eval("f(x) := x * x; str(f(3 ± 0.1σ))"), "9.00 ± 0.60σ");
}

// Measurements.jl's correlation: a value met twice is one error, not two independent ones
#[test]
fn a_value_met_twice_is_one_error() {
	assert_eq!(eval("x = 5 ± 1σ; str(x - x)"), "0 ± 0σ");
	assert_eq!(eval("x = 5 ± 1σ; str(x + x)"), "10.0 ± 2.0σ");
	assert_eq!(eval("x = 1 ± 0.1σ; y = 2 ± 0.1σ; z = x + y; str(z - x)"), "2.00 ± 0.10σ");
}

// P219's two digits: a ± part just under a power of ten rounds up to it
#[test]
fn a_spread_just_under_a_power_of_ten_shows_two_digits() {
	assert_eq!(eval("x = 0 ± 0.0999999; str(x + 0)"), "0.00 ± 0.10");
}

#[test]
fn the_parts_of_a_gaussian() {
	assert_eq!(eval("x = 5 ± 1σ; y = x * 2; y.value"), 10.0);
	assert_eq!(eval("x = 5 ± 1σ; y = x * 2; y.uncertainty"), 2.0);
	assert_eq!(eval("x = 5 ± 1σ; y = x * 2; y.low"), 8.0);
	assert_eq!(eval("x = 5 ± 1σ; y = x * 2; y.high"), 12.0);
}

#[test]
fn a_gaussian_reads_back_as_it_is_written() {
	assert_eq!(eval("x = 5 ± 1σ; x + 1").serialize(), "6.0 ± 1.0σ");
	assert_eq!(eval("x = 5 ± 1; x + 1").serialize(), "6.0 ± 1.0");
}

#[test]
fn an_interval_and_a_gaussian_do_not_mix() {
	fails_with("x = 5 ± 1σ; y = 2 ± 1; x + y", "interval and a gaussian");
	fails_with("x = 5 ± 1; y = 2 ± 1σ; x * y", "interval and a gaussian");
}
