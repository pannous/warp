use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::ops::{Add, Div, Mul, Sub};
use num_bigint::BigInt;
use num_traits::{ToPrimitive, Zero};

fn deserialize_leaked_bigint<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<&'static BigInt, D::Error> {
	BigInt::deserialize(deserializer).map(|big| &*Box::leak(Box::new(big)))
}

// pub mod Numbers{
pub fn tee() {
	println!("tee");
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
			Number::Complex(r, i) => *r == 0.0 && *i == 0.0,
			Number::Float(f) => *f == 0.0,
			Number::Nan | Number::Inf | Number::NegInf => false,
		}
	}
	pub fn abs(&self) -> f64 {
		match self {
			Number::Int(i) => i.abs() as f64,
			Number::BigInt(b) => b.to_f64().unwrap_or(f64::INFINITY).abs(),
			Number::Quotient(n, d) => (*n as f64 / *d as f64).abs(),
			Number::Complex(r, i) => (r * r + i * i).sqrt(),
			Number::Float(f) => f.abs(),
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

	pub fn is_integer(&self) -> bool {
		matches!(self, Number::Int(_) | Number::BigInt(_))
	}

	pub fn to_bigint(&self) -> BigInt {
		match self {
			Number::Int(i) => BigInt::from(*i),
			Number::BigInt(b) => (*b).clone(),
			other => panic!("not an integer: {}", other),
		}
	}

	/// Integer literal of any size: 123456789012345678901234567890
	pub fn parse_integer(digits: &str) -> Option<Number> {
		digits.parse::<BigInt>().ok().map(Number::from_bigint)
	}

	pub(crate) fn is_number(token: &str) -> bool {
		token.parse::<f64>().is_ok()
	}

	pub(crate) fn parse(token: &str) -> Self {
		let parsed = token.parse().expect("Expected a valid number");
		Number::Float(parsed)
	}
}

impl Display for Number {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Number::Int(i) => write!(f, "{}", i),
			Number::BigInt(b) => write!(f, "{}", b),
			Number::Float(fl) => write!(f, "{}", fl),
			Number::Quotient(numer, denom) => write!(f, "{}/{}", numer, denom),
			Number::Complex(real, imag) => write!(f, "{} + {}i", real, imag),
			Number::Nan => write!(f, "NaN"),
			Number::Inf => write!(f, "∞"),
			Number::NegInf => write!(f, "-∞"),
		}
	}
}

impl Add for Number {
	type Output = Self;

	fn add(self, other: Self) -> Self::Output {
		match (self, other) {
			(Number::Quotient(n1, d1), Number::Quotient(n2, d2)) => {
				// a/b + c/d = (ad + bc) / bd
				Number::Quotient(n1 * d2 + n2 * d1, d1 * d2)
			}
			(Number::Int(n1), Number::Int(n2)) => n1.checked_add(n2).map(Number::Int).unwrap_or_else(|| Number::from_bigint(BigInt::from(n1) + BigInt::from(n2))),
			(a, b) if a.is_integer() && b.is_integer() => Number::from_bigint(a.to_bigint() + b.to_bigint()),
			(Number::Float(n1), Number::Float(n2)) => Number::Float(n1 + n2),
			(Number::Complex(r1, i1), Number::Complex(r2, i2)) => Number::Complex(r1 + r2, i1 + i2),
			// Mixed type conversions - convert to Float
			(Number::Int(n1), Number::Float(n2)) => Number::Float(n1 as f64 + n2),
			(Number::Float(n1), Number::Int(n2)) => Number::Float(n1 + n2 as f64),
			_ => panic!("unsupported types"),
		}
	}
}

impl Sub for Number {
	type Output = Self;

	fn sub(self, other: Self) -> Self::Output {
		match (self, other) {
			(Number::Quotient(n1, d1), Number::Quotient(n2, d2)) => {
				// a/b - c/d = (ad - bc) / bd
				Number::Quotient(n1 * d2 - n2 * d1, d1 * d2)
			}
			(Number::Int(n1), Number::Int(n2)) => n1.checked_sub(n2).map(Number::Int).unwrap_or_else(|| Number::from_bigint(BigInt::from(n1) - BigInt::from(n2))),
			(a, b) if a.is_integer() && b.is_integer() => Number::from_bigint(a.to_bigint() - b.to_bigint()),
			(Number::Float(n1), Number::Float(n2)) => Number::Float(n1 - n2),
			(Number::Complex(r1, i1), Number::Complex(r2, i2)) => Number::Complex(r1 - r2, i1 - i2),
			// Mixed type conversions - convert to Float
			(Number::Int(n1), Number::Float(n2)) => Number::Float(n1 as f64 - n2),
			(Number::Float(n1), Number::Int(n2)) => Number::Float(n1 - n2 as f64),
			_ => panic!("unsupported types"),
		}
	}
}

impl Mul for Number {
	type Output = Self;

