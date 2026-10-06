// `a // b` is the Euclidean quotient that goes with `%` (a == b*(a//b) + a%b): floor(a/b) for a positive divisor,
// ceil(a/b) for a negative one. Each operand runs once, an exact quotient rounds exactly beyond the f64 range, and no
// Euclidean-% warning is given for code the user never wrote
use crate::is;

#[test]
fn floor_division_is_euclidean() {
	is!("7//-2", -3);
	is!("-7//2", -4);
	is!("7.5//2", 3);
	is!("a=-7; b=-2; a//b * b + a%b == a", 1);
}

#[test]
fn the_dividend_is_evaluated_once() {
	is!("global n=0; f() := { n = n + 1; 7 }; f()//2; n", 1);
}

#[test]
fn floor_of_an_exact_number_beyond_f64() {
	is!("floor(10^30/3) == 333333333333333333333333333333", 1);
	is!("x=7; floor(x/-2)", -4);
}

#[test]
fn floor_division_warns_nothing_about_percent() {
	let _ = warp::diagnostic::take_warnings();
	is!("-7//2", -4);
	assert!(warp::diagnostic::take_warnings().iter().all(|warning| !warning.to_string().contains('%')));
}
