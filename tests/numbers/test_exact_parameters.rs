//! A parameter typed `real`, `exact` or `rational` takes a decimal literal: decimals are exact numbers, only a whole
//! type (`int`) refuses them (samples/sample.wasp `fun calculate(a: int, b: real)`)
use crate::is;

#[test]
fn an_exact_parameter_takes_a_decimal() {
	is!("g(b: real) := b; g(2.5)", 2.5);
	is!("g(b: exact) := b * 2; g(2.5)", 5);
	is!("fun calculate(a: int, b: real) = a + b * 2; calculate(1, 2.5)", 6);
	is!("g(b: rational) := b; g(0.75) == 3/4", true);
}

#[test]
fn float64_is_the_word_float() {
	is!("float64 ratio = 1.5; ratio * 2", 3.0);
}
