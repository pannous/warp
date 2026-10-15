//! The dimensions and units warp knows, read once from lib/units.warp's `unit_definitions` block (card dimensions-units
//! Q2, decision unit-definitions): `dimension length meter` names a dimension and its anchor unit, `dimension speed m/s`
//! the dimension of an expression of units; `unit cm := m/100` is a unit that is an amount of another, `unit °C := (it +
//! 273.15) K` one counting from another zero. A unit of several units or of none (`unit mph := mi/h`, `unit ° := π/180`)
//! is an alias, a word for its expression. Read by its own small reader, not the parser: the parser asks these
//! definitions whether a word is a unit.
use super::{Dimension, Unit};
use crate::extensions::reals::Rational;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, ToPrimitive, Zero};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const SOURCE: &str = include_str!("../../../lib/units.warp");
const BLOCK_END: &str = "}";
const DIMENSION_WORD: &str = "dimension ";
const UNIT_WORD: &str = "unit ";
const DEFINE: &str = ":=";
/// `unit m := 1 meter`: a unit of the same amount as an anchor
const ONE: &str = "1 ";
/// `(it + 273.15) K`: a unit counting from another zero shifts its amount `it` first
const SHIFT_START: &str = "(it +";
/// `unit °R := it * 5/9 K`: a unit of the same zero, its amount `it` scaled
const IT_TIMES: &str = "it *";
/// `it`, the amount in the unit a unit line defines; `unit °C as K := it + 273.15` shifts it
const IT: &str = "it";
const IT_PLUS: &str = "it +";
/// `length as pixels := it * display.dpi`: a bridge between two dimensions
const BRIDGE_WORD: &str = " as ";
/// `display.dpi := 96 dpi`: a setting, a property of a platform object
const SETTING_DOT: char = '.';
const TIMES: char = '*';
const STATEMENT_END: char = ';';
const DIVIDE: &str = "/";
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
	("pixels", Dimension::Pixel),
];

/// The dimensions whose units take SI prefixes (decision unit-definitions: not pixels, not eras, not furlongs)
const SI_DIMENSIONS: [Dimension; 4] = [Dimension::Length, Dimension::Mass, Dimension::Time, Dimension::Temperature];
/// The SI prefixes a unit of an SI dimension takes by itself (`km`, `µs`, `GK`), each a power of ten; giga down to nano
/// keep every unit a whole number of its dimension's finest step within 64 bits
const SI_PREFIXES: [(&str, i32); 10] =
	[("G", 9), ("M", 6), ("k", 3), ("h", 2), ("da", 1), ("d", -1), ("c", -2), ("m", -3), ("µ", -6), ("n", -9)];

pub(super) struct Definitions {
	/// The dimensions by name, in written order, and the names of products of them (`speed := length/duration`)
	pub(super) dimensions: Vec<(&'static str, Dimension)>,
	pub(super) derived_dimensions: Vec<(&'static str, Vec<(Dimension, i32)>)>,
	pub(super) units: Vec<Unit>,
	/// The base unit of each dimension, by name: the unit a quantity's amount counts in at run time
	pub(super) base_units: Vec<(Dimension, &'static str)>,
	pub(super) aliases: Vec<(&'static str, &'static str)>,
	pub(super) bridges: Vec<Bridge<'static>>,
	pub(super) settings: Vec<Setting<'static>>,
}

/// `length as pixels := it * display.dpi`: an amount of `from` is `factor` times as much of `to`, the factor read at
/// each conversion (decision dimension-bridges); the way back divides by it
#[derive(Clone)]
pub(super) struct Bridge<'a> {
	pub(super) from: Dimension,
	pub(super) to: Dimension,
	pub(super) factor: &'a str,
}

/// `display.dpi := 96 dpi`: a value of the platform a program may set and the unit it counts in; the default is the
/// host's (warp-runtime system_values.rs DEFAULT_DPI, where the screen tells none)
#[derive(Clone)]
pub(super) struct Setting<'a> {
	pub(super) name: &'a str,
	pub(super) unit: &'a str,
}

/// lib/units.warp's definitions, and the program's own unit lines while it is compiled (context.program_unit_lines)
pub(super) fn definitions() -> &'static Definitions {
	static LIB: OnceLock<Definitions> = OnceLock::new();
	let lines = crate::compiler::compile_context::read(|context| context.program_unit_lines.clone());
	if lines.is_empty() {
		return LIB
			.get_or_init(|| read(std::iter::empty()).unwrap_or_else(|problem| panic!("lib/units.warp: {problem}")));
	}
	with_program_units(lines)
}

