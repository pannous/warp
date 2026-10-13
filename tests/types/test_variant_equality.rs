// card variant-left: a variant without payload compares from either side of `==`; it is a value, never the undefined
// name the `x is 5` hint (P61) teaches `x be 5` for
use crate::is;

const SHAPE: &str = "enum Shape { Circle(r), Dot, Box }";

fn with_shape(code: &str) -> String {
	format!("{SHAPE}; {code}")
}

#[test]
fn a_variant_compares_on_the_left() {
	is!(&with_shape("x = Dot; Dot == x"), true);
	is!(&with_shape("x = Box; Dot == x"), false);
	is!(&with_shape("x = Box; Dot != x"), true);
}

#[test]
fn two_variants_compare_by_name() {
	is!(&with_shape("Dot == Dot"), true);
	is!(&with_shape("Dot == Box"), false);
}
