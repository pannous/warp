use crate::is;
use crate::common;

#[test]
fn test_float_modulo_keeps_fraction() {
	is!("f(x:float) := x % 2; f(2.5)", 0.5);
	is!("f(x:float) := x % 2; f(-0.5)", 1.5);
}

#[test]
fn test_float_comparison_does_not_truncate() {
	is!("f(x:float) := x == 2; f(2.7)", false);
	is!("f(x:float) := x > 1 ? 1 : 0; f(1.5)", 1);
	is!("f(x:float) := x < 2; f(1.5)", true);
}

#[test]
fn test_float_power_keeps_fraction() {
	is!("f(x:float) := x ^ 2; f(1.5)", 2.25);
	is!("f(x:float) := x ^ -2; f(2.0)", 0.25);
}

#[test]
fn test_literal_float_operations_do_not_truncate() {
	is!("2.7 == 2", false);
	is!("1.5 > 1", true);
	is!("1.5 ^ 2", 2.25);
	is!("2.7 % 2", 0.7);
}

#[test]
fn test_fractional_exponent_uses_libm_pow() {
	is!("f(x:float) := 2.0 ^ x; f(0.5)", std::f64::consts::SQRT_2);
	is!("f(x:float) := x ^ 0.5; f(4.0)", 2.0);
	is!("2 ^ 0.5", std::f64::consts::SQRT_2);
	is!("2 ^ 3", 8);
}

#[test]
fn test_undefined_float_power_is_refused_loudly() {
	common::fails_with("f(x:float) := (0 - 2.0) ^ x; f(0.5)", "invalid number");
	common::fails_with("y = 0.5; 2 ^ y", "integer exponent");
}