/// The definitions with a program's unit lines, read once per distinct lines (kept for the process, like the lib's)
fn with_program_units(lines: String) -> &'static Definitions {
	static READ: OnceLock<Mutex<HashMap<String, &'static Definitions>>> = OnceLock::new();
	let mut read_before = READ.get_or_init(Default::default).lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	read_before.entry(lines.clone()).or_insert_with(|| {
		let lines: &'static str = Box::leak(lines.into_boxed_str());
		Box::leak(Box::new(read(lines.lines()).expect("program_unit_lines keeps readable lines only")))
	})
}

/// A program's own units (decision unit-definitions): its lines `unit furlong := 201.168 m` at the start of a line,
/// read before the program is parsed, so `3 furlong` binds like `3 m`; a line the reader can't read stays in the
/// program, where the units lowering reports it
pub fn program_unit_lines(source: &str) -> String {
	let mut reader = lib_reader();
	// `unit furlong := 201.168m; print(…)`: a unit line ends at its line's `;`
	let lines = source.lines().map(|line| line.split(STATEMENT_END).next().unwrap_or_default().trim_end());
	let lines = lines.filter(|line| line.starts_with(UNIT_WORD) && line.contains(DEFINE));
	lines.filter(|line| reader.knows_words_of(line) && reader.line(line).is_ok()).collect::<Vec<_>>().join("\n")
}

/// The bridges between dimensions (`length as pixels := it * display.dpi`): each with the host's system value its factor
/// reads (`display_dpi`) and the SI amount of one of the setting's unit (1 dpi is 5000/127 px/m)
pub(super) fn bridges() -> Vec<(Dimension, Dimension, String, Rational)> {
	let factor = |bridge: &Bridge| {
		let setting = definitions().settings.iter().find(|setting| setting.name == bridge.factor)?;
		Some((bridge.from, bridge.to, setting.name.replace(SETTING_DOT, "_"), measure(setting.unit)?.0))
	};
	definitions().bridges.iter().filter_map(factor).collect()
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

/// `mi/h`, `1/s`, `kg*m/s²`: the SI amount and packed dimension of an alias made of units; None for one of other words
/// (`π/180`)
fn alias_measure(expression: &str) -> Option<(Rational, i64)> {
	let (amount, dimension) = measure(expression)?;
	Some((amount, dimension?))
}

/// The SI amount of a product of numbers and units, and its packed dimension where run-time quantities know it (not
/// of `px/inch`)
fn measure(expression: &str) -> Option<(Rational, Option<i64>)> {
	let (mut amount, mut dimension, mut divide) = (Rational::integer(1), Some(0), false);
	for token in tokens(expression) {
		if token == DIVIDE {
			divide = true;
			continue;
		}
		let (factor, weight) = match decimal(token) {
			Some(number) => (number, Some(0)),
			None => {
				let (name, power) = powered(token);
				let (amount, weight) = match definitions().units.iter().find(|unit| unit.name == name) {
					Some(unit) => (si_amount(unit), packed(unit.dimension)),
					// `dpi`: a word for units, `px/inch`
					None => measure(definitions().aliases.iter().find(|(alias, _)| *alias == name)?.1)?,
				};
				let weight = weight.map(|weight| weight * power as i64);
				((1..power).fold(amount.clone(), |total, _| total.mul(&amount)), weight)
			}
		};
		let weight = |dimension: Option<i64>| Some(dimension? + weight? * if divide { -1 } else { 1 });
		(amount, dimension) = match divide {
			true => (amount.mul(&factor.inverse()?), weight(dimension)),
			false => (amount.mul(&factor), weight(dimension)),
		};
		divide = false;
	}
	Some((amount, dimension))
}

/// A unit as it is read: its amount and zero in its dimension's base unit, before they become whole steps
#[derive(Clone)]
struct Read<'a> {
	name: &'a str,
	dimension: Dimension,
	scale: Rational,
	zero: Rational,
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

/// lib/units.warp's lines, its units with their SI prefixes, read once
fn lib_reader() -> Reader<'static> {
	static LIB: OnceLock<Reader<'static>> = OnceLock::new();
	let read = || -> Result<Reader<'static>, String> {
		let mut reader = Reader::default();
		lib_lines().try_for_each(|line| reader.line(line))?;
		let prefixed = with_si_prefixes(&reader.units, &reader.aliases);
		reader.units.extend(prefixed);
		Ok(reader)
	};
	LIB.get_or_init(|| read().unwrap_or_else(|problem| panic!("lib/units.warp: {problem}"))).clone()
}