	fn mul(self, other: Self) -> Self::Output {
		match (self, other) {
			(Number::Quotient(n1, d1), Number::Quotient(n2, d2)) => {
				// a/b * c/d = ac / bd
				Number::Quotient(n1 * n2, d1 * d2)
			}
			(Number::Int(n1), Number::Int(n2)) => n1.checked_mul(n2).map(Number::Int).unwrap_or_else(|| Number::from_bigint(BigInt::from(n1) * BigInt::from(n2))),
			(a, b) if a.is_integer() && b.is_integer() => Number::from_bigint(a.to_bigint() * b.to_bigint()),
			(Number::Float(n1), Number::Float(n2)) => Number::Float(n1 * n2),
			(Number::Int(n1), Number::Float(n2)) => Number::Float(n1 as f64 * n2),
			(Number::Float(n1), Number::Int(n2)) => Number::Float(n1 * n2 as f64),
			(Number::Complex(r1, i1), Number::Complex(r2, i2)) => {
				// (a + bi)(c + di) = (ac - bd) + (ad + bc)i
				Number::Complex(r1 * r2 - i1 * i2, r1 * i2 + i1 * r2)
			}
			_ => panic!("unsupported types"),
		}
	}
}

impl Div for Number {
	type Output = Self;

	fn div(self, other: Self) -> Self::Output {
		match (self, other) {
			(Number::Quotient(n1, d1), Number::Quotient(n2, d2)) => {
				Number::Quotient(n1 * d2, d1 * n2)
			}
			(Number::Quotient(q1, q2), Number::Int(n2)) => Number::Quotient(q1, q2 * n2),
			(Number::Quotient(q1, q2), Number::Float(n2)) => {
				Number::Float(q1 as f64 / q2 as f64 * n2)
			}
			(Number::Int(n1), Number::Quotient(q1, q2)) => Number::Quotient(n1 * q2, q1),
			(Number::Float(n1), Number::Quotient(q1, q2)) => {
				Number::Float(n1 / q1 as f64 / q2 as f64)
			}
			(Number::Int(n1), Number::Int(n2)) => Number::Quotient(n1, n2),
			(Number::Float(n1), Number::Float(n2)) => Number::Float(n1 / n2),
			(Number::Int(n1), Number::Float(n2)) => Number::Float(n1 as f64 / n2),
			(Number::Float(n1), Number::Int(n2)) => Number::Float(n1 / n2 as f64),
			(Number::Complex(r1, i1), Number::Complex(r2, i2)) => {
				// (a + bi) / (c + di) = (a + bi)(c - di) / (c^2 + d^2)
				Number::Complex(
					(r1 * r2 + i1 * i2) / (r2 * r2 + i2 * i2),
					(i1 * r2 - r1 * i2) / (r2 * r2 + i2 * i2),
				)
			}
			_ => panic!("unsupported types"),
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
			Number::Complex(_, _) => unimplemented!(),
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
			(Number::Complex(r1, i1), Number::Complex(r2, i2)) => r1 == r2 && i1 == i2,
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
		match self {
			Number::Int(i) => *i == *other as i64,
			Number::Float(f) => *f == *other as f64,
			Number::Quotient(n, d) => *n / d == *other as i64,
			Number::Complex(r, i) => *r == *other as f64 && *i == 0.0,
			_ => false,
		}
	}
}

impl PartialEq<i64> for Number {
	fn eq(&self, other: &i64) -> bool {
		match self {
			Number::Int(i) => *i == *other,
			Number::Float(f) => *f == *other as f64,
			Number::Quotient(n, d) => *n / d == *other,
			Number::Complex(r, i) => *r == *other as f64 && *i == 0.0,
			_ => false,
		}
	}
}

// high precision
// impl PartialEq<f64> for Number {
//     fn eq(&self, other: &f64) -> bool {
//         match self {
//             Number::Int(i) => *i as f64 == *other,
//             Number::Float(f) => *f == *other,
//             Number::Quotient(n, d) => *n as f64 / *d as f64 == *other,
//             Number::Complex(r, i) => *r == *other && *i == 0.0,
//             // _ => false,
//         }
//     }
// }

impl PartialEq<f32> for Number {
	fn eq(&self, other: &f32) -> bool {
		match self {
			Number::Int(i) => *i as f32 == *other,
			Number::BigInt(b) => b.to_f32() == Some(*other),
			Number::Float(f) => *f as f32 == *other,
			Number::Quotient(n, d) => *n as f32 / *d as f32 == *other,
			Number::Complex(r, i) => *r as f32 == *other && *i == 0.0,
			Number::Nan => other.is_nan(),
			Number::Inf => *other == f32::INFINITY,
			Number::NegInf => *other == f32::NEG_INFINITY,
			// _ => false,
		}
	}
}

// impl PartialEq for Number {
//     fn eq(&self, other: &Self) -> bool {
//         match (self, other) {
//             (Number::Int(a), Number::Int(b)) => a == b,
//             (Number::Float(a), Number::Float(b)) => (a - b).abs() < f64::EPSILON,
//             (Number::Int(a), Number::Float(b)) => (*a as f64 - *b).abs() < f64::EPSILON,
//             (Number::Float(a), Number::Int(b)) => (*a - *b as f64).abs() < f64::EPSILON,
//         }
//     }
// }
