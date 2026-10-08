//! Exact real numbers beyond Q (Footguns.md "Exact real numbers").
//!
//! One mechanism, not a zoo of special types: an exact number is a sparse polynomial with rational
//! coefficients over named generators (π, ℯ, ⅈ, square and cube roots), kept in a normal form by
//! simplification rules. This mirrors the Lean model `HyperGeneral` (a list of coefficient × exponent
//! pairs, simplified to a normal form). The hyperreal generator ε takes any integer exponent (ω = ε⁻¹):
//! an exact number is then a Laurent polynomial in ε (notes/hyperreals.md).
//!
//! Normal form: no zero coefficients; π and ℯ carry any nonzero integer exponent; ⅈ appears at most
//! once (ⅈ² = -1); a monomial has at most one square root √r with r > 1 square-free and at most one cube
//! root ∛r with r > 1 cube-free (√a·√b = √(ab) reduced). The set of such monomials is linearly
//! independent over Q given that π and ℯ are algebraically independent (Schanuel's conjecture, assumed)
//! and Besicovitch's theorem for radicals, so two exact numbers are equal iff their normal forms are.
//!
//! Anything without an exact rule (sin(1), exp(√3), 1/(1+√2)) is an `Approx` f64 and prints as `≈…`.
//! Order comparisons of exact numbers use interval arithmetic with increasing precision; when that
//! cannot decide within the precision budget the comparison is an error, never a guess.

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;

/// Rational in lowest terms, denominator > 0
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rational {
	pub numerator: BigInt,
	pub denominator: BigInt,
}

impl Rational {
	/// n/d in lowest terms, d nonzero
	pub fn new(numerator: BigInt, denominator: BigInt) -> Rational {
		assert!(!denominator.is_zero(), "rational with zero denominator");
		let divisor = numerator.gcd(&denominator) * denominator.signum();
		Rational { numerator: numerator / &divisor, denominator: denominator / &divisor }
	}

	/// The decimal a float was written as, exact: 0.3 is 3/10 (its shortest text, which reads back as the same float)
	pub fn of_decimal(value: f64) -> Option<Rational> {
		let text = format!("{value}");
		let (whole, fraction) = text.split_once('.').unwrap_or((&text, ""));
		let numerator: BigInt = format!("{whole}{fraction}").parse().ok()?;
		Some(Rational::new(numerator, BigInt::from(10).pow(fraction.len() as u32)))
	}

	/// The exact decimal text when the denominator has no prime factor but 2 and 5: 981/100 is "9.81", 1/3 has none
	pub fn decimal_text(&self) -> Option<String> {
		let factors = [BigInt::from(2), BigInt::from(5)];
		let mut rest = self.denominator.clone();
		// one place per factor 2 or 5 taken out: enough places, the surplus zeros are trimmed below
		let mut digits = 0u32;
		while !rest.is_one() {
			let factor = factors.iter().find(|factor| (&rest % *factor).is_zero())?;
			rest /= factor;
			digits += 1;
		}
		let scaled = (&self.numerator * BigInt::from(10).pow(digits) / &self.denominator).abs().to_string();
		let padded = format!("{scaled:0>width$}", width = digits as usize + 1);
		let (whole, fraction) = padded.split_at(padded.len() - digits as usize);
		let fraction = fraction.trim_end_matches('0');
		let sign = if self.is_negative() { "-" } else { "" };
		Some(if fraction.is_empty() { format!("{sign}{whole}") } else { format!("{sign}{whole}.{fraction}") })
	}

	pub fn integer(value: impl Into<BigInt>) -> Rational {
		Rational { numerator: value.into(), denominator: BigInt::one() }
	}

	pub fn zero() -> Rational {
		Rational::integer(0)
	}

	pub fn is_zero(&self) -> bool {
		self.numerator.is_zero()
	}

	pub fn is_integer(&self) -> bool {
		self.denominator.is_one()
	}

	pub fn is_negative(&self) -> bool {
		self.numerator.is_negative()
	}

