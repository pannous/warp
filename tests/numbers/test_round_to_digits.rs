// `round(x, n)` and `x.round(n)`: x rounded to n digits after the point (samples/neural_net.wasp); round(x) stays whole
use warp::is;

#[test]
#[allow(clippy::approx_constant)] // 3.14159 is the wasp literal under test, not π
fn round_to_digits() {
	is!("round(3.14159, 3)", 3.142);
	is!("x=3.14159; x.round(3)", 3.142);
	is!("x=sqrt(2); round(x, 2)", 1.41);
}

#[test]
fn round_without_digits_is_whole() {
	is!("round(7.5)", 8);
}
