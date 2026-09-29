// Superscript digits are a power operator, vulgar fractions are exact ratios
use warp::*;

#[test]
fn test_multi_digit_superscript_exponent() {
	is!("2¹⁰", 1024);
	is!("x=2;x⁴", 16);
	is!("3⁴+1", 82);
}

#[test]
fn test_vulgar_fractions_are_exact() {
	is!("½+½", 1);
	is!("⅔*3", 2);
	is!("⅓+⅓+⅓==1", true);
	is!("⅞", 0.875);
}

#[test]
fn test_number_glyphs_at_end_of_input() {
	is!("3⁴", 81);
	is!("⅓*3", 1);
}
