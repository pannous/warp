use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::ops::{Add, Div, Mul, Neg, Sub};
use num_bigint::BigInt;
use super::reals::{Exact, Rational, Real};
use num_integer::Integer;
use num_traits::{One, Pow, Signed, ToPrimitive, Zero};

fn deserialize_leaked_real<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<&'static Real, D::Error> {
	Real::deserialize(deserializer).map(|real| &*Box::leak(Box::new(real)))
}

fn deserialize_leaked_bigint<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<&'static BigInt, D::Error> {
	BigInt::deserialize(deserializer).map(|big| &*Box::leak(Box::new(big)))
}

fn deserialize_leaked_rational<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<&'static Rational, D::Error> {
	Rational::deserialize(deserializer).map(|ratio| &*Box::leak(Box::new(ratio)))
}

// PartialEq per hand!
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Number {
	Nan,
	// True,
	// False,
	Inf,
	NegInf,
	Int(i64),
	Float(f64),
	Quotient(i64, i64),
	Complex(f64, f64),
	/// Integer beyond i64: always normalized, a value that fits i64 is `Int`.
	/// Leaked to keep Number Copy: every distinct big value costs its memory for the process lifetime.
	BigInt(#[serde(deserialize_with = "deserialize_leaked_bigint")] &'static BigInt),
	/// Exact ratio whose numerator or denominator exceeds i64. Leaked like BigInt to keep Number Copy.
	/// A ratio that fits in two i64 is always `Quotient`.
	BigQuotient(#[serde(deserialize_with = "deserialize_leaked_rational")] &'static Rational),
	/// Exact real beyond Q (2√2, π/2, 3+√2) or a marked approximation (≈0.841…), see reals.rs.
	/// Rationals never take this form. Leaked like BigInt to keep Number Copy.
	Real(#[serde(deserialize_with = "deserialize_leaked_real")] &'static Real),
	// use num_traits::{One, Zero};
	// Number::BigNum(_) => unimplemented!(),
	// Hyper(Vec<Pair<f64,f64>>)
	// Hyper Hyperreal with epsilon infinitesimal and omega infinite parts
	// other variants as needed
}

impl Number {
	pub fn zero(&self) -> bool {
		match self {
			Number::Int(i) => *i == 0,
			Number::BigInt(b) => b.is_zero(),
			Number::Quotient(n, _d) => *n == 0,
			Number::BigQuotient(q) => q.is_zero(),
			Number::Complex(r, i) => *r == 0.0 && *i == 0.0,
			Number::Float(f) => *f == 0.0,
			Number::Real(r) => r.is_zero(),
			Number::Nan | Number::Inf | Number::NegInf => false,
		}
	}
	pub fn abs(&self) -> f64 {
		match self {
			Number::Int(i) => i.abs() as f64,
			Number::BigInt(b) => b.to_f64().unwrap_or(f64::INFINITY).abs(),
			Number::Quotient(n, d) => (*n as f64 / *d as f64).abs(),
			Number::BigQuotient(q) => q.to_f64().abs(),
			Number::Complex(r, i) => (r * r + i * i).sqrt(),
			Number::Float(f) => f.abs(),
			Number::Real(r) => r.to_f64().abs(),
			Number::Nan => f64::NAN,
			Number::Inf => f64::INFINITY,
			Number::NegInf => f64::NEG_INFINITY,
		}
	}
}

impl Number {
	/// Normalize: an integer that fits i64 is always `Int`, only larger ones are `BigInt`
	pub fn from_bigint(big: BigInt) -> Number {
		big.to_i64().map(Number::Int).unwrap_or_else(|| Number::BigInt(Box::leak(Box::new(big))))
	}

	pub fn real(real: Real) -> Number {
		Number::Real(Box::leak(Box::new(real)))
	}

	pub fn is_integer(&self) -> bool {
		matches!(self, Number::Int(_) | Number::BigInt(_))
	}

	fn is_ratio(&self) -> bool {
		matches!(self, Number::Quotient(..) | Number::BigQuotient(_))
	}

	/// An integer or a ratio: exact in Q, what to_rational takes
	fn is_rational(&self) -> bool {
		self.is_integer() || self.is_ratio()
	}

	/// An f64 result as a Number: ±∞ and NaN are their own variants
	fn of_f64(value: f64) -> Number {
		match value {
			f64::INFINITY => Number::Inf,
			f64::NEG_INFINITY => Number::NegInf,
			nan if nan.is_nan() => Number::Nan,
			finite => Number::Float(finite),
		}
	}

	pub fn to_bigint(&self) -> BigInt {
		match self {
			Number::Int(i) => BigInt::from(*i),
			Number::BigInt(b) => (*b).clone(),
			other => panic!("not an integer: {}", other),
		}
	}

	/// Exact numerator/denominator in lowest terms: an integral value is an integer, n/0 is ±∞ or NaN.
	/// Parts beyond i64 become `BigQuotient` (exact), never a silent Float.
	pub fn ratio(numerator: Number, denominator: Number) -> Number {
		let (numerator, denominator) = match (&numerator, &denominator) {
			(Number::Quotient(..) | Number::BigQuotient(_), _) | (_, Number::Quotient(..) | Number::BigQuotient(_)) => {
				let left = numerator.to_rational();
				let right = denominator.to_rational();
				(left.numerator * right.denominator, left.denominator * right.numerator)
			}
			_ => (numerator.to_bigint(), denominator.to_bigint()),
		};
		if denominator.is_zero() {
			return match numerator.signum().to_i64() {
				Some(1) => Number::Inf,
				Some(-1) => Number::NegInf,
				_ => Number::Nan,
			};
		}
		Number::from_rational(Rational::new(numerator, denominator))
	}

	/// Normalize: a ratio that fits two i64 is `Quotient`, only larger ones are `BigQuotient`
	pub fn from_rational(ratio: Rational) -> Number {
		if ratio.is_integer() {
			return Number::from_bigint(ratio.numerator);
		}
		match (ratio.numerator.to_i64(), ratio.denominator.to_i64()) {
			(Some(n), Some(d)) => Number::Quotient(n, d),
			_ => Number::BigQuotient(Box::leak(Box::new(ratio))),
		}
	}

	pub fn to_rational(&self) -> Rational {
		match self {
			Number::Int(i) => Rational::integer(*i),
			Number::BigInt(b) => Rational::integer((*b).clone()),
			Number::Quotient(n, d) => Rational::new(BigInt::from(*n), BigInt::from(*d)),
			Number::BigQuotient(q) => (*q).clone(),
			other => panic!("not a rational: {}", other),
		}
	}

	/// `n/d` as an exact decimal when it terminates (denominator 2^a·5^b): 1/8 → "0.125", 1/3 → None
	fn terminating_decimal(numerator: i64, denominator: i64) -> Option<String> {
		Number::terminating_decimal_big(BigInt::from(numerator), BigInt::from(denominator))
	}

	fn terminating_decimal_big(numerator: BigInt, denominator: BigInt) -> Option<String> {
		let divisor = numerator.gcd(&denominator) * denominator.signum();
		let (numerator, denominator) = (numerator / &divisor, denominator / &divisor);
		let (mut rest, mut digits) = (denominator.clone(), 0u32);
		for factor in [2i64, 5] {
			let factor = BigInt::from(factor);
			let mut count = 0;
			while (&rest % &factor).is_zero() {
				rest /= &factor;
				count += 1;
			}
			digits = digits.max(count);
		}
		if !rest.is_one() {
			return None;
		}
		let scaled = (numerator * BigInt::from(10).pow(digits) / denominator).to_string();
		let (sign, magnitude) = scaled.strip_prefix('-').map_or(("", scaled.as_str()), |m| ("-", m));
		let padded = format!("{:0>width$}", magnitude, width = digits as usize + 1);
		let (whole, fraction) = padded.split_at(padded.len() - digits as usize);
		Some(format!("{sign}{whole}.{fraction}"))
	}

	/// A decimal literal is exact when its shortest round-trip form has at most f64's 15 guaranteed
	/// significant digits: `0.1` is 1/10, while `π` (3.141592653589793) stays an f64 approximation
	pub fn is_exact_decimal(value: f64) -> bool {
		const F64_DECIMAL_DIGITS: usize = f64::DIGITS as usize;
		let scientific = format!("{:e}", value);
		let mantissa = scientific.split('e').next().unwrap_or_default();
		value.is_finite() && mantissa.chars().filter(char::is_ascii_digit).count() <= F64_DECIMAL_DIGITS
	}

	/// Integer literal of any size: 123456789012345678901234567890
	pub fn parse_integer(digits: &str) -> Option<Number> {
		digits.parse::<BigInt>().ok().map(Number::from_bigint)
	}

}

impl Display for Number {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Number::Int(i) => write!(f, "{}", i),
			Number::BigInt(b) => write!(f, "{}", b),
			Number::Float(fl) => write!(f, "{}", fl),
			Number::Quotient(numer, denom) => match Number::terminating_decimal(*numer, *denom) {
				Some(decimal) => write!(f, "{}", decimal),
				None => write!(f, "{}/{}", numer, denom),
			},
			Number::BigQuotient(q) => match Number::terminating_decimal_big(q.numerator.clone(), q.denominator.clone()) {
				Some(decimal) => write!(f, "{}", decimal),
				None => write!(f, "{}", q),
			},
			Number::Complex(real, imag) => write!(f, "{} + {}i", real, imag),
			Number::Real(real) => write!(f, "{}", real),
			Number::Nan => write!(f, "NaN"),
			Number::Inf => write!(f, "∞"),
			Number::NegInf => write!(f, "-∞"),
		}
	}
}

#[derive(Clone, Copy)]
enum Operation {
	Add,
	Sub,
	Mul,
	Div,
}

impl Operation {
	fn of_f64(self, a: f64, b: f64) -> f64 {
		match self {
			Operation::Add => a + b,
			Operation::Sub => a - b,
			Operation::Mul => a * b,
			Operation::Div => a / b,
		}
	}

	/// None on overflow
	fn of_i64(self, a: i64, b: i64) -> Option<i64> {
		match self {
			Operation::Add => a.checked_add(b),
			Operation::Sub => a.checked_sub(b),
			Operation::Mul => a.checked_mul(b),
			Operation::Div => unreachable!("an integer quotient is a ratio: Number::ratio"),
		}
	}

	fn of_bigint(self, a: BigInt, b: BigInt) -> BigInt {
		match self {
			Operation::Add => a + b,
			Operation::Sub => a - b,
			Operation::Mul => a * b,
			Operation::Div => unreachable!("an integer quotient is a ratio: Number::ratio"),
		}
	}

	fn of_rational(self, a: &Rational, b: &Rational) -> Rational {
		match self {
			Operation::Add => a.add(b),
			Operation::Sub => a.add(&b.neg()),
			Operation::Mul => a.mul(b),
			Operation::Div => unreachable!("a quotient of ratios is Number::ratio"),
		}
	}

	fn of_complex(self, (r1, i1): (f64, f64), (r2, i2): (f64, f64)) -> Number {
		let (real, imaginary) = match self {
			Operation::Add => (r1 + r2, i1 + i2),
			Operation::Sub => (r1 - r2, i1 - i2),
			Operation::Mul => (r1 * r2 - i1 * i2, r1 * i2 + i1 * r2),
			Operation::Div => {
				// (a + bi) / (c + di) = (a + bi)(c - di) / (c² + d²)
				let divisor = r2 * r2 + i2 * i2;
				((r1 * r2 + i1 * i2) / divisor, (i1 * r2 - r1 * i2) / divisor)
			}
		};
		Number::Complex(real, imaginary)
	}

	/// Exact where both operands are: ratios and exact reals add, subtract and multiply in normal form
	fn of_exact(self, a: &Exact, b: &Exact) -> Option<Exact> {
		match self {
			Operation::Add => Some(a.add(b)),
			Operation::Sub => Some(a.sub(b)),
			Operation::Mul => Some(a.mul(b)),
			Operation::Div => None,
		}
	}
}

fn exact_of(number: &Number) -> Option<Exact> {
	match number {
		Number::Real(Real::Exact(exact)) => Some(exact.clone()),
		rational if rational.is_rational() => Some(Exact::rational(rational.to_rational())),
		_ => None,
	}
}

/// The operands the exact arms of + - * / do not take, never a panic: a complex operand makes the result complex, an
/// exact real stays exact with a ratio or another exact real (else it is an approximation, Real::Approx), ∞ and NaN
/// follow IEEE
fn mixed(a: Number, b: Number, operation: Operation) -> Number {
	let complex = |number: Number| match number {
		Number::Complex(real, imaginary) => (real, imaginary),
		other => (f64::from(other), 0.0),
	};
	if matches!(a, Number::Complex(..)) || matches!(b, Number::Complex(..)) {
		return operation.of_complex(complex(a), complex(b));
	}
	if matches!(a, Number::Real(_)) || matches!(b, Number::Real(_)) {
		let exact = exact_of(&a).zip(exact_of(&b)).and_then(|(a, b)| operation.of_exact(&a, &b));
		return Number::real(exact.map_or_else(|| Real::Approx(operation.of_f64(f64::from(a), f64::from(b))), Real::Exact));
	}
	Number::of_f64(operation.of_f64(f64::from(a), f64::from(b)))
}

impl Neg for Number {
	type Output = Self;

	fn neg(self) -> Self::Output {
		match self {
			Number::Int(n) => n.checked_neg().map(Number::Int).unwrap_or_else(|| Number::from_bigint(-BigInt::from(n))),
			Number::BigInt(big) => Number::from_bigint(-big.clone()),
			Number::Float(f) => Number::Float(-f),
			Number::Quotient(n, d) => Number::Quotient(-n, d),
			Number::BigQuotient(q) => Number::from_rational(q.neg()),
			Number::Complex(r, i) => Number::Complex(-r, -i),
			Number::Real(r) => Number::real(r.neg()),
			Number::Inf => Number::NegInf,
			Number::NegInf => Number::Inf,
			Number::Nan => Number::Nan,
		}
	}
}

/// + - * of two numbers: exact for integers and ratios (an integer overflowing i64 becomes a BigInt), f64 for floats
/// (an infinite or NaN result stays a Float), else mixed
fn arithmetic(a: Number, b: Number, operation: Operation) -> Number {
	match (a, b) {
		(Number::Int(n1), Number::Int(n2)) => {
			operation.of_i64(n1, n2).map(Number::Int).unwrap_or_else(|| Number::from_bigint(operation.of_bigint(BigInt::from(n1), BigInt::from(n2))))
		}
		(a, b) if a.is_integer() && b.is_integer() => Number::from_bigint(operation.of_bigint(a.to_bigint(), b.to_bigint())),
		(a, b) if (a.is_ratio() || b.is_ratio()) && a.is_rational() && b.is_rational() => {
			Number::from_rational(operation.of_rational(&a.to_rational(), &b.to_rational()))
		}
		(Number::Float(n1), Number::Float(n2)) => Number::Float(operation.of_f64(n1, n2)),
		(Number::Int(n1), Number::Float(n2)) => Number::Float(operation.of_f64(n1 as f64, n2)),
		(Number::Float(n1), Number::Int(n2)) => Number::Float(operation.of_f64(n1, n2 as f64)),
		(a, b) => mixed(a, b, operation),
	}
}

impl Add for Number {
	type Output = Self;

	fn add(self, other: Self) -> Self::Output {
		arithmetic(self, other, Operation::Add)
	}
}

impl Sub for Number {
	type Output = Self;

	fn sub(self, other: Self) -> Self::Output {
		arithmetic(self, other, Operation::Sub)
	}
}

impl Mul for Number {
	type Output = Self;

	fn mul(self, other: Self) -> Self::Output {
		arithmetic(self, other, Operation::Mul)
	}
}

impl Div for Number {
	type Output = Self;

	fn div(self, other: Self) -> Self::Output {
		match (self, other) {
			(Number::Quotient(q1, q2), Number::Float(n2)) => {
				Number::Float(q1 as f64 / q2 as f64 * n2)
			}
			(Number::Float(n1), Number::Quotient(q1, q2)) => {
				Number::Float(n1 / q1 as f64 / q2 as f64)
			}
			(Number::BigQuotient(q), Number::Float(n2)) => Number::Float(q.to_f64() / n2),
			(Number::Float(n1), Number::BigQuotient(q)) => Number::Float(n1 / q.to_f64()),
			(a, b) if matches!(a, Number::Quotient(..) | Number::BigQuotient(_) | Number::Int(_) | Number::BigInt(_))
				&& matches!(b, Number::Quotient(..) | Number::BigQuotient(_) | Number::Int(_) | Number::BigInt(_)) => {
				Number::ratio(a, b)
			}
			(Number::Float(n1), Number::Float(n2)) => Number::Float(n1 / n2),
			(Number::Int(n1), Number::Float(n2)) => Number::Float(n1 as f64 / n2),
			(Number::Float(n1), Number::Int(n2)) => Number::Float(n1 / n2 as f64),
			(a, b) => mixed(a, b, Operation::Div),
		}
	}
}

impl From<Number> for f64 {
	fn from(val: Number) -> Self {
		match val {
			Number::Int(i) => i as f64,
			Number::BigInt(b) => b.to_f64().unwrap_or(f64::NAN),
			Number::Float(f) => f,
			Number::Quotient(numer, denom) => numer as f64 / denom as f64,
			Number::BigQuotient(q) => q.to_f64(),
			// a complex number is a real number only without an imaginary part
			Number::Complex(real, imaginary) => if imaginary == 0.0 { real } else { f64::NAN },
			Number::Real(r) => r.to_f64(),
			Number::Nan => f64::NAN,
			Number::Inf => f64::INFINITY,
			Number::NegInf => f64::NEG_INFINITY,
		}
	}
}

use std::cmp::PartialEq;

impl PartialEq for Number {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Number::Int(i1), Number::Int(i2)) => i1 == i2,
			(Number::BigInt(b1), Number::BigInt(b2)) => b1 == b2,
			// simple approximation:  f64 as f32
			(Number::Float(f1), Number::Float(f2)) => {
				// Handle NaN and Inf specially for semantic equality
				if f1.is_nan() && f2.is_nan() {
					true
				} else if f1.is_infinite() && f2.is_infinite() {
					f1.signum() == f2.signum() // both +Inf or both -Inf
				} else {
					*f1 as f32 == *f2 as f32
				}
			}
			// (Number::Float(f1), Number::Float(f2)) => *f1 == *f2,
			// (Number::Float(f1), Number::Float(f2)) => f1 == f2,
			(Number::Quotient(n1, d1), Number::Quotient(n2, d2)) => n1 * d2 == n2 * d1,
			(Number::Quotient(n, d), Number::Int(i)) | (Number::Int(i), Number::Quotient(n, d)) => *n == i * d,
			(Number::BigQuotient(q1), Number::BigQuotient(q2)) => q1 == q2,
			(Number::BigQuotient(q), Number::Quotient(n, d)) | (Number::Quotient(n, d), Number::BigQuotient(q)) => {
				**q == Rational::new(BigInt::from(*n), BigInt::from(*d))
			}
			(Number::BigQuotient(q), Number::Int(i)) | (Number::Int(i), Number::BigQuotient(q)) => {
				q.is_integer() && q.numerator == BigInt::from(*i)
			}
			(Number::BigQuotient(q), Number::BigInt(b)) | (Number::BigInt(b), Number::BigQuotient(q)) => {
				q.is_integer() && &q.numerator == *b
			}
			(quotient @ Number::Quotient(..), Number::Float(f)) | (Number::Float(f), quotient @ Number::Quotient(..)) => {
				f64::from(*quotient) as f32 == *f as f32
			}
			(Number::BigQuotient(q), Number::Float(f)) | (Number::Float(f), Number::BigQuotient(q)) => {
				q.to_f64() as f32 == *f as f32
			}
			(Number::Complex(r1, i1), Number::Complex(r2, i2)) => r1 == r2 && i1 == i2,
			(Number::Real(a), Number::Real(b)) => a == b,
			(Number::Real(r), Number::Float(f)) | (Number::Float(f), Number::Real(r)) => r.to_f64() as f32 == *f as f32,
			// Special values: semantic equality (not IEEE 754)
			(Number::Nan, Number::Nan) => true,
			(Number::Inf, Number::Inf) => true,
			(Number::NegInf, Number::NegInf) => true,
			// Cross-type special value equality: Float(NaN) == Nan, etc.
			(Number::Float(f), Number::Nan) | (Number::Nan, Number::Float(f)) => f.is_nan(),
			(Number::Float(f), Number::Inf) | (Number::Inf, Number::Float(f)) => {
				f.is_infinite() && f.is_sign_positive()
			}
			(Number::Float(f), Number::NegInf) | (Number::NegInf, Number::Float(f)) => {
				f.is_infinite() && f.is_sign_negative()
			}
			_ => false,
		}
	}
}

