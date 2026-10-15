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
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const SOURCE: &str = include_str!("../../../lib/units.warp");
const BLOCK_END: &str = "}";
const DIMENSION_WORD: &str = "dimension ";
const DEFINE: &str = ":=";
const BASE: char = ':';
const COMMENT: &str = "//";
const MULTIPLY: [char; 2] = ['*', '·'];
/// The names of the run-time tables lib/units.warp's Quantity reads, in place of the block (run_time_tables)
pub(super) const BLOCK_NAME: &str = "unit_definitions";
const RUN_TIME_NAMES: &str = "unit_names";
const RUN_TIME_SCALES: &str = "unit_scales";
const RUN_TIME_DIMENSIONS: &str = "unit_dimensions";
/// The dimensions a run-time quantity knows, packed into one int: length + 256·mass + 65536·time
const RUN_TIME_DIMENSION_WEIGHTS: [(Dimension, i64); 3] =
	[(Dimension::Length, 1), (Dimension::Mass, 256), (Dimension::Time, 65536)];
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
	static LIB: OnceLock<Definitions> = OnceLock::new();
	let lib = LIB
		.get_or_init(|| read(lib_lines()).unwrap_or_else(|problem| panic!("lib/units.warp {BLOCK_NAME}: {problem}")));
	crate::compiler::compile_context::read(|context| match context.program_unit_lines.is_empty() {
		true => lib,
		false => with_program_units(&context.program_unit_lines),
	})
}

/// The definitions of lib/units.warp and a program's own units, read once per distinct program_unit_lines
fn with_program_units(lines: &str) -> &'static Definitions {
	static EXTENDED: OnceLock<Mutex<HashMap<String, &'static Definitions>>> = OnceLock::new();
	let mut extended = EXTENDED.get_or_init(Default::default).lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	extended.entry(lines.to_string()).or_insert_with(|| {
		let lines: &'static str = Box::leak(lines.into());
		let read = read(lib_lines().chain(lines.lines())).expect("program_unit_lines keeps readable units only");
		Box::leak(Box::new(read))
	})
}

/// A program's own units (decision dimensions-units, user 2026-10-15): of its top-level definitions (`furlong :=
/// 201.168*m`), those that define a new unit as the lines of the block do, one per line. They stay the program's
/// constants and are also units to convert to (`x as furlong`)
pub(super) fn program_unit_lines(definitions: &[String]) -> String {
	let mut reader = Reader::default();
	lib_lines().for_each(|line| reader.line(line).expect("lib/units.warp is read already"));
	let mut lines = vec![];
	for definition in definitions {
		if reader.defines_new_unit(definition) && reader.line(definition).is_ok() {
			lines.push(definition.as_str());
		}
	}
	lines.join("\n")
}

/// Is `name` a unit the program defines itself (program_unit_lines), a constant of the program too
pub(super) fn is_program_unit(name: &str) -> bool {
	crate::compiler::compile_context::read(|context| {
		context
			.program_unit_lines
			.lines()
			.any(|line| line.split_once(DEFINE).is_some_and(|(own, _)| own.trim() == name))
	})
}

/// The lines of lib/units.warp's block that define something
fn lib_lines() -> impl Iterator<Item = &'static str> {
	let block = SOURCE.split_once(&format!("{BLOCK_NAME} = data {{")).map_or("", |(_, block)| block);
	block
		.lines()
		.map(str::trim)
		.take_while(|line| *line != BLOCK_END)
		.filter(|line| !line.is_empty() && !line.starts_with(COMMENT))
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
struct Read<'a> {
	name: &'a str,
	dimension: Dimension,
	scale: Rational,
	zero: Rational,
}

fn read(lines: impl Iterator<Item = &'static str>) -> Result<Definitions, String> {
	let mut reader = Reader::default();
	for line in lines {
		reader.line(line)?;
	}
	let Reader { dimensions, derived_dimensions, base_units, units, aliases } = reader;
	Ok(Definitions { dimensions, derived_dimensions, units: in_whole_steps(units)?, base_units, aliases })
}

/// The definitions read so far, one line at a time
#[derive(Default)]
struct Reader<'a> {
	dimensions: Vec<(&'a str, Dimension)>,
	derived_dimensions: Vec<(&'a str, Vec<(Dimension, i32)>)>,
	base_units: Vec<(Dimension, &'a str)>,
	units: Vec<Read<'a>>,
	aliases: Vec<(&'a str, &'a str)>,
}

impl<'a> Reader<'a> {
	fn line(&mut self, line: &'a str) -> Result<(), String> {
		if let Some((name, expression)) = line.strip_prefix(DIMENSION_WORD).and_then(|rest| rest.split_once(DEFINE)) {
			self.derived_dimensions.push((name.trim(), powers_of(expression, &self.dimensions)?));
		} else if let Some(name) = line.strip_prefix(DIMENSION_WORD) {
			let dimension = DIMENSION_WORDS
				.iter()
				.find(|(word, _)| *word == name.trim())
				.ok_or(format!("unknown dimension {name}"))?;
			self.dimensions.push((name.trim(), dimension.1));
		} else if let Some((name, expression)) = line.split_once(DEFINE) {
			let (name, expression) = (name.trim(), expression.trim());
			match unit_of(expression, &self.units)? {
				Some((dimension, scale, zero)) => self.units.push(Read { name, dimension, scale, zero }),
				None => self.aliases.push((name, expression)),
			}
		} else if let Some((name, dimension_name)) = line.split_once(BASE) {
			let dimension = self
				.dimensions
				.iter()
				.find(|(known, _)| *known == dimension_name.trim())
				.ok_or(format!("{name}: undeclared dimension {dimension_name}"))?
				.1;
			self.base_units.push((dimension, name.trim()));
			self.units.push(Read { name: name.trim(), dimension, scale: Rational::integer(1), zero: Rational::zero() });
		} else {
			return Err(format!("unreadable line {line}"));
		}
		Ok(())
	}

	/// `furlong := 201.168 m`: a word not known yet defined as an amount of one known unit
	fn defines_new_unit(&self, statement: &str) -> bool {
		let Some((name, expression)) = statement.split_once(DEFINE) else {
			return false;
		};
		let name = name.trim();
		let known =
			self.units.iter().any(|read| read.name == name) || self.aliases.iter().any(|(alias, _)| *alias == name);
		let word = !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_');
		word && !known && matches!(unit_of(expression.trim(), &self.units), Ok(Some(_)))
	}
}

/// `1000 m`, `m/100`, `5/9 K + 45967/180`: the dimension, scale and zero of a unit that is an amount of one unit plus an
/// optional plain offset; None for an expression of several units or of none (an alias)
fn unit_of(expression: &str, units: &[Read<'_>]) -> Result<Option<(Dimension, Rational, Rational)>, String> {
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
fn product<'a, 'n>(term: &str, units: &'a [Read<'n>]) -> Result<Option<(Rational, Option<&'a Read<'n>>)>, String> {
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
	// `14 * 24 h` is `14 24 h`: the factors of a product are its words
	for word in term.split(|c: char| c.is_whitespace() || MULTIPLY.contains(&c)).filter(|word| !word.is_empty()) {
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
fn in_whole_steps(units: Vec<Read<'static>>) -> Result<Vec<Unit>, String> {
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
