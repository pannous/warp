// A ratio divided by a float, and a float by a ratio: 1/2 ÷ 2.0 is 0.25 (it was 1.0: the ratio times the float)
use warp::extensions::numbers::Number;

#[test]
fn a_ratio_divides_by_a_float() {
	assert_eq!(Number::Quotient(1, 2) / Number::Float(2.0), Number::Float(0.25));
	assert_eq!(Number::Float(3.0) / Number::Quotient(3, 4), Number::Float(4.0));
}
