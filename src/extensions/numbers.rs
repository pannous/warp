use std::fmt::Display;
use std::ops::{Add, Sub, Mul, Div};

// pub mod Numbers{
pub fn tee() {
    println!("tee");
}

#[derive(Debug, PartialEq)]
pub enum Number {
    Int(i64),
    Float(f64),
    Quotient(i64, i64),
    Complex(f64, f64),
    // other variants as needed
}


impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Number::Int(i) => write!(f, "{}", i),
            Number::Float(fl) => write!(f, "{}", fl),
            Number::Quotient(numer, denom) => write!(f, "{}/{}", numer, denom),
            Number::Complex(real, imag) => write!(f, "{} + {}i", real, imag),
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
            (Number::Int(n1), Number::Int(n2)) => Number::Int(n1 + n2),
            (Number::Float(n1), Number::Float(n2)) => Number::Float(n1 + n2),
            (Number::Complex(r1, i1), Number::Complex(r2, i2)) => Number::Complex(r1 + r2, i1 + i2),
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
            (Number::Int(n1), Number::Int(n2)) => Number::Int(n1 - n2),
            (Number::Float(n1), Number::Float(n2)) => Number::Float(n1 - n2),
            (Number::Complex(r1, i1), Number::Complex(r2, i2)) => Number::Complex(r1 - r2, i1 - i2),
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
            (Number::Int(n1), Number::Int(n2)) => Number::Int(n1 * n2),
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
            (Number::Quotient(n1, d1), Number::Quotient(n2, d2)) => Number::Quotient(n1 * d2, d1 * n2),
            (Number::Quotient(q1, q2), Number::Int(n2)) => Number::Quotient(q1, q2 * n2),
            (Number::Quotient(q1, q2), Number::Float(n2)) => Number::Float(q1 as f64 / q2 as f64 * n2),
            (Number::Int(n1), Number::Quotient(q1, q2)) => Number::Quotient(n1 * q2, q1),
            (Number::Float(n1), Number::Quotient(q1, q2)) => Number::Float(n1 / q1 as f64 / q2 as f64),
            (Number::Int(n1), Number::Int(n2)) => Number::Quotient(n1, n2),
            (Number::Float(n1), Number::Float(n2)) => Number::Float(n1 / n2),
            (Number::Int(n1), Number::Float(n2)) => Number::Float(n1 as f64 / n2),
            (Number::Float(n1), Number::Int(n2)) => Number::Float(n1 / n2 as f64),
            (Number::Complex(r1, i1), Number::Complex(r2, i2)) => {
                // (a + bi) / (c + di) = (a + bi)(c - di) / (c^2 + d^2)
                Number::Complex((r1 * r2 + i1 * i2) / (r2 * r2 + i2 * i2), (i1 * r2 - r1 * i2) / (r2 * r2 + i2 * i2))
            }
            _ => panic!("unsupported types"),
        }
    }
}


impl Into<f64> for Number {
    fn into(self) -> f64 {
        match self {
            Number::Int(i) => i as f64,
            Number::Float(f) => f,
            Number::Quotient(numer, denom) => numer as f64 / denom as f64,
            Number::Complex(_, _) => unimplemented!(),
        }
    }
}

// }