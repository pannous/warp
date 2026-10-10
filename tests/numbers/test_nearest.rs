// card nearest-helper: `nearest(x, rational, limit)` is the fraction closest to x whose denominator is at most limit
// (10^6 when left out), by continued fractions as Python's Fraction.limit_denominator (lib/prelude.warp)
use crate::common::fails_with;
use crate::is;

#[test]
fn nearest_fraction() {
	is!("nearest(sqrt(2), rational) == 665857/470832", true);
	is!("nearest(sqrt(2), rational, 1000) == 1393/985", true);
	is!("nearest(pi, rational, 100) == 311/99", true);
	is!("type(nearest(sqrt(2), rational))", "rational");
}

#[test]
fn nearest_of_exact_values() {
	is!("nearest(0.75, rational) == 3/4", true);
	is!("nearest(-2.5, rational) == -5/2", true);
	is!("nearest(3.0, rational)", 3);
	is!("x = sqrt(2); x.nearest(rational, 1000) == 1393/985", true);
}

#[test]
fn nearest_takes_two_or_three_arguments() {
	fails_with("nearest(1.5, rational, 1, 2)", "nearest takes 2 to 3 arguments, got 4");
}

#[test]
fn float_to_exact_refusals_name_nearest() {
	fails_with("x: rational = sqrt(2)", "truncate with `as int`, or use nearest(√(2), rational) for the closest fraction");
	fails_with("sqrt(2) as rational", "or use nearest(√(2), rational) for the closest fraction");
}
