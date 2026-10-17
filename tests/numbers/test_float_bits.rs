use crate::is;

// card f64-const: the IEEE 754 bits of a float, what an emitter written in warp needs for `f64.const`

#[test]
fn float_bits_gives_the_ieee_bits_of_a_float() {
	is!("float_bits(1.0)", 4607182418800017408i64);
	is!("float_bits(-0.5)", -4620693217682128896i64);
	is!("float_bits(0.0)", 0);
	is!("float_bits(2)", 4611686018427387904i64); // an Int is its float
	is!("x := 0.1; float_bits(x) >> 52", 1019);
}

#[test]
fn the_top_bits_are_the_biased_exponent() {
	is!("float_bits(1.5)", 4609434218613702656i64);
	is!("float_bits(1.5) >> 52", 1023);
}

#[test]
fn bits_float_gives_the_float_of_ieee_bits() {
	is!("bits_float(4607182418800017408)", 1.0);
	is!("bits_float(-4620693217682128896)", -0.5);
	is!("bits_float(float_bits(3.25))", 3.25);
	is!("f(x) := bits_float(float_bits(x)); f(0.1) + 1", 1.1);
}