/// lib/units.warp's definitions, then a program's own unit lines
fn read(program_lines: impl Iterator<Item = &'static str>) -> Result<Definitions, String> {
	let mut reader = lib_reader();
	program_lines.into_iter().try_for_each(|line| reader.line(line))?;
	let Reader { dimensions, derived_dimensions, base_units, units, aliases, bridges, settings } = reader;
	let units = in_whole_steps(units)?;
	Ok(Definitions { dimensions, derived_dimensions, units, base_units, aliases, bridges, settings })
}

/// The definitions read so far, one line at a time
#[derive(Clone, Default)]
struct Reader<'a> {
	dimensions: Vec<(&'a str, Dimension)>,
	derived_dimensions: Vec<(&'a str, Vec<(Dimension, i32)>)>,
	base_units: Vec<(Dimension, &'a str)>,
	units: Vec<Read<'a>>,
	aliases: Vec<(&'a str, &'a str)>,
	bridges: Vec<Bridge<'a>>,
	settings: Vec<Setting<'a>>,
}

impl<'a> Reader<'a> {
	/// `unit furlong := 201.168 m`: each word of the right side a number or a unit read so far (lib/units.warp may also
	/// name words of no unit, `unit ° := π/180`)
	fn knows_words_of(&self, line: &str) -> bool {
		let (name, expression) = line.split_once(DEFINE).unwrap_or_default();
		let target = name.split_once(BRIDGE_WORD).map(|(_, target)| target.trim());
		let expression = targeted(expression.trim(), target);
		let expression = shifted(&expression).map_or(expression.as_str(), |(_, rest)| rest);
		tokens(expression)
			.into_iter()
			.all(|token| [DIVIDE, IT].contains(&token) || decimal(token).is_some() || self.knows(powered(token).0))
	}

	fn line(&mut self, line: &'a str) -> Result<(), String> {
		let definition = |keyword: &str, separator: &str| {
			let (name, expression) = line.strip_prefix(keyword)?.split_once(separator)?;
			Some((name.trim(), expression.trim()))
		};
		if let Some((name, anchor)) = definition(DIMENSION_WORD, " ") {
			match DIMENSION_WORDS.iter().find(|(word, _)| *word == name) {
				Some((_, dimension)) => {
					self.dimensions.push((name, *dimension));
					self.base_units.push((*dimension, anchor));
					let scale = Rational::integer(1);
					self.units.push(Read { name: anchor, dimension: *dimension, scale, zero: Rational::zero() });
				}
				None => self.derived_dimensions.push((name, powers_of(anchor, &self.units)?)),
			}
			return Ok(());
		}
		if let Some((name, expression)) = definition(UNIT_WORD, DEFINE) {
			return self.unit_line(name, expression);
		}
		if let Some((name, expression)) = definition("", DEFINE).filter(|(name, _)| name.contains(SETTING_DOT)) {
			let (amount, unit) = expression.split_once(' ').ok_or(format!("{line}: an amount and its unit"))?;
			decimal(amount).ok_or(format!("{line}: {amount} is no number"))?;
			self.settings.push(Setting { name, unit: unit.trim() });
			return Ok(());
		}
		if let Some((dimensions, expression)) = definition("", DEFINE).filter(|(name, _)| name.contains(BRIDGE_WORD)) {
			let (from, to) = dimensions.split_once(BRIDGE_WORD).ok_or(format!("unreadable line {line}"))?;
			let dimension = |name: &str| {
				let known = self.dimensions.iter().find(|(known, _)| *known == name.trim());
				known.map(|(_, dimension)| *dimension).ok_or(format!("{line}: {name} is no dimension"))
			};
			let factor = expression.strip_prefix(IT_TIMES).ok_or(format!("{line}: the right side is it * a value"))?;
			self.bridges.push(Bridge { from: dimension(from)?, to: dimension(to)?, factor: factor.trim() });
			return Ok(());
		}
		Err(format!("unreadable line {line}"))
	}

	/// `unit cm := m/100`, `unit °C as K := it + 273.15` (the unit `as` names is the right side's, decision unit-target)
	fn unit_line(&mut self, name: &'a str, expression: &'a str) -> Result<(), String> {
		let (name, target) = match name.split_once(BRIDGE_WORD) {
			Some((name, target)) => (name.trim(), Some(self.symbol_of(target.trim()))),
			None => (name, None),
		};
		if self.knows(name) {
			return Err(format!("{name} is a unit already"));
		}
		let anchors = &self.base_units;
		let one_anchor = expression.strip_prefix(ONE).filter(|anchor| anchors.iter().any(|(_, base)| base == anchor));
		if let Some(anchor) = one_anchor.filter(|_| target.is_none()) {
			// `unit m := 1 meter`: the symbol shows in place of its anchor, the anchor's name stays a word for it
			self.units.iter_mut().filter(|unit| unit.name == anchor).for_each(|unit| unit.name = name);
			self.base_units.iter_mut().filter(|(_, base)| *base == anchor).for_each(|(_, base)| *base = name);
			self.aliases.push((anchor, name));
			return Ok(());
		}
		match (unit_of(&targeted(expression, target), &self.units)?, target) {
			(Some((dimension, scale, zero)), _) => self.units.push(Read { name, dimension, scale, zero }),
			(None, None) => self.aliases.push((name, expression)),
			(None, Some(target)) => return Err(format!("{name}: {expression} is no amount of {target}")),
		}
		Ok(())
	}

	fn knows(&self, name: &str) -> bool {
		self.units.iter().any(|unit| unit.name == name) || self.aliases.iter().any(|(alias, _)| *alias == name)
	}

	/// `kelvin` → `K`: a unit by its symbol, where an alias names one unit
	fn symbol_of(&self, name: &'a str) -> &'a str {
		let alias = self.aliases.iter().find(|(alias, _)| *alias == name);
		alias
			.map(|(_, symbol)| *symbol)
			.filter(|symbol| self.units.iter().any(|unit| unit.name == *symbol))
			.unwrap_or(name)
	}
}

/// The right side of a unit line in the unit `as` names: `it + 273.15` as K → `(it + 273.15) K`, `it * 5/9` → `it * 5/9 K`
fn targeted(expression: &str, target: Option<&str>) -> String {
	match (target, expression.strip_prefix(IT_PLUS)) {
		(None, _) => expression.to_string(),
		(Some(target), Some(shift)) => format!("(it + {}) {target}", shift.trim()),
		(Some(target), None) => format!("{expression} {target}"),
	}
}

/// `km`, `µg`, `ns`: each SI prefix on each unit of an SI dimension that is a power of ten of its anchor and carries no
/// prefix itself (`g` takes them, `kg` and `cm` do not); a unit written in lib/units.warp keeps its own definition
fn with_si_prefixes<'a>(units: &[Read<'a>], aliases: &[(&'a str, &'a str)]) -> Vec<Read<'a>> {
	let known =
		|name: &str| units.iter().any(|unit| unit.name == name) || aliases.iter().any(|(alias, _)| *alias == name);
	let prefixed = |name: &str| SI_PREFIXES.iter().any(|(prefix, _)| name.strip_prefix(prefix).is_some_and(known));
	let takes_prefixes = |unit: &&Read<'a>| {
		SI_DIMENSIONS.contains(&unit.dimension)
			&& unit.zero.is_zero()
			&& power_of_ten(&unit.scale)
			&& !prefixed(unit.name)
	};
	units
		.iter()
		.filter(takes_prefixes)
		.flat_map(|unit| SI_PREFIXES.iter().map(move |(prefix, power)| (unit, prefix, power)))
		.filter_map(|(unit, prefix, power)| {
			let name: &'static str = Box::leak(format!("{prefix}{}", unit.name).into_boxed_str());
			let ten = Rational::integer(BigInt::from(10).pow(power.unsigned_abs()));
			let factor = if *power > 0 { ten } else { ten.inverse()? };
			(!known(name)).then(|| Read {
				name,
				dimension: unit.dimension,
				scale: unit.scale.mul(&factor),
				zero: Rational::zero(),
			})
		})
		.collect()
}