	pub fn add(&self, other: &Rational) -> Rational {
		Rational::new(&self.numerator * &other.denominator + &other.numerator * &self.denominator, &self.denominator * &other.denominator)
	}

	pub fn neg(&self) -> Rational {
		Rational { numerator: -&self.numerator, denominator: self.denominator.clone() }
	}

	pub fn mul(&self, other: &Rational) -> Rational {
		Rational::new(&self.numerator * &other.numerator, &self.denominator * &other.denominator)
	}

	/// None for division by zero
	pub fn inverse(&self) -> Option<Rational> {
		(!self.is_zero()).then(|| Rational::new(self.denominator.clone(), self.numerator.clone()))
	}

	pub fn to_f64(&self) -> f64 {
		let (n, d) = (self.numerator.to_f64().unwrap_or(f64::NAN), self.denominator.to_f64().unwrap_or(f64::NAN));
		if n.is_finite() && d.is_finite() {
			n / d
		} else {
			// huge parts: scale both down before dividing
			let shift = self.numerator.bits().max(self.denominator.bits()).saturating_sub(1000) as usize;
			(&self.numerator >> shift).to_f64().unwrap_or(f64::NAN) / (&self.denominator >> shift).to_f64().unwrap_or(f64::NAN)
		}
	}

	pub fn compare(&self, other: &Rational) -> Ordering {
		(&self.numerator * &other.denominator).cmp(&(&other.numerator * &self.denominator))
	}
}

impl fmt::Display for Rational {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if self.is_integer() {
			write!(f, "{}", self.numerator)
		} else {
			write!(f, "{}/{}", self.numerator, self.denominator)
		}
	}
}

/// A named generator of the polynomial ring. The derived order is the print order: `π√2`, `ℯ∛3`.
/// `Epsilon` is the hyperreal infinitesimal with any integer exponent (ω = ε⁻¹); comparisons decide by the lowest
/// ε power first and only then by the real coefficients.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Generator {
	Pi,
	Euler,
	Imaginary,
	/// √r, r > 1 square-free
	SquareRoot(BigInt),
	/// ∛r, r > 1 cube-free
	CubeRoot(BigInt),
	/// ε, the canonical positive infinitesimal: 0 < ε < r for every positive real r
	Epsilon,
}

impl Generator {
	fn approximate(&self) -> f64 {
		match self {
			Generator::Pi => std::f64::consts::PI,
			Generator::Euler => std::f64::consts::E,
			Generator::Imaginary => f64::NAN,
			Generator::SquareRoot(r) => r.to_f64().unwrap_or(f64::INFINITY).sqrt(),
			Generator::CubeRoot(r) => r.to_f64().unwrap_or(f64::INFINITY).cbrt(),
			Generator::Epsilon => f64::NAN, // no f64 holds an infinitesimal: callers refuse hyperreals first
		}
	}
}

impl fmt::Display for Generator {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Generator::Pi => write!(f, "π"),
			Generator::Euler => write!(f, "ℯ"),
			Generator::Imaginary => write!(f, "ⅈ"),
			Generator::SquareRoot(r) => write!(f, "√{r}"),
			Generator::CubeRoot(r) => write!(f, "∛{r}"),
			Generator::Epsilon => write!(f, "ε"),
		}
	}
}

/// Product of generator powers, sorted by generator, no zero exponents
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Monomial(pub Vec<(Generator, i64)>);

impl Monomial {
	fn one() -> Monomial {
		Monomial(vec![])
	}

	fn of(generator: Generator) -> Monomial {
		Monomial(vec![(generator, 1)])
	}

	fn exponent(&self, generator: &Generator) -> i64 {
		self.0.iter().find(|(g, _)| g == generator).map_or(0, |(_, e)| *e)
	}

	fn has_imaginary(&self) -> bool {
		self.exponent(&Generator::Imaginary) != 0
	}

	fn without(&self, generator: &Generator) -> Monomial {
		Monomial(self.0.iter().filter(|(g, _)| g != generator).cloned().collect())
	}

