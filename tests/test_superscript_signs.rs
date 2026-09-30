//! Superscript signs and digits are one power: `10⁻¹` is 1/10, `x²⁺³` is x⁵

use warp::is;

#[test]
fn a_negative_superscript_is_a_reciprocal_power() {
	is!("10⁻¹ == 0.1", true);
	is!("2⁻² == 0.25", true);
	is!("2⁻¹ * 4", 2);
}

#[test]
fn a_negative_superscript_on_a_variable() {
	is!("x=2; x⁻¹ == 0.5", true);
}

#[test]
fn a_superscript_plus_adds_to_the_exponent_chain() {
	is!("x=2; x²⁺³", 32);
	is!("x=2; x⁺³", 8);
}

#[test]
fn superscript_digits_without_sign_still_work() {
	is!("x=2; x⁵", 32);
	is!("x=3; x²", 9);
}