/// 1000, 1/100, 1: a whole power of ten, up or down
fn power_of_ten(amount: &Rational) -> bool {
	let is_power = |number: &BigInt| {
		let mut number = number.clone();
		while (&number % 10u32).is_zero() && !number.is_zero() {
			number /= 10u32;
		}
		number.is_one()
	};
	is_power(&amount.numerator) && is_power(&amount.denominator)
}

/// `1000 m`, `m/100`, `(it + 459.67) * 5/9 K`: the dimension, scale and zero of a unit that is an amount of one unit,
/// counting from another zero where `it` is shifted first; None for an expression of several units or of none (an alias)
fn unit_of(expression: &str, units: &[Read<'_>]) -> Result<Option<(Dimension, Rational, Rational)>, String> {
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

/// `(it + 273.15) K` → `273.15` and `K`, `it * 5/9 K` → `0` and `5/9 K`, `it / 100 m` → `0` and `/ 100 m`; None for an
/// amount of no `it`
fn shifted(expression: &str) -> Option<(&str, &str)> {
	let scaled = expression.strip_prefix(IT).map(str::trim_start).filter(|rest| rest.starts_with([TIMES, '/']));
	if let Some(rest) = scaled {
		return Some(("0", rest.trim_start_matches(TIMES).trim()));
	}
	let (shift, rest) = expression.strip_prefix(SHIFT_START)?.split_once(')')?;
	Some((shift.trim(), rest.trim()))
}

/// A product of numbers and at most one unit, not divided by (`5/9 K`, `m/100`); None for any other term
fn product<'a, 'n>(term: &str, units: &'a [Read<'n>]) -> Result<Option<(Rational, Option<&'a Read<'n>>)>, String> {
	let mut amount = Rational::integer(1);
	let mut unit = None;
	let mut divide = false;
	// `4 * it inch`: the amount `it` anywhere in the product
	for token in tokens(term).into_iter().filter(|token| *token != IT) {
		if token == DIVIDE {
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
fn powers_of(expression: &str, units: &[Read<'_>]) -> Result<Vec<(Dimension, i32)>, String> {
	let mut powers: Vec<(Dimension, i32)> = vec![];
	let mut sign = 1;
	for token in tokens(expression) {
		match token {
			DIVIDE => sign = -1,
			"1" => {}
			word => {
				let (name, power) = powered(word);
				let dimension =
					units.iter().find(|unit| unit.name == name).ok_or(format!("unknown unit {name}"))?.dimension;
				powers.push((dimension, sign * power));
				sign = 1;
			}
		}
	}
	Ok(powers)
}

/// `5/9 K` → `5`, `/`, `9`, `K`; `kg*m` → `kg`, `m`
fn tokens(term: &str) -> Vec<&str> {
	let mut tokens = vec![];
	for word in term.split(|c: char| c.is_whitespace() || c == TIMES) {
		let mut rest = word;
		while let Some(at) = rest.find(DIVIDE) {
			tokens.extend([&rest[..at], DIVIDE].into_iter().filter(|token| !token.is_empty()));
			rest = &rest[at + 1..];
		}
		// `201.168m`: a number glued to its unit
		let number = rest.find(|c: char| !c.is_ascii_digit() && c != '.').filter(|at| *at > 0).unwrap_or(rest.len());
		tokens.extend([&rest[..number], &rest[number..]].into_iter().filter(|token| !token.is_empty()));
	}
	tokens
}

/// `s²` → `s` and 2, `m` → `m` and 1
fn powered(word: &str) -> (&str, i32) {
	match word.strip_suffix('²').or(word.strip_suffix('³')) {
		Some(name) => (name, if word.ends_with('²') { 2 } else { 3 }),
		None => (word, 1),
	}
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
