//! The dimensions and units warp knows, read once from lib/units.warp's `unit_definitions` block (card dimensions-units
//! Q2, user 2026-10-15): `dimension length` names a dimension, `m: length` is its base unit, `cm := m/100` a unit that is
//! an amount of another, `°C := K + 273.15` one counting from another zero. A definition of several units or of none
//! (`mph := mi/h`, `° := π/180`) is an alias, a word for its expression. Read by its own small reader, not the parser:
//! the parser asks these definitions whether a word is a unit.
use super::{Dimension, Unit};
use crate::extensions::reals::Rational;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, ToPrimitive};
use std::sync::OnceLock;

const SOURCE: &str = include_str!("../../../lib/units.warp");
const BLOCK_START: &str = "unit_definitions = data {";
const BLOCK_END: &str = "}";
const DIMENSION_WORD: &str = "dimension ";
const DEFINE: &str = ":=";
const BASE: char = ':';
const COMMENT: &str = "//";
/// The dimension each name of lib/units.warp stands for: time's is `duration` (decision anchor-time)
const DIMENSION_WORDS: [(&str, Dimension); 5] = [
	("length", Dimension::Length),
	("mass", Dimension::Mass),
	("duration", Dimension::Time),
	("era", Dimension::Era),
	("temperature", Dimension::Temperature),
];

