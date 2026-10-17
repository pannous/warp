// card nearest-default-limit (user decision 2026-10-14): nearest caps the denominator at 1000000 as Python's
// limit_denominator does, limit 0 asks for the exact fraction, and a precision replaces the default limit
use crate::is;

#[test]
fn a_float_gives_pythons_fraction() {
	is!("x = sqrt(2.0); nearest(x, rational) == 665857/470832", true);
	is!("x = 3.141592653589793; nearest(x, rational) == 3126535/995207", true);
}

// the exact √2 lies a hair closer to the semiconvergent 941664/665857 than to the float's 665857/470832

#[test]
fn limit_zero_is_the_exact_fraction() {
	is!("nearest(sqrt(2), rational, 0) == 131836323/93222358", true);
	is!("nearest(π, rational, limit: 0) == 245850922/78256779", true);
	is!("x = sqrt(2.0); nearest(x, rational, 0) == 131836323/93222358", true);
}

#[test]
fn a_precision_reaches_beyond_the_default_limit() {
	is!("nearest(sqrt(2), rational, within: 1e-12) == 1607521/1136689", true);
}