	/// Product as coefficient × monomial in normal form
	fn times(&self, other: &Monomial) -> (Rational, Monomial) {
		let mut coefficient = Rational::integer(1);
		let mut powers: BTreeMap<Generator, i64> = BTreeMap::new();
		let (mut square, mut cube) = (BigInt::one(), BigInt::one());
		for (generator, exponent) in self.0.iter().chain(other.0.iter()) {
			match generator {
				Generator::SquareRoot(r) => square *= r,
				Generator::CubeRoot(r) => cube *= r,
				_ => *powers.entry(generator.clone()).or_insert(0) += exponent,
			}
		}
		if let Some(imaginary) = powers.get_mut(&Generator::Imaginary) {
			if imaginary.rem_euclid(4) >= 2 {
				coefficient = coefficient.neg();
			}
			*imaginary = imaginary.rem_euclid(2);
		}
		let (outside, inside) = extract_power(&square, 2);
		coefficient = coefficient.mul(&Rational::integer(outside));
		if !inside.is_one() {
			powers.insert(Generator::SquareRoot(inside), 1);
		}
		let (outside, inside) = extract_power(&cube, 3);
		coefficient = coefficient.mul(&Rational::integer(outside));
		if !inside.is_one() {
			powers.insert(Generator::CubeRoot(inside), 1);
		}
		(coefficient, Monomial(powers.into_iter().filter(|(_, e)| *e != 0).collect()))
	}

	/// 1/m as coefficient × monomial: 1/√r = √r/r, 1/∛r = ∛r²/r, 1/ⅈ = -ⅈ
	fn inverse(&self) -> (Rational, Monomial) {
		let mut coefficient = Rational::integer(1);
		let mut result = Monomial::one();
		for (generator, exponent) in &self.0 {
			let factor = match generator {
				Generator::Pi | Generator::Euler | Generator::Epsilon => Monomial(vec![(generator.clone(), -exponent)]),
				Generator::Imaginary => {
					coefficient = coefficient.neg();
					Monomial::of(Generator::Imaginary)
				}
				Generator::SquareRoot(r) => {
					coefficient = coefficient.mul(&Rational::new(BigInt::one(), r.clone()));
					Monomial::of(generator.clone())
				}
				Generator::CubeRoot(r) => {
					coefficient = coefficient.mul(&Rational::new(BigInt::one(), r.clone()));
					let (square_coefficient, square) = Monomial::of(generator.clone()).times(&Monomial::of(generator.clone()));
					coefficient = coefficient.mul(&square_coefficient);
					square
				}
			};
			let (c, m) = result.times(&factor);
			coefficient = coefficient.mul(&c);
			result = m;
		}
		(coefficient, result)
	}

	fn to_f64(&self) -> f64 {
		self.0.iter().map(|(g, e)| g.approximate().powi(*e as i32)).product()
	}
}

impl fmt::Display for Monomial {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		for (generator, exponent) in &self.0 {
			let (symbol, exponent) = match generator {
				Generator::Epsilon if *exponent < 0 => (OMEGA.to_string(), -exponent),
				_ => (generator.to_string(), *exponent),
			};
			write!(f, "{symbol}")?;
			if exponent != 1 {
				write!(f, "{}", superscript(exponent))?;
			}
		}
		Ok(())
	}
}

fn superscript(n: i64) -> String {
	n.to_string()
		.chars()
		.map(|c| match c {
			'-' => '⁻',
			digit => ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'][digit.to_digit(10).unwrap_or(0) as usize],
		})
		.collect()
}

/// How ε⁻ⁿ prints: ω, ω²
const OMEGA: &str = "ω";

/// Trial division bound for reducing radicands. After removing primes below it, a cofactor below
/// BOUND^(k+1) has at most k prime factors, so a perfect-power test finishes the reduction exactly.
const TRIAL_DIVISION_BOUND: u64 = 10_000;

