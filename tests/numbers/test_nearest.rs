// card nearest-helper: `nearest(x, rational, limit)` is the fraction closest to x whose denominator is at most limit, as
// Python's Fraction.limit_denominator; `within: ε` the first convergent that close; neither the first convergent that is
// the same float (user, 2026-10-10: a fixed 10^6 default was "hard to believe"). An exact √2 or π expands exactly
// (src/real.rs), a float at run time by lib/prelude.warp
use crate::common::fails_with;
use crate::is;

#[test]
fn nearest_fraction() {
	is!("nearest(sqrt(2), rational) == 131836323/93222358", true);
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
fn nearest_takes_two_to_four_arguments() {
	fails_with("nearest(1.5, rational, 1, 0.1, 2)", "nearest takes 2 to 4 arguments, got 5");
}

#[test]
fn float_to_exact_refusals_name_nearest() {
	fails_with("x: rational = sqrt(2)", "truncate with `as int`, or use nearest(√(2), rational) for the closest fraction");
	fails_with("sqrt(2) as rational", "or use nearest(√(2), rational) for the closest fraction");
}

#[test]
fn nearest_within_a_precision() {
	is!("nearest(sqrt(2), rational, within: 1e-6) == 1393/985", true);
	is!("x = sqrt(2); nearest(x, rational, within: 1e-6) == 1393/985", true);
	is!("x = sqrt(2); nearest(x, rational, within = 1e-9) == 47321/33461", true);
	is!("x = sqrt(2); nearest(x, rational) == 131836323/93222358", true);
}

#[test]
fn nearest_expands_exact_values_beyond_floats() {
	// p² - 2q² = -1: a convergent of √2 itself, not of its float
	is!("r = nearest(sqrt(2), rational, within: 1e-40); r * r - 2 == -1/4689566069222821420312720463003656425961", true);
	is!("nearest(sqrt(2), rational, 10^38) == 133984184101103275326877813426364627544/94741125149636933417873079920900017937", true);
	is!("nearest(π, rational) == 245850922/78256779", true);
}

// card nearest-rational: nearest is a method too, of a variable and of a call's result (card sqrt-round)
#[test]
fn nearest_as_a_method() {
	is!("sqrt(2).nearest(rational, within: 0.001) == 41/29", true);
	is!("type(sqrt(2).nearest(rational))", "rational");
	is!("x = 0.1; x.nearest(rational) == 1/10", true);
	is!("x = sqrt(2.0); x.nearest(rational, within: 0.001) == 41/29", true);
}
