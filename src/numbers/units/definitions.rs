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
const BLOCK_END: &str = "}";
const DIMENSION_WORD: &str = "dimension ";
const UNIT_WORD: &str = "unit ";
const DEFINE: &str = ":=";
/// `(it + 273.15) K`: a unit counting from another zero shifts its amount `it` first
const SHIFT_START: &str = "(it +";
const TIMES: &str = "*";
const COMMENT: &str = "//";
/// The names of the run-time tables lib/units.warp's Quantity reads, in place of the block (run_time_tables)
pub(super) const BLOCK_NAME: &str = "unit_definitions";
const RUN_TIME_NAMES: &str = "unit_names";
const RUN_TIME_SCALES: &str = "unit_scales";
const RUN_TIME_DIMENSIONS: &str = "unit_dimensions";
/// The dimensions a run-time quantity knows, packed into one int: length + 256·mass + 65536·time
const RUN_TIME_DIMENSION_WEIGHTS: [(Dimension, i64); 3] =
	[(Dimension::Length, 1), (Dimension::Mass, 256), (Dimension::Time, 65536)];
/// The dimension each name of lib/units.warp stands for: time's is `duration` (decision anchor-time)
const DIMENSION_WORDS: [(&str, Dimension); 6] = [
	("length", Dimension::Length),
	("mass", Dimension::Mass),
	("duration", Dimension::Time),
	("era", Dimension::Era),
	("temperature", Dimension::Temperature),
	("pixel", Dimension::Pixel),
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

/// The run-time tables of lib/units.warp's Quantity (`quantity(5, "km")`), in place of the block itself: each unit and
/// each alias of units (`mph`, `Hz`) with its SI amount and its dimension packed as length + 256·mass + 65536·time
pub(super) fn run_time_tables() -> String {
	let rows: Vec<(&str, Rational, i64)> = definitions()
		.units
		.iter()
		.filter_map(|unit| Some((unit.name, si_amount(unit), packed(unit.dimension)?)))
		.chain(definitions().aliases.iter().filter_map(|(name, expression)| {
			let (amount, dimension) = alias_measure(expression)?;
			Some((*name, amount, dimension))
		}))
		.collect();
	let column = |cell: &dyn Fn(&(&str, Rational, i64)) -> String| rows.iter().map(cell).collect::<Vec<_>>().join(", ");
	format!(
		"{RUN_TIME_NAMES} = [{}]\n{RUN_TIME_SCALES} = [{}]\n{RUN_TIME_DIMENSIONS} = [{}]",
		column(&|(name, _, _)| format!("\"{name}\"")),
		column(&|(_, amount, _)| amount.to_string()),
		column(&|(_, _, dimension)| dimension.to_string()),
	)
}

/// A unit's amount in its dimension's base unit, the SI unit
fn si_amount(unit: &Unit) -> Rational {
	let base =
		definitions().base_units.iter().find(|(dimension, _)| *dimension == unit.dimension).map(|(_, name)| *name);
	let steps = definitions().units.iter().find(|known| Some(known.name) == base).map_or(1, |known| known.factor);
	Rational::new(BigInt::from(unit.factor), BigInt::from(steps))
}

fn packed(dimension: Dimension) -> Option<i64> {
	RUN_TIME_DIMENSION_WEIGHTS.iter().find(|(known, _)| *known == dimension).map(|(_, weight)| *weight)
}

/// `mi/h`, `1/s`: the SI amount and packed dimension of an alias made of units; None for one of other words (`π/180`)
fn alias_measure(expression: &str) -> Option<(Rational, i64)> {
	let (mut amount, mut dimension, mut divide) = (Rational::integer(1), 0, false);
	for token in tokens(expression) {
		if token == "/" {
			divide = true;
			continue;
		}
		let (factor, weight) = match decimal(token) {
			Some(number) => (number, 0),
			None => {
				let unit = definitions().units.iter().find(|unit| unit.name == token)?;
				(si_amount(unit), packed(unit.dimension)?)
			}
		};
		(amount, dimension) = match divide {
			true => (amount.mul(&factor.inverse()?), dimension - weight),
			false => (amount.mul(&factor), dimension + weight),
		};
		divide = false;
	}
	Some((amount, dimension))
}

/// A unit as it is read: its amount and zero in its dimension's base unit, before they become whole steps
struct Read {
	name: &'static str,
	dimension: Dimension,
	scale: Rational,
	zero: Rational,
}

fn read(source: &'static str) -> Result<Definitions, String> {
	let block = source.split_once(&format!("{BLOCK_NAME} = data {{")).ok_or("no block")?.1;
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
		let definition = |keyword: &str| {
			let (name, expression) = line.strip_prefix(keyword)?.split_once(DEFINE)?;
			Some((name.trim(), expression.trim()))
		};
		if let Some((name, anchor)) = definition(DIMENSION_WORD) {
			match DIMENSION_WORDS.iter().find(|(word, _)| *word == name) {
				Some((_, dimension)) => {
					dimensions.push((name, *dimension));
					base_units.push((*dimension, anchor));
					units.push(Read { name: anchor, dimension: *dimension, scale: Rational::integer(1), zero: Rational::zero() });
				}
				None => derived_dimensions.push((name, powers_of(anchor, &units)?)),
			}
		} else if let Some((name, expression)) = definition(UNIT_WORD) {
			match unit_of(expression, &units)? {
				Some((dimension, scale, zero)) => units.push(Read { name, dimension, scale, zero }),
				None => aliases.push((name, expression)),
			}
		} else {
			return Err(format!("unreadable line {line}"));
		}
	}
	Ok(Definitions { dimensions, derived_dimensions, units: in_whole_steps(units)?, base_units, aliases })
}

/// `1000 m`, `m/100`, `(it + 459.67) * 5/9 K`: the dimension, scale and zero of a unit that is an amount of one unit,
/// counting from another zero where `it` is shifted first; None for an expression of several units or of none (an alias)
fn unit_of(expression: &str, units: &[Read]) -> Result<Option<(Dimension, Rational, Rational)>, String> {
	let (offset, amount) = match shifted(expression) {
		Some((offset, rest)) => (decimal(offset).ok_or(format!("{expression}: the shift is no number"))?, rest),
		None => (Rational::zero(), expression),
	};
	let Some((amount, Some(unit))) = product(amount, units)? else {
		return Ok(None);
	};
	// the zero counts in the unit written, which may count from a zero of its own
	let scale = amount.mul(&unit.scale);
	Ok(Some((unit.dimension, scale.clone(), offset.mul(&scale).add(&unit.zero))))
}

/// `(it + 273.15) K` → `273.15` and `K`; None for an amount of no shifted `it`
fn shifted(expression: &str) -> Option<(&str, &str)> {
	let (shift, rest) = expression.strip_prefix(SHIFT_START)?.split_once(')')?;
	Some((shift.trim(), rest.trim()))
}

/// A product of numbers and at most one unit, not divided by (`5/9 K`, `m/100`); None for any other term
fn product<'a>(term: &str, units: &'a [Read]) -> Result<Option<(Rational, Option<&'a Read>)>, String> {
	let mut amount = Rational::integer(1);
	let mut unit = None;
	let mut divide = false;
	for token in tokens(term).into_iter().filter(|token| *token != TIMES) {
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

/// `m/s`, `m²`, `1/s`: the powers of the dimensions of the units named
fn powers_of(expression: &str, units: &[Read]) -> Result<Vec<(Dimension, i32)>, String> {
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
					units.iter().find(|unit| unit.name == name).ok_or(format!("unknown unit {name}"))?.dimension;
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