/// n = outside^k · inside with inside k-th-power-free (n > 0)
fn extract_power(n: &BigInt, k: u32) -> (BigInt, BigInt) {
	let (mut outside, mut inside, mut rest) = (BigInt::one(), BigInt::one(), n.clone());
	let mut p = 2u64;
	while p <= TRIAL_DIVISION_BOUND && BigInt::from(p).pow(2) <= rest {
		let prime = BigInt::from(p);
		let mut count = 0;
		while (&rest % &prime).is_zero() {
			rest /= &prime;
			count += 1;
		}
		outside *= prime.pow(count / k);
		inside *= prime.pow(count % k);
		p += if p == 2 { 1 } else { 2 };
	}
	let root = rest.nth_root(k);
	if root.pow(k) == rest && !rest.is_one() {
		outside *= root;
	} else {
		inside *= rest;
	}
	(outside, inside)
}

/// Exact number: sum of coefficient × monomial in normal form
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Exact(pub BTreeMap<Monomial, Rational>);

impl Exact {
	pub fn rational(value: Rational) -> Exact {
		let mut terms = BTreeMap::new();
		if !value.is_zero() {
			terms.insert(Monomial::one(), value);
		}
		Exact(terms)
	}

	pub fn integer(value: i64) -> Exact {
		Exact::rational(Rational::integer(value))
	}

	pub fn generator(generator: Generator) -> Exact {
		Exact(BTreeMap::from([(Monomial::of(generator), Rational::integer(1))]))
	}

	pub fn pi() -> Exact {
		Exact::generator(Generator::Pi)
	}

	pub fn euler() -> Exact {
		Exact::generator(Generator::Euler)
	}

	pub fn imaginary() -> Exact {
		Exact::generator(Generator::Imaginary)
	}

	fn term(coefficient: Rational, monomial: Monomial) -> Exact {
		let mut terms = BTreeMap::new();
		if !coefficient.is_zero() {
			terms.insert(monomial, coefficient);
		}
		Exact(terms)
	}

	pub fn is_zero(&self) -> bool {
		self.0.is_empty()
	}

	/// The value when it is rational
	pub fn as_rational(&self) -> Option<Rational> {
		match self.0.iter().next() {
			None => Some(Rational::zero()),
			Some((monomial, coefficient)) if self.0.len() == 1 && monomial.0.is_empty() => Some(coefficient.clone()),
			_ => None,
		}
	}

	/// The only term, when there is exactly one
	fn single_term(&self) -> Option<(&Monomial, &Rational)> {
		(self.0.len() == 1).then(|| self.0.iter().next()).flatten()
	}

	/// q when the value is exactly q·π
	pub fn pi_multiple(&self) -> Option<Rational> {
		if self.is_zero() {
			return Some(Rational::zero());
		}
		let (monomial, coefficient) = self.single_term()?;
		(*monomial == Monomial::of(Generator::Pi)).then(|| coefficient.clone())
	}

	pub fn has_imaginary(&self) -> bool {
		self.0.keys().any(Monomial::has_imaginary)
	}

	pub fn epsilon() -> Exact {
		Exact::generator(Generator::Epsilon)
	}

	pub fn omega() -> Exact {
		Exact::term(Rational::integer(1), Monomial(vec![(Generator::Epsilon, -1)]))
	}

	/// A hyperreal that is no real: some term has a power of ε
	pub fn has_epsilon(&self) -> bool {
		self.0.keys().any(|monomial| monomial.exponent(&Generator::Epsilon) != 0)
	}

	/// The terms with ε^power, ε removed from them
	fn epsilon_part(&self, power: i64) -> Exact {
		let terms = self.0.iter().filter(|(monomial, _)| monomial.exponent(&Generator::Epsilon) == power);
		Exact(terms.map(|(monomial, coefficient)| (monomial.without(&Generator::Epsilon), coefficient.clone())).collect())
	}

	/// The lowest power of ε among the terms: negative for an infinite number, positive for an infinitesimal
	fn lowest_epsilon_power(&self) -> Option<i64> {
		self.0.keys().map(|monomial| monomial.exponent(&Generator::Epsilon)).min()
	}

	/// The standard part st(x): the real nearest to a finite hyperreal; an error for an infinite one
	pub fn standard_part(&self) -> Result<Exact, String> {
		match self.lowest_epsilon_power() {
			Some(power) if power < 0 => Err(format!("st({self}): {self} is infinite, it has no standard part")),
			_ => Ok(self.epsilon_part(0)),
		}
	}

