//! The standard library module uncertain (card uncertain-warp): `x ± r` as the warp classes Interval and Gaussian with
//! operator methods, the same numbers as the built-in ± (tests/numbers/test_plus_minus*.rs)
use crate::is;

fn shown(program: &str) -> String {
	format!("use uncertain; {program}")
}

#[test]
fn intervals_take_the_worst_case() {
	is!(&shown("x = plus_minus(5, 1); y = plus_minus(2, 1); str(x + y)"), "7.0 ± 2.0");
	is!(&shown("x = plus_minus(5, 1); str(x - x)"), "0.0 ± 2.0");
	is!(&shown("x = plus_minus(5, 1); str(x * 2)"), "10.0 ± 2.0");
	is!(&shown("x = plus_minus(6, 0.3); y = plus_minus(2, 0.2); str(x / y)"), "3.00 ± 0.50");
}

#[test]
fn intervals_compare_certainly() {
	is!(&shown("plus_minus(1, 0.5) < plus_minus(3, 0.5)"), true);
	is!(&shown("plus_minus(1, 1.5) < plus_minus(3, 1.5)"), false);
	is!(&shown("plus_minus(1, 1.5).possibly_less(plus_minus(3, 1.5))"), true);
	is!(&shown("plus_minus(11.5, 1) ≈ 12"), true);
}

#[test]
fn gaussians_add_in_quadrature() {
	is!(&shown("x = gaussian(5, 1); y = gaussian(2, 1); str(x + y)"), "7.0 ± 1.4σ");
	is!(&shown("x = gaussian(6, 0.3); y = gaussian(2, 0.2); str(x * y)"), "12.0 ± 1.3σ");
	is!(&shown("x = gaussian(5, 1); str(x * 2)"), "10.0 ± 2.0σ");
}

#[test]
fn a_gaussian_met_twice_is_one_error() {
	is!(&shown("x = gaussian(5, 1); str(x + x)"), "10.0 ± 2.0σ");
}