impl PartialEq<i32> for Number {
	fn eq(&self, other: &i32) -> bool {
		*self == *other as i64
	}
}

impl PartialEq<i64> for Number {
	fn eq(&self, other: &i64) -> bool {
		match self {
			Number::Int(i) => *i == *other,
			Number::Float(f) => *f == *other as f64,
			Number::Quotient(n, d) => *n / d == *other,
			Number::BigQuotient(q) => q.is_integer() && q.numerator == BigInt::from(*other),
			Number::Complex(r, i) => *r == *other as f64 && *i == 0.0,
			_ => false,
		}
	}
}


impl PartialEq<f32> for Number {
	fn eq(&self, other: &f32) -> bool {
		match self {
			Number::Int(i) => *i as f32 == *other,
			Number::BigInt(b) => b.to_f32() == Some(*other),
			Number::Float(f) => *f as f32 == *other,
			Number::Quotient(n, d) => *n as f32 / *d as f32 == *other,
			Number::BigQuotient(q) => q.to_f64() as f32 == *other,
			Number::Complex(r, i) => *r as f32 == *other && *i == 0.0,
			Number::Real(r) => r.to_f64() as f32 == *other,
			Number::Nan => other.is_nan(),
			Number::Inf => *other == f32::INFINITY,
			Number::NegInf => *other == f32::NEG_INFINITY,
			// _ => false,
		}
	}
}