	pub fn add(&self, other: &Exact) -> Exact {
		let mut terms = self.0.clone();
		for (monomial, coefficient) in &other.0 {
			let sum = terms.get(monomial).map_or_else(|| coefficient.clone(), |c| c.add(coefficient));
			if sum.is_zero() {
				terms.remove(monomial);
			} else {
				terms.insert(monomial.clone(), sum);
			}
		}
		Exact(terms)
	}

	pub fn neg(&self) -> Exact {
		Exact(self.0.iter().map(|(m, c)| (m.clone(), c.neg())).collect())
	}

	pub fn sub(&self, other: &Exact) -> Exact {
		self.add(&other.neg())
	}

	pub fn mul(&self, other: &Exact) -> Exact {
		let mut product = Exact::integer(0);
		for (m1, c1) in &self.0 {
			for (m2, c2) in &other.0 {
				let (c, m) = m1.times(m2);
				product = product.add(&Exact::term(c.mul(c1).mul(c2), m));
			}
		}
		product
	}

	pub fn scale(&self, factor: &Rational) -> Exact {
		self.mul(&Exact::rational(factor.clone()))
	}

	/// Exact inverse of a single term; None for zero or a sum (no general rationalization yet)
	pub fn inverse(&self) -> Option<Exact> {
		let (monomial, coefficient) = self.single_term()?;
		let (c, m) = monomial.inverse();
		Some(Exact::term(c.mul(&coefficient.inverse()?), m))
	}

	pub fn pow(&self, exponent: i64) -> Option<Exact> {
		let base = if exponent < 0 { self.inverse()? } else { self.clone() };
		let (mut result, mut square, mut n) = (Exact::integer(1), base, exponent.unsigned_abs());
		while n > 0 {
			if n & 1 == 1 {
				result = result.mul(&square);
			}
			n >>= 1;
			if n > 0 {
				square = square.mul(&square);
			}
		}
		Some(result)
	}

	/// Exact root of index 2 or 3 when it exists in the normal form: rationals (√8 = 2√2, √-4 = 2ⅈ,
	/// ∛-27 = -3) and single terms whose π, ℯ exponents divide by the index (√π² = π); else None
	pub fn root(&self, index: u32) -> Option<Exact> {
		if self.is_zero() {
			return Some(Exact::integer(0));
		}
		let (monomial, coefficient) = self.single_term()?;
		let mut result = Exact::integer(1);
		for (generator, exponent) in &monomial.0 {
			match generator {
				Generator::Pi | Generator::Euler | Generator::Epsilon if exponent % index as i64 == 0 => {
					result = result.mul(&Exact::term(Rational::integer(1), Monomial(vec![(generator.clone(), exponent / index as i64)])));
				}
				_ => return None,
			}
		}
		// √(n/d) = √(n·d)/d, ∛(n/d) = ∛(n·d²)/d
		let negative = coefficient.is_negative();
		let radicand = coefficient.numerator.abs() * coefficient.denominator.pow(index - 1);
		let (outside, inside) = extract_power(&radicand, index);
		let mut root = Exact::rational(Rational::new(outside, coefficient.denominator.clone()));
		if !inside.is_one() {
			let generator = if index == 2 { Generator::SquareRoot(inside) } else { Generator::CubeRoot(inside) };
			root = root.mul(&Exact::generator(generator));
		}
		if negative {
			root = if index == 2 { root.mul(&Exact::imaginary()) } else { root.neg() };
		}
		Some(root.mul(&result))
	}

	/// f64 approximation, NaN for a value with an imaginary part
	pub fn to_f64(&self) -> f64 {
		if self.has_imaginary() {
			return f64::NAN;
		}
		self.0.iter().map(|(m, c)| c.to_f64() * m.to_f64()).sum()
	}