pub(super) struct Definitions {
	/// The dimensions by name, in written order, and the names of products of them (`speed := length/duration`)
	pub(super) dimensions: Vec<(&'static str, Dimension)>,
	pub(super) derived_dimensions: Vec<(&'static str, Vec<(Dimension, i32)>)>,
	pub(super) units: Vec<Unit>,
	/// The base unit of each dimension, by name: the unit a quantity's amount counts in at run time
	pub(super) base_units: Vec<(Dimension, &'static str)>,
	pub(super) aliases: Vec<(&'static str, &'static str)>,
}

pub(super) fn definitions() -> &'static Definitions {
	static DEFINITIONS: OnceLock<Definitions> = OnceLock::new();
	DEFINITIONS
		.get_or_init(|| read(SOURCE).unwrap_or_else(|problem| panic!("lib/units.warp unit_definitions: {problem}")))
}

/// A unit as it is read: its amount and zero in its dimension's base unit, before they become whole steps
struct Read {
	name: &'static str,
	dimension: Dimension,
	scale: Rational,
	zero: Rational,
}

fn read(source: &'static str) -> Result<Definitions, String> {
	let block = source.split_once(BLOCK_START).ok_or("no block")?.1;
	let mut dimensions = vec![];
	let mut derived_dimensions = vec![];
	let mut base_units = vec![];
	let mut units: Vec<Read> = vec![];
	let mut aliases = vec![];
	for line in block.lines().map(str::trim) {
		if line == BLOCK_END {
			break;
		}
		if line.is_empty() || line.starts_with(COMMENT) {
			continue;
		}
		if let Some((name, expression)) = line.strip_prefix(DIMENSION_WORD).and_then(|rest| rest.split_once(DEFINE)) {
			derived_dimensions.push((name.trim(), powers_of(expression, &dimensions)?));
		} else if let Some(name) = line.strip_prefix(DIMENSION_WORD) {
			let dimension = DIMENSION_WORDS
				.iter()
				.find(|(word, _)| *word == name.trim())
				.ok_or(format!("unknown dimension {name}"))?;
			dimensions.push((name.trim(), dimension.1));
		} else if let Some((name, expression)) = line.split_once(DEFINE) {
			let (name, expression) = (name.trim(), expression.trim());
			match unit_of(expression, &units)? {
				Some((dimension, scale, zero)) => units.push(Read { name, dimension, scale, zero }),
				None => aliases.push((name, expression)),
			}
		} else if let Some((name, dimension_name)) = line.split_once(BASE) {
			let dimension = dimensions
				.iter()
				.find(|(known, _)| *known == dimension_name.trim())
				.ok_or(format!("{name}: undeclared dimension {dimension_name}"))?
				.1;
			base_units.push((dimension, name.trim()));
			units.push(Read { name: name.trim(), dimension, scale: Rational::integer(1), zero: Rational::zero() });
		} else {
			return Err(format!("unreadable line {line}"));
		}
	}
	Ok(Definitions { dimensions, derived_dimensions, units: in_whole_steps(units)?, base_units, aliases })
}

/// `1000 m`, `m/100`, `5/9 K + 45967/180`: the dimension, scale and zero of a unit that is an amount of one unit plus an
/// optional plain offset; None for an expression of several units or of none (an alias)
fn unit_of(expression: &str, units: &[Read]) -> Result<Option<(Dimension, Rational, Rational)>, String> {
	let terms: Vec<_> = expression.split('+').map(|term| product(term, units)).collect::<Result<_, _>>()?;
	let (amount, unit, offset) = match terms.as_slice() {
		[Some((amount, Some(unit)))] => (amount, *unit, Rational::zero()),
		[Some((amount, Some(unit))), Some((offset, None))] => (amount, *unit, offset.clone()),
		_ => return Ok(None),
	};
	// the zero counts in the unit written, which may count from a zero of its own
	let zero = offset.mul(&unit.scale).add(&unit.zero);
	Ok(Some((unit.dimension, amount.mul(&unit.scale), zero)))
}

/// A product of numbers and at most one unit, not divided by (`5/9 K`, `m/100`); None for any other term
fn product<'a>(term: &str, units: &'a [Read]) -> Result<Option<(Rational, Option<&'a Read>)>, String> {
	let mut amount = Rational::integer(1);
	let mut unit = None;
	let mut divide = false;
	for token in tokens(term) {
		if token == "/" {
			divide = true;
			continue;
		}
		let factor = match (decimal(token), units.iter().find(|read| read.name == token)) {
			(Some(number), _) => number,
			(None, Some(found)) if unit.is_none() && !divide => {
				unit = Some(found);
				Rational::integer(1)
			}
			// several units, a unit divided by, or a word of no unit (π)
			_ => return Ok(None),
		};
		amount = match divide {
			true => amount.mul(&factor.inverse().ok_or(format!("{term} divides by 0"))?),
			false => amount.mul(&factor),
		};
		divide = false;
	}
	Ok(Some((amount, unit)))
}

/// `length/duration`, `length²`, `1/duration`: the powers of the named dimensions
fn powers_of(expression: &str, dimensions: &[(&str, Dimension)]) -> Result<Vec<(Dimension, i32)>, String> {
	let mut powers: Vec<(Dimension, i32)> = vec![];
	let mut sign = 1;
	for token in tokens(expression) {
		match token {
			"/" => sign = -1,
			"1" => {}
			word => {
				let (name, power) = match word.strip_suffix('²').or(word.strip_suffix('³')) {
					Some(name) => (name, if word.ends_with('²') { 2 } else { 3 }),
					None => (word, 1),
				};
				let dimension =
					dimensions.iter().find(|(known, _)| *known == name).ok_or(format!("unknown dimension {name}"))?.1;
				powers.push((dimension, sign * power));
				sign = 1;
			}
		}
	}
	Ok(powers)
}

/// `5/9 K` → `5`, `/`, `9`, `K`
fn tokens(term: &str) -> Vec<&str> {
	let mut tokens = vec![];
	for word in term.split_whitespace() {
		let mut rest = word;
		while let Some(at) = rest.find('/') {
			tokens.extend([&rest[..at], "/"].into_iter().filter(|token| !token.is_empty()));
			rest = &rest[at + 1..];
		}
		if !rest.is_empty() {
			tokens.push(rest);
		}
	}
	tokens
}

/// `2.54` as the exact 127/50, `1000` as 1000; None for a word
fn decimal(token: &str) -> Option<Rational> {
	let (whole, fraction) = token.split_once('.').unwrap_or((token, ""));
	if whole.is_empty() || !whole.chars().chain(fraction.chars()).all(|c| c.is_ascii_digit()) {
		return None;
	}
	let numerator: BigInt = format!("{whole}{fraction}").parse().ok()?;
	Some(Rational::new(numerator, BigInt::from(10).pow(fraction.len() as u32)))
}

/// Units in whole steps of their dimension: each amount times the least common multiple of the dimension's
/// denominators (only their ratios matter: 1 m is 50 steps of cm's 1, km 50000)
fn in_whole_steps(units: Vec<Read>) -> Result<Vec<Unit>, String> {
	let step = |dimension: Dimension| {
		units
			.iter()
			.filter(|read| read.dimension == dimension)
			.flat_map(|read| [&read.scale.denominator, &read.zero.denominator])
			.fold(BigInt::one(), |multiple, denominator| multiple.lcm(denominator))
	};
	let whole = |amount: &Rational, steps: &BigInt| {
		let whole = amount.mul(&Rational::new(steps.clone(), BigInt::one()));
		whole.numerator.to_i64().filter(|_| whole.is_integer())
	};
	units
		.iter()
		.map(|read| {
			let steps = step(read.dimension);
			let factor = whole(&read.scale, &steps).ok_or(format!("{} is too fine", read.name))?;
			let zero = whole(&read.zero, &steps).ok_or(format!("{} has too fine a zero", read.name))?;
			Ok(Unit { name: read.name, dimension: read.dimension, factor, zero })
		})
		.collect()
}
