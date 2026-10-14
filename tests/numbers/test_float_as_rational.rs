//! `x as rational` of a float is the exact fraction of its shortest printed digits, reduced (decision float-as-rational)
use crate::common::fails_with;
use crate::is;

#[test]
fn test_float_as_rational() {
	is!("sqrt(2.0) as rational == 14142135623730951/10000000000000000", true);
	is!("rational(sqrt(2.0)) == 14142135623730951/10000000000000000", true);
	is!("x = 0.1 as float; x as rational == 1/10", true);
	is!("x = -2.5 as float; x as rational == -5/2", true);
	is!("x = 1e20 as float; x as rational == 10^20", true);
	is!("x = 1e-20 as float; x as rational == 1/10^20", true);
	is!("x = 0.0 as float; x as rational == 0", true);
	is!("type(sqrt(2.0) as rational)", "rational");
}

/// card float-rational-crash: a program without exact numbers otherwise had no int runtime for float_as_exact
#[test]
fn test_float_as_rational_alone() {
	is!("x = sin(1); y = x as rational; type(y)", "rational");
}

#[test]
fn test_infinite_float_as_rational() {
	fails_with("x = 1e308 as float; y = x * 10; y as rational", "not a finite number");
}
