// Arithmetic of any two Numbers gives a Number, never a panic (code_quality #13): a complex operand makes the result
// complex, an exact real keeps it exact where it can, ∞ and NaN follow IEEE
use warp::extensions::numbers::Number;
use warp::extensions::reals::{Exact, Real};

#[test]
fn complex_mixes_with_any_number() {
	assert_eq!(Number::Complex(1.0, 2.0) + Number::Int(1), Number::Complex(2.0, 2.0));
	assert_eq!(Number::Float(0.5) * Number::Complex(2.0, 4.0), Number::Complex(1.0, 2.0));
	assert_eq!(Number::Complex(2.0, 2.0) - Number::Quotient(1, 2), Number::Complex(1.5, 2.0));
	assert_eq!(Number::Complex(2.0, 4.0) / Number::Int(2), Number::Complex(1.0, 2.0));
	assert_eq!(f64::from(Number::Complex(3.0, 0.0)), 3.0);
	assert!(f64::from(Number::Complex(1.0, 2.0)).is_nan());
}

#[test]
fn infinity_and_nan_follow_ieee() {
	assert_eq!(Number::Inf + Number::Int(1), Number::Inf);
	assert_eq!(Number::Int(2) * Number::NegInf, Number::NegInf);
	assert!(matches!(Number::Nan - Number::Float(1.0), Number::Nan));
	assert!(matches!(Number::Quotient(1, 3) + Number::Float(f64::NAN), Number::Nan));
	assert_eq!(Number::Quotient(1, 2) * Number::Inf, Number::Inf);
}

#[test]
fn an_exact_real_stays_exact_with_a_ratio() {
	let pi = Number::real(Real::Exact(Exact::pi()));
	let sum = pi + Number::Quotient(1, 3);
	assert!(matches!(sum, Number::Real(Real::Exact(_))), "{sum}");
	assert_eq!(sum - Number::Quotient(1, 3), pi);
	let approximate = pi + Number::Float(0.25);
	assert!((f64::from(approximate) - (std::f64::consts::PI + 0.25)).abs() < 1e-12, "{approximate}");
}