	/// Sign by interval arithmetic with increasing precision; an error for complex values or when the
	/// precision budget cannot separate the value from zero
	pub fn sign(&self) -> Result<Ordering, String> {
		if self.is_zero() {
			return Ok(Ordering::Equal);
		}
		if self.has_imaginary() {
			return Err(format!("{self} is complex, complex numbers are not ordered"));
		}
		if self.has_epsilon() {
			// the terms of the lowest ε power outweigh all others; ε itself is positive
			let lowest = self.lowest_epsilon_power().unwrap_or(0);
			return self.epsilon_part(lowest).sign();
		}
		let mut precision = 64;
		while precision <= PRECISION_BUDGET {
			let interval = self.interval(precision);
			if interval.lo.is_positive() {
				return Ok(Ordering::Greater);
			}
			if interval.hi.is_negative() {
				return Ok(Ordering::Less);
			}
			precision *= 2;
		}
		Err(format!("undecidable: the sign of {self} is not known within {PRECISION_BUDGET} bits"))
	}

	fn interval(&self, precision: usize) -> Interval {
		let mut sum = Interval { lo: BigInt::zero(), hi: BigInt::zero() };
		for (monomial, coefficient) in &self.0 {
			let term = monomial.interval(precision).scale(coefficient);
			sum = Interval { lo: sum.lo + term.lo, hi: sum.hi + term.hi };
		}
		sum
	}
}

impl fmt::Display for Exact {
	/// `2√2`, `π/2`, `3+√2`, `-ⅈ`
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if self.is_zero() {
			return write!(f, "0");
		}
		for (index, (monomial, coefficient)) in self.0.iter().enumerate() {
			let magnitude = coefficient.numerator.abs();
			if coefficient.is_negative() {
				write!(f, "-")?;
			} else if index > 0 {
				write!(f, "+")?;
			}
			if monomial.0.is_empty() || !magnitude.is_one() {
				write!(f, "{magnitude}")?;
			}
			write!(f, "{monomial}")?;
			if !coefficient.is_integer() {
				write!(f, "/{}", coefficient.denominator)?;
			}
		}
		Ok(())
	}
}

/// Largest fixed-point precision (bits) tried before a comparison gives up
pub const PRECISION_BUDGET: usize = 4096;

/// [lo, hi] · 2^-precision
#[derive(Clone, Debug)]
struct Interval {
	lo: BigInt,
	hi: BigInt,
}

impl Interval {
	fn exact(value: BigInt) -> Interval {
		Interval { lo: value.clone(), hi: value }
	}

	/// Product of two positive intervals
	fn times(&self, other: &Interval, precision: usize) -> Interval {
		Interval { lo: (&self.lo * &other.lo) >> precision, hi: ((&self.hi * &other.hi) >> precision) + 1 }
	}

	/// 1/x of a positive interval; its lower end may round to 0, then the result is just wide
	fn inverse(&self, precision: usize) -> Interval {
		let one = BigInt::one() << (2 * precision);
		let lo = if self.hi.is_positive() { &one / &self.hi } else { BigInt::zero() };
		let hi = if self.lo.is_positive() { &one / &self.lo + 1 } else { one.clone() << precision };
		Interval { lo, hi }
	}

	fn scale(&self, q: &Rational) -> Interval {
		let (a, b) = (&self.lo * &q.numerator, &self.hi * &q.numerator);
		let (low, high) = if q.is_negative() { (b, a) } else { (a, b) };
		Interval { lo: low.div_floor(&q.denominator), hi: -((-high).div_floor(&q.denominator)) }
	}
}

impl Monomial {
	fn interval(&self, precision: usize) -> Interval {
		let mut result = Interval::exact(BigInt::one() << precision);
		for (generator, exponent) in &self.0 {
			let mut factor = generator_interval(generator, precision);
			if *exponent < 0 {
				factor = factor.inverse(precision);
			}
			for _ in 0..exponent.unsigned_abs() {
				result = result.times(&factor, precision);
			}
		}
		result
	}
}

/// Guard bits for the series of π and ℯ: their rounding errors stay far below 2^GUARD units
const GUARD: usize = 64;

fn generator_interval(generator: &Generator, precision: usize) -> Interval {
	let widen = |value: BigInt| {
		let error = BigInt::one() << (GUARD - 16);
		Interval { lo: (&value - &error) >> GUARD, hi: ((value + error) >> GUARD) + 1 }
	};
	match generator {
		// Machin: π = 16·atan(1/5) - 4·atan(1/239)
		Generator::Pi => widen(arctan_inverse(5, precision + GUARD) * 16 - arctan_inverse(239, precision + GUARD) * 4),
		Generator::Euler => widen(euler(precision + GUARD)),
		Generator::SquareRoot(r) => {
			let root = (r << (2 * precision)).sqrt();
			Interval { lo: root.clone(), hi: root + 1 }
		}
		Generator::CubeRoot(r) => {
			let root = (r << (3 * precision)).cbrt();
			Interval { lo: root.clone(), hi: root + 1 }
		}
		Generator::Imaginary => unreachable!("complex values are rejected before interval evaluation"),
		Generator::Epsilon => unreachable!("hyperreals are signed by their ε powers before interval evaluation"),
	}
}

/// atan(1/x) · 2^bits, truncated
fn arctan_inverse(x: u32, bits: usize) -> BigInt {
	let x_squared = BigInt::from(x) * x;
	let mut power = (BigInt::one() << bits) / x;
	let mut sum = power.clone();
	let mut k = 1u32;
	while !power.is_zero() {
		power /= &x_squared;
		let term = &power / (2 * k + 1);
		if k % 2 == 1 {
			sum -= term;
		} else {
			sum += term;
		}
		k += 1;
	}
	sum
}

/// ℯ · 2^bits = Σ 2^bits/k!, truncated
fn euler(bits: usize) -> BigInt {
	let mut term = BigInt::one() << bits;
	let mut sum = term.clone();
	let mut k = 1u32;
	while !term.is_zero() {
		term /= k;
		sum += &term;
		k += 1;
	}
	sum
}

/// A real number: exact normal form, or an f64 approximation where no exact rule applies
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Real {
	Exact(Exact),
	Approx(f64),
}

impl Real {
	pub fn to_f64(&self) -> f64 {
		match self {
			Real::Exact(exact) => exact.to_f64(),
			Real::Approx(value) => *value,
		}
	}

	pub fn is_zero(&self) -> bool {
		match self {
			Real::Exact(exact) => exact.is_zero(),
			Real::Approx(value) => *value == 0.0,
		}
	}

	pub fn is_exact(&self) -> bool {
		matches!(self, Real::Exact(_))
	}

	pub fn neg(&self) -> Real {
		match self {
			Real::Exact(exact) => Real::Exact(exact.neg()),
			Real::Approx(value) => Real::Approx(-value),
		}
	}

	/// Ordering of self and other; an error when it cannot be decided
	pub fn compare(&self, other: &Real) -> Result<Ordering, String> {
		match (self, other) {
			(Real::Exact(a), Real::Exact(b)) => a.sub(b).sign(),
			_ => {
				let (a, b) = (self.to_f64(), other.to_f64());
				if a.is_nan() || b.is_nan() {
					return Err(format!("{self} and {other} are not ordered"));
				}
				// f64 functions are accurate to a few ulps; closer than that is not decidable
				let tolerance = (a.abs() + b.abs()) * 1e-13 + f64::MIN_POSITIVE;
				if a - b > tolerance {
					Ok(Ordering::Greater)
				} else if b - a > tolerance {
					Ok(Ordering::Less)
				} else {
					Err(format!("undecidable: {self} and {other} agree to f64 precision but one is an approximation"))
				}
			}
		}
	}

	/// Equality: exact normal forms compare structurally, an approximation only when clearly different
	pub fn equals(&self, other: &Real) -> Result<bool, String> {
		match (self, other) {
			(Real::Exact(a), Real::Exact(b)) => Ok(a == b),
			// compare only succeeds for clearly different values
			_ => self.compare(other).map(|_| false),
		}
	}
}

impl fmt::Display for Real {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Real::Exact(exact) => write!(f, "{exact}"),
			Real::Approx(value) => write!(f, "≈{value}"),
		}
	}
}
