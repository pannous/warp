//! Unit words as values (wiki/unit.md): `3km` is `3*km`, `1 m + 1km` is `1001 m`.
//! The units and their scales, the words a program writes them in, and the lowerings around them; static_units checks
//! the units of a program and runs its amounts as exact SI numbers. The unit shown is the finest one written:
//! `3km+10m` is `3010m`. A quantity is an amount times units to powers: `6 m²`, `6 m·kg`, `5 km/h` (km·h⁻¹), `1 kg·m/s²`.

use crate::compiler::meta::DataValue;
use crate::extensions::numbers::Number;
use crate::node::{error, Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashMap;
mod definitions;
mod quantity_arithmetic;
pub use quantity_arithmetic::*;

pub mod static_units;

use crate::extensions::reals::Rational;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Dimension {
	Length,
	Mass,
	Era,
	Time,
	Temperature,
}

#[derive(Debug, PartialEq)]
/// `factor` counts the smallest step of the dimension, so every unit is a whole number of it: time in ms, length in
/// 0.1 mm (1 ft = 3048, 1 km = 10_000_000), mass in 10 µg (1 lb = 45_359_237), temperature in 1/900 K (1 °F = 500).
/// `zero` is where the unit's 0 lies, in steps above the dimension's zero: 0 °C is 273.15 K, 245_835 steps
struct Unit {
	name: &'static str,
	dimension: Dimension,
	factor: i64,
	zero: i64,
}

/// The units of lib/units.warp (definitions.rs)
fn units() -> &'static [Unit] {
	&definitions::definitions().units
}

/// The sign before a temperature unit's letter: °C, °F
pub const DEGREE: char = '°';

/// Unit words standing for a unit expression (lib/units.warp): `60 mph` is `60 mi/h`, `440Hz` is `440/s` (lib/sound.warp),
/// an angle `30°` is the exact `30π/180` (card angles-degrees)
fn unit_aliases() -> &'static [(&'static str, &'static str)] {
	&definitions::definitions().aliases
}

/// The long names a conversion target may use, singular or plural: `2 h in minutes` (P36). Quantities keep the short
/// names, `2 minutes` stays a duration of the time module.
const LONG_NAMES: [(&str, &str); 19] = [
	("millisecond", "ms"),
	("second", "s"),
	("minute", "min"),
	("hour", "h"),
	("millimeter", "mm"),
	("centimeter", "cm"),
	("meter", "m"),
	("metre", "m"),
	("kilometer", "km"),
	("inches", "inch"),
	("foot", "ft"),
	("feet", "ft"),
	("yard", "yd"),
	("mile", "mi"),
	("milligram", "mg"),
	("gram", "g"),
	("kilogram", "kg"),
	("pound", "lb"),
	("lbs", "lb"),
];
// `100 cm in m`: the word of a conversion written with spaces (`as` is an operator)
/// `now` compared with a duration is its local time of day, unless the program names something so (card time-day-value);
/// a bare `time` is the type (user decision print-time), so `time < 12h` fails with the fix `now < 12h`
use crate::time::{NOW_WORD, TIME_WORD};
use crate::words::IN_WORD;

fn unit_named(name: &str) -> Option<&'static Unit> {
	if crate::compiler::compile_context::read(|context| context.shadowed_units.contains(name)) {
		return None;
	}
	units().iter().find(|unit| unit.name == name)
}

/// `run` with the unit words `program` defines as its names, not units: `g(x) := x + 1 m` keeps m a unit
fn shadowing<T>(program: &Node, run: impl FnOnce() -> T) -> T {
	crate::compiler::compile_context::scoped(|context| &mut context.shadowed_units, defined_unit_names(program), run)
}

/// A conversion target: a unit by its short name or its long name, `minute` or `minutes`
fn target_unit(node: &Node) -> Option<&'static Unit> {
	let Node::Symbol(name) = node.drop_meta() else {
		return None;
	};
	let singular = name.strip_suffix('s').unwrap_or(name);
	unit_named(name).or_else(|| {
		LONG_NAMES.iter().find(|(long, _)| *long == singular || *long == name).and_then(|(_, short)| unit_named(short))
	})
}

/// The unit words as the units passes read them: aliases expanded (`60 mph`), the whole unit after `as` (`q as km/h`)
pub fn lower_unit_words(program: Node) -> Node {
	let program = lower_unit_aliases(program);
	let program = match QUANTITY_CAST_WORDS.iter().any(|word| crate::soft_keywords::program_names(&program, word)) {
		true => program,
		false => with_quantity_casts(program),
	};
	shadowing(&program.clone(), || with_whole_conversion_units(program))
}

/// `x as quantity`, and `x as unit` with a got-it note: the run-time quantity(x) (card unit-possible)
const QUANTITY_CAST_WORDS: [&str; 2] = [RUN_TIME_QUANTITY_WORD, "unit"];

fn with_quantity_casts(node: Node) -> Node {
	let node = node.map_children(with_quantity_casts);
	let Node::Key(value, Op::As, target) = node.drop_meta() else {
		return node;
	};
	let Some(word) = target.drop_meta().symbol_name().filter(|word| QUANTITY_CAST_WORDS.contains(word)) else {
		return node;
	};
	if word != RUN_TIME_QUANTITY_WORD {
		crate::normalize::set_position_of(&node);
		crate::errors::diagnostic::note_alias(&format!("as {word}"), &format!("as {RUN_TIME_QUANTITY_WORD}"));
	}
	as_run_time_quantity(value).unwrap_or_else(|| {
		Node::List(
			vec![Node::Symbol(RUN_TIME_QUANTITY_WORD.to_string()), (**value).clone()],
			Bracket::Round,
			Separator::None,
		)
	})
}

/// `60 mi/h as km/h` parses `(60 mi/h as km)/h`: the unit after `as` regrouped whole, as after `in`, when it is one
fn with_whole_conversion_units(node: Node) -> Node {
	let node = match with_written_amount(node.map_children(with_whole_conversion_units)) {
		Node::Key(conversion, op @ (Op::Div | Op::Mul), rest) => match conversion.drop_meta() {
			Node::Key(quantity, Op::As, unit)
				if unit_expression(&Node::Key(unit.clone(), op, rest.clone())).is_some() =>
			{
				Node::Key(quantity.clone(), Op::As, Box::new(Node::Key(unit.clone(), op, rest)))
			}
			_ => Node::Key(conversion, op, rest),
		},
		other => other,
	};
	with_converted_last_operand(node)
}

/// A quantity as it may be written, in the form `amount unit` the units passes read: `1 mile` is `1 mi`,
/// `3km+10m == 3010 meters` gives the unit to the last amount, `(23/18)m/s` (a fraction amount, as one shows) is
/// `(23/18) * (m/s)`, and `6 m² as cm²` (parsed `(6 m² as cm)²`) converts to cm²
fn with_written_amount(node: Node) -> Node {
	match node {
		Node::List(items, Bracket::None, Separator::Space) if items.len() == 2 => {
			let (amount, unit) = (&items[0], &items[1]);
			let short_name = target_unit(unit).map(|unit| Node::Symbol(unit.name.to_string()));
			match (amount.drop_meta(), short_name) {
				(Node::Key(..), Some(short_name)) => with_unit(amount, &short_name),
				(_, Some(short_name)) => Node::List(vec![amount.clone(), short_name], Bracket::None, Separator::Space),
				(Node::List(_, Bracket::Round, _), None) if unit_expression(unit).is_some() => Node::Key(
					Box::new(amount.clone()),
					Op::Mul,
					Box::new(Node::List(vec![unit.clone()], Bracket::Round, Separator::None)),
				),
				_ => Node::List(items, Bracket::None, Separator::Space),
			}
		}
		Node::Key(conversion, op @ (Op::Square | Op::Cube), nothing)
			if nothing.is_empty_node() && powered_conversion(&conversion, op).is_some() =>
		{
			let Node::Key(quantity, Op::As, unit) = conversion.drop_meta().clone() else {
				unreachable!("a powered conversion is an `as`")
			};
			Node::Key(quantity, Op::As, Box::new(Node::Key(unit, op, nothing)))
		}
		other => other,
	}
}

/// `"a: " + x as km` parses `("a: " + x) as km`: a text has no unit to convert, so `as` takes the operand after the
/// last `+` (card print-km)
fn with_converted_last_operand(node: Node) -> Node {
	match node {
		Node::Key(sum, Op::As, unit) if unit_expression(&unit).is_some() => match sum.drop_meta() {
			Node::Key(text, Op::Add, last) if joins_text(text) => {
				Node::Key(text.clone(), Op::Add, Box::new(Node::Key(last.clone(), Op::As, unit)))
			}
			_ => Node::Key(sum, Op::As, unit),
		},
		other => other,
	}
}

/// `"a: "`, `"a: " + x + " or "`: a text, or a sum with a text in it, which every operand joins as its text
pub(crate) fn joins_text(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Text(_) => true,
		Node::Key(left, Op::Add, right) => joins_text(left) || joins_text(right),
		_ => false,
	}
}

/// `60 mph` as `60 mi/h`: each unit alias the program does not define read as its unit expression (not a field name:
/// `r.mph`, `{mph: 3}`)
fn lower_unit_aliases(program: Node) -> Node {
	let aliases: Vec<(&str, &str)> =
		unit_aliases().iter().copied().filter(|(alias, _)| mentions_word(&program, alias)).collect();
	if aliases.is_empty() {
		return program;
	}
	let defined = defined_unit_names(&program);
	let aliases: Vec<(&str, &str)> = aliases.into_iter().filter(|(alias, _)| !defined.contains(*alias)).collect();
	with_unit_aliases(program, &aliases)
}

fn with_unit_aliases(node: Node, aliases: &[(&str, &str)]) -> Node {
	let expansion = |unit: &Node| match unit.drop_meta() {
		Node::Symbol(name) => {
			aliases.iter().find(|(alias, _)| alias == name).map(|(_, expression)| crate::warp_parser::parse(expression))
		}
		_ => None,
	};
	match node {
		// `60 mph`, `60*mph`: the amount joins the first unit, `60 mi / h`, as a written `60 mi/h` reads
		Node::List(items, Bracket::None, Separator::Space) if items.len() == 2 && expansion(&items[1]).is_some() => {
			let [amount, unit] = <[Node; 2]>::try_from(items).expect("two items");
			times_first_unit(with_unit_aliases(amount, aliases), expansion(&unit).expect("guarded"))
		}
		Node::Key(amount, Op::Mul, unit) if expansion(&unit).is_some() => {
			times_first_unit(with_unit_aliases(*amount, aliases), expansion(&unit).expect("guarded"))
		}
		Node::Symbol(name) => expansion(&Node::Symbol(name.clone())).unwrap_or(Node::Symbol(name)),
		// a field's type, `speed: mph`
		Node::Type { name, body } if body.is_empty_node() && expansion(&name).is_some() => {
			expansion(&name).expect("guarded")
		}
		Node::Type { name, body } => Node::Type { name, body: Box::new(with_unit_aliases(*body, aliases)) },
		Node::Key(object, Op::Dot, field) => Node::Key(Box::new(with_unit_aliases(*object, aliases)), Op::Dot, field),
		Node::Key(field, Op::Colon, value) => Node::Key(field, Op::Colon, Box::new(with_unit_aliases(*value, aliases))),
		other => other.map_children(|child| with_unit_aliases(child, aliases)),
	}
}

/// Whether the word occurs anywhere, class bodies and field types included
fn mentions_word(node: &Node, word: &str) -> bool {
	match node.drop_meta() {
		Node::Symbol(name) => name == word,
		Node::Type { name, body } => mentions_word(name, word) || mentions_word(body, word),
		other => other.parts().into_iter().any(|child| mentions_word(child, word)),
	}
}

fn times_first_unit(amount: Node, expression: Node) -> Node {
	match expression.drop_meta() {
		Node::Key(first, op @ (Op::Mul | Op::Div), rest) => {
			Node::Key(Box::new(times_first_unit(amount, *first.clone())), *op, rest.clone())
		}
		_ => Node::Key(Box::new(amount), Op::Mul, Box::new(expression)),
	}
}

/// A unit word's base dimension and how many of the dimension's smallest steps it is (`km`: Length, 10_000_000)
pub fn dimension_and_factor(name: &str) -> Option<(String, i64)> {
	unit_named(name).map(|unit| (format!("{:?}", unit.dimension), unit.factor))
}

/// `h` as its unit's long name `hour` (the unit word where it has none); none when `program` defines the word itself
pub fn unit_long_name(name: &str, program: &Node) -> Option<&'static str> {
	let unit = shadowing(program, || unit_named(name))?;
	let long_name = LONG_NAMES.iter().find(|(_, short)| *short == unit.name);
	Some(long_name.map_or(unit.name, |(long, _)| *long))
}

pub fn is_unit(name: &str) -> bool {
	unit_named(name).is_some()
}

/// A unit or a unit alias: the parser binds it to the amount before it, `60 mph` is `60*mph`
pub fn names_unit(name: &str) -> bool {
	is_unit(name) || unit_aliases().iter().any(|(alias, _)| *alias == name)
}

/// lib/units.warp's class of quantities whose unit is known only at run time, and the word making one
pub const RUN_TIME_QUANTITY: &str = "Quantity";
const RUN_TIME_QUANTITY_WORD: &str = "quantity";

/// A unit written in the program (`1 m`, `5 m/s`, `6 m²`) as the run-time `quantity(1, "m")`, for where it meets one
/// (card units-mixed)
pub fn as_run_time_quantity(node: &Node) -> Option<Node> {
	let (amount, unit) = unit_literal(node)?;
	Some(run_time_quantity(amount, unit))
}

/// The amount and the unit text of `quantity(amount, "unit")`
pub fn run_time_quantity_parts(node: &Node) -> Option<(&Node, &str)> {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _) => match items.as_slice() {
			[word, amount, unit] if word.drop_meta().name() == RUN_TIME_QUANTITY_WORD => match unit.drop_meta() {
				Node::Text(unit) => Some((amount, unit.as_str())),
				_ => None,
			},
			_ => None,
		},
		_ => None,
	}
}

fn run_time_quantity(amount: Node, unit: String) -> Node {
	Node::List(
		vec![Node::Symbol(RUN_TIME_QUANTITY_WORD.to_string()), amount, Node::Text(unit)],
		Bracket::Round,
		Separator::None,
	)
}

/// `5 m ± 1 cm` as the run-time `quantity(5 ± 1/100, "m")`: its amount an interval in the unit of the value (card
/// plus-minus-units)
pub fn lower_run_time_tolerances(program: Node) -> Node {
	shadowing(&program.clone(), || match has_unit_tolerance(&program) {
		true => with_run_time_tolerances(program),
		false => program,
	})
}

fn has_unit_tolerance(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(value, Op::PlusMinus, spread) if unit_literal(value).is_some() || unit_literal(spread).is_some() => {
			true
		}
		other => other.parts().into_iter().any(has_unit_tolerance),
	}
}

fn with_run_time_tolerances(node: Node) -> Node {
	match node {
		Node::Key(value, Op::PlusMinus, spread)
			if unit_literal(&value).is_some() || unit_literal(&spread).is_some() =>
		{
			run_time_tolerance(&value, &spread).unwrap_or_else(|message| error(&message))
		}
		other => other.map_children(with_run_time_tolerances),
	}
}

/// `5 m ± 1 cm` is `quantity(5 ± 1/100, "m")`, `5 m ± 1` counts the 1 in m, `5 ± 1 cm` the 5 in cm
fn run_time_tolerance(value: &Node, spread: &Node) -> Result<Node, String> {
	let interval = |amount: Node, spread: Node| Node::Key(Box::new(amount), Op::PlusMinus, Box::new(spread));
	let (amount, unit) = match (unit_literal(value), unit_literal(spread)) {
		// `5 m ± 1 cm`: the spread counts in the value's unit, value·spread/value in SI amounts
		(Some((amount, unit)), Some(_)) => {
			let si = |quantity: &Node| match static_units::constant(quantity) {
				Ok(Some((si_amount, signature))) => Ok((static_units::exact_amount(&si_amount), signature)),
				_ => Err(format!("a tolerance applies to numbers and quantities: {}", quantity.serialize())),
			};
			let ((value_si, value_signature), (spread_si, spread_signature)) = (si(value)?, si(spread)?);
			if value_signature != spread_signature {
				return Err("incompatible units".to_string());
			}
			let spread_in_unit = match (static_units::exact_amount(&amount), value_si, spread_si) {
				(Some(amount), Some(value_si), Some(spread_si)) => {
					value_si.inverse().map(|per_value| amount.mul(&spread_si).mul(&per_value))
				}
				_ => None,
			}
			.ok_or_else(|| {
				format!("{} ± {}: write the spread in the value's unit", value.serialize(), spread.serialize())
			})?;
			(interval(amount, quotient_node(&spread_in_unit)), unit)
		}
		(Some((amount, unit)), None) => (interval(amount, spread.clone()), unit),
		(None, Some((amount, unit))) => (interval(value.clone(), amount), unit),
		(None, None) => unreachable!("guarded by has_unit_tolerance"),
	};
	Ok(run_time_quantity(amount, unit))
}

/// `5 m/s` as its amount and unit text: a number times units, then more units multiplied or divided
fn unit_literal(node: &Node) -> Option<(Node, String)> {
	match node.drop_meta() {
		Node::Key(amount, Op::Mul, unit) if matches!(amount.drop_meta(), Node::Number(_)) => {
			Some((amount.drop_meta().clone(), unit_text(unit)?))
		}
		Node::Key(literal, op @ (Op::Mul | Op::Div), unit) => {
			let (amount, written) = unit_literal(literal)?;
			let joint = if *op == Op::Mul { "·" } else { "/" };
			Some((amount, format!("{written}{joint}{}", unit_text(unit)?)))
		}
		Node::List(items, Bracket::Round, _) if items.len() == 1 => unit_literal(&items[0]),
		_ => None,
	}
}

/// `m`, `m²`: a unit's name, to a power
fn unit_text(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Symbol(name) if is_unit(name) => Some(name.clone()),
		Node::Key(base, power @ (Op::Square | Op::Cube), _) => Some(format!("{}{power}", unit_text(base)?)),
		_ => None,
	}
}

/// `meters`, `kilogram`: a long name of a quantity's unit; durations (`2 minutes`) belong to the time module
pub fn is_long_unit_name(name: &str) -> bool {
	!is_unit(name) && target_unit(&Node::Symbol(name.to_string())).is_some_and(|unit| unit.dimension != Dimension::Time)
}

/// A unit to a power within a quantity: km¹, h⁻¹, m²
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Factor {
	unit: &'static Unit,
	power: i32,
}

/// An exact amount (a whole number or a fraction: `3/2 m`, `23/18 m/s`) of a product of units to powers: `3 m`, `6 m²`,
/// `6 m·kg`, `5 km/h` (km·h⁻¹), `1 kg·m/s²`.
/// A quantity equals the bare number of its own unit: `3km+10m == 3010`
#[derive(Clone, Debug, PartialEq)]
pub struct Quantity {
	amount: Rational,
	factors: Vec<Factor>,
}

/// Between the units of a product: `m·kg`
const UNIT_PRODUCT: &str = "·";

/// `m·kg/s²`: the units with a positive power, then those with a negative one after a slash
fn units_text(factors: &[Factor]) -> String {
	let side = |positive: bool| {
		let units = factors.iter().filter(|factor| (factor.power > 0) == positive).map(|factor| {
			match factor.power.unsigned_abs() {
				1 => factor.unit.name.to_string(),
				power => format!("{}{}", factor.unit.name, crate::extensions::reals::superscript(power.into())),
			}
		});
		units.collect::<Vec<String>>().join(UNIT_PRODUCT)
	};
	match (side(true), side(false)) {
		(above, below) if below.is_empty() => above,
		(above, below) if above.is_empty() => format!("1/{below}"),
		(above, below) => format!("{above}/{below}"),
	}
}

/// An amount as it reads back before a unit: 3, 9.81, and a fraction without a finite decimal in parentheses, (23/18)
pub(crate) fn amount_text(amount: &Rational) -> String {
	amount.decimal_text().unwrap_or_else(|| format!("({amount})"))
}

/// The units as they follow an amount, right after it (user 2026-10-08: no space): m, m/s², and /m for 1/m (3/m)
pub(crate) fn unit_suffix(factors: &[Factor]) -> String {
	let units = units_text(factors);
	match units.strip_prefix("1/") {
		Some(below) => format!("/{below}"),
		None => units,
	}
}

impl fmt::Display for Quantity {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}{}", amount_text(&self.amount), unit_suffix(&self.factors))
	}
}

/// A ± amount of units, a final unit field given `5 m ± 1 cm`: `5.000 ± 0.010m`; a Gaussian keeps its σ apart from the
/// unit, `12.00 ± 0.60σ m`, as lib/units.warp quantity_text shows it (card quantity-final)
#[derive(Clone, Debug, PartialEq)]
pub struct UncertainQuantity {
	amount: crate::numbers::uncertain::Uncertain,
	factors: Vec<Factor>,
}

impl UncertainQuantity {
	pub(crate) fn new(amount: crate::numbers::uncertain::Uncertain, factors: &[Factor]) -> UncertainQuantity {
		UncertainQuantity { amount, factors: factors.to_vec() }
	}
}

impl fmt::Display for UncertainQuantity {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let apart = if self.amount.gaussian { " " } else { "" };
		write!(f, "{}{apart}{}", self.amount, unit_suffix(&self.factors))
	}
}

impl Quantity {
	/// Is the amount the whole number n: a quantity equals the bare number of its own unit (`3km+10m == 3010`)
	pub fn is_amount(&self, n: i64) -> bool {
		self.amount == Rational::integer(n)
	}
}

/// Dimension → power, the same for `km/h` and `m/s`; two quantities add or compare only with equal signatures
fn signature(factors: &[Factor]) -> Vec<(Dimension, i32)> {
	let mut powers: Vec<(Dimension, i32)> = vec![];
	for factor in factors {
		match powers.iter_mut().find(|(dimension, _)| *dimension == factor.unit.dimension) {
			Some((_, power)) => *power += factor.power,
			None => powers.push((factor.unit.dimension, factor.power)),
		}
	}
	powers.retain(|(_, power)| *power != 0);
	powers.sort_by_key(|(dimension, _)| *dimension as u8);
	powers
}

/// The text of a unit result held in a `Node::Data`, None for any other data
pub fn describe(data: &DataValue) -> Option<String> {
	if let Some(quantity) = data.downcast_ref::<Quantity>() {
		return Some(quantity.to_string());
	}
	if let Some(uncertain) = data.downcast_ref::<crate::numbers::uncertain::Uncertain>() {
		return Some(uncertain.to_string());
	}
	if let Some(uncertain) = data.downcast_ref::<UncertainQuantity>() {
		return Some(uncertain.to_string());
	}
	if let Some(duration) = data.downcast_ref::<crate::time::Duration>() {
		return Some(duration.to_string());
	}
	data.downcast_ref::<crate::time::Time>().map(|time| time.to_string())
}

/// The unit of a bare number of sleep (decision sleep-seconds)
const SECOND: &str = "s";
/// what the host word sleep takes per second of a run-time number
const MILLISECONDS_PER_SECOND: i64 = 1000;

/// `sleep(1000 ms)`, `sleep 1 s`, `sleep(2 seconds)`, `sleep 2`: a constant duration is its milliseconds, what the host
/// word takes; a run-time number counts seconds as a bare one does, so `sleep(t)` sleeps `t * 1000` milliseconds
/// (async agent, 2026-10-06: quantities do not reach run time yet, notes/units_runtime.md)
pub fn lower_sleep_durations(program: Node) -> Node {
	// a duration whose unit word is one of the program's names (`s = 1; sleep 5 s`) stays as written; `m = 2` leaves
	// `sleep 5 ms` a duration (card sleep-unit)
	fn lower(node: Node, shadowed: &std::collections::HashSet<String>) -> Node {
		match node {
			// the word itself: `(sleep 300); 1` is a statement list whose first item merely starts with sleep
			Node::List(items, bracket, separator)
				if items.len() >= 2
					&& matches!(items[0].drop_meta(), Node::Symbol(word) if word == crate::host::SLEEP) =>
			{
				let duration = match &items[1..] {
					[single] => single.clone(),
					several => Node::List(several.to_vec(), Bracket::None, Separator::Space),
				};
				let shadows = duration.mentions_where(|name| shadowed.contains(name));
				let bare = matches!(duration.drop_meta(), Node::Number(_));
				if bare {
					if let Err(error) = warn_bare_duration(&duration) {
						return error;
					}
				}
				let duration = match bare {
					true => Node::List(vec![duration, Node::Symbol(SECOND.into())], Bracket::None, Separator::Space),
					false => duration,
				};
				let sleeping = |milliseconds: Node| {
					Node::List(vec![items[0].clone(), milliseconds], Bracket::Round, Separator::None)
				};
				match (milliseconds(&duration).filter(|_| !shadows), &items[1..]) {
					(Some(amount), _) => sleeping(Node::int(amount)),
					// a number at run time counts seconds too (decision sleep-seconds): `f(t) := sleep(t)`
					(None, [seconds]) => sleeping(Node::Key(
						Box::new(lower(seconds.clone(), shadowed)),
						Op::Mul,
						Box::new(Node::int(MILLISECONDS_PER_SECOND)),
					)),
					(None, _) => {
						Node::List(items.into_iter().map(|item| lower(item, shadowed)).collect(), bracket, separator)
					}
				}
			}
			other => other.map_children(|child| lower(child, shadowed)),
		}
	}
	let shadowed = defined_unit_names(&program);
	lower(program, &shadowed)
}

/// Comparisons of quantities where the rest of the program runs (card time-day-value): a comparison of constant
/// quantities, also of a main-level variable assigned one once (`x = 2 km; if x > 1500 m : …`), is its answer 1 or 0,
/// and such a variable no other statement reads is dropped; the word `time` of a program that names nothing so is the
/// local time of day of now, compared with a constant duration in milliseconds: `now < 24h` is
/// `system·time of day < 86400000` (lowering/signals/system_values.rs reads it, so `whenever now > 18h {…}` listens too)
pub fn lower_quantity_comparisons(program: Node) -> Node {
	shadowing(&program.clone(), || quantity_comparisons_folded(program))
}

fn quantity_comparisons_folded(program: Node) -> Node {
	if !(needs_quantities(&program) || program.mentions_any(&[NOW_WORD, TIME_WORD])) {
		return program;
	}
	let bound = crate::system_values::assigned_symbols(&program);
	let constants = constant_quantities(&program);
	let clock = !bound.contains(NOW_WORD);
	let time_is_the_type = !bound.contains(TIME_WORD);
	let folded = fold_comparisons(program, &constants, clock, time_is_the_type);
	without_unread_constants(folded, &constants)
}

/// The main-level variables assigned a quantity once, and nowhere else: `x = 2 km`, each with its value
fn constant_quantities(program: &Node) -> Constants {
	let mut assignments: HashMap<String, usize> = HashMap::new();
	program.visit(&mut |part| {
		if let Node::Key(target, Op::Assign | Op::Define | Op::Arrow, _)
		| Node::Key(target, Op::AddAssign | Op::SubAssign | Op::MulAssign | Op::DivAssign, _) = part
		{
			crate::variable_signals::symbols(target)
				.into_iter()
				.for_each(|name| *assignments.entry(name).or_default() += 1);
		}
	});
	let mut constants = Constants::new();
	for statement in crate::variable_signals::main_statements(program).0 {
		let Node::Key(target, Op::Assign, value) = statement.drop_meta() else {
			continue;
		};
		let Node::Symbol(name) = target.drop_meta() else {
			continue;
		};
		let value = with_constants(value, &constants);
		if assignments.get(name) == Some(&1) && needs_quantities(&value) {
			if let Ok(Some((_, signature))) = static_units::constant(&value) {
				if !signature.is_empty() {
					constants.insert(name.clone(), value);
				}
			}
		}
	}
	constants
}

/// `x > 1500 m` of `x = 2 km`: the expression with the constants' values in place of their names
fn with_constants(node: &Node, constants: &Constants) -> Node {
	constants.iter().fold(node.clone(), |node, (name, value)| {
		let grouped = Node::List(vec![value.clone()], Bracket::Round, Separator::None);
		crate::lowering::library_words::substitute(node, name, &grouped)
	})
}

fn fold_comparisons(node: Node, constants: &Constants, clock: bool, time_is_the_type: bool) -> Node {
	let Node::Key(left, op, right) = node.drop_meta() else {
		return node.map_children(|child| fold_comparisons(child, constants, clock, time_is_the_type));
	};
	if !op.is_comparison() {
		return node.map_children(|child| fold_comparisons(child, constants, clock, time_is_the_type));
	}
	let is_clock = |side: &Node| clock && side.is_symbol(NOW_WORD);
	let type_test = *op == Op::Eq && !crate::type_tests::is_equality_operand(right);
	if time_is_the_type && (left.is_symbol(TIME_WORD) || right.is_symbol(TIME_WORD) && !type_test) {
		let other = if left.is_symbol(TIME_WORD) { right } else { left };
		if let Some(amount) = milliseconds(&with_constants(other, constants)) {
			return error(&format!(
				"time is the type of times, not the clock: compare now, e.g. now {op} {}",
				written_milliseconds(amount)
			));
		}
	}
	if is_clock(left) || (is_clock(right) && !type_test) {
		let (duration, clock_left) = if is_clock(left) { (right, true) } else { (left, false) };
		return match clock_milliseconds(&with_constants(duration, constants)) {
			Ok(Some(amount)) => {
				let time_of_day = Node::Symbol(format!(
					"{}{}",
					crate::system_values::SYSTEM_PREFIX,
					warp_runtime::host_words::TIME_OF_DAY
				));
				let (left, right) =
					if clock_left { (time_of_day, Node::int(amount)) } else { (Node::int(amount), time_of_day) };
				Node::Key(Box::new(left), *op, Box::new(right))
			}
			Ok(None) => node,
			Err(message) => error(&message),
		};
	}
	let reads_quantities =
		needs_quantities(&node) || constants.keys().any(|name| crate::warp_parser::mentions(&node, name));
	match static_units::constant(&with_constants(&node, constants)) {
		Ok(Some((truth @ (Node::True | Node::False), _))) if reads_quantities => truth,
		Err(message) if reads_quantities => error(&message),
		_ => node.map_children(|child| fold_comparisons(child, constants, clock, time_is_the_type)),
	}
}

/// `18h`, `90min`, `500ms`: an amount of milliseconds in the largest unit it is whole in
fn written_milliseconds(amount: i64) -> String {
	const UNITS: [(i64, &str); 3] = [(3_600_000, "h"), (60_000, "min"), (1000, "s")];
	match UNITS.iter().find(|(size, _)| amount % size == 0) {
		Some((size, unit)) => format!("{}{unit}", amount / size),
		None => format!("{amount}ms"),
	}
}

/// What the time of day is compared with, in milliseconds; None when it is no constant (left as written, loud later)
fn clock_milliseconds(duration: &Node) -> Result<Option<i64>, String> {
	if let Some(amount) = milliseconds(duration) {
		return Ok(Some(amount));
	}
	match static_units::constant(duration) {
		Ok(Some((amount, signature))) if signature.is_empty() => {
			Err(format!("the time of day is a duration: compare it with one, e.g. now < {}h", amount.serialize()))
		}
		Ok(Some(_)) => Err(format!("DimensionError: the time of day is a duration, {} is none", duration.serialize())),
		_ => Ok(None),
	}
}

fn without_unread_constants(program: Node, constants: &Constants) -> Node {
	if constants.is_empty() {
		return program;
	}
	let (statements, bracket, separator) = crate::variable_signals::main_statements(&program);
	let assigns = |statement: &Node, name: &str| matches!(statement.drop_meta(), Node::Key(target, Op::Assign, _) if matches!(target.drop_meta(), Node::Symbol(assigned) if assigned == name));
	let unread = |name: &str| {
		statements.iter().all(|statement| assigns(statement, name) || !crate::warp_parser::mentions(statement, name))
	};
	let last = statements.len().saturating_sub(1);
	let kept: Vec<Node> = statements
		.iter()
		.enumerate()
		.filter(|(index, statement)| {
			*index == last || !constants.keys().any(|name| assigns(statement, name) && unread(name))
		})
		.map(|(_, statement)| statement.clone())
		.collect();
	if kept.len() == statements.len() {
		return program;
	}
	Node::List(kept, bracket, separator)
}

/// User #17: a bare number of `sleep(1)` gets the warning that names the units; it counts seconds (decision
/// sleep-seconds), as in sh and Python; `use relaxed` and `use script` take it without the warning
fn warn_bare_duration(duration: &Node) -> Result<(), Node> {
	if crate::compiler::compile_context::read(|context| context.hints_muted) {
		return Ok(());
	}
	let amount = duration.serialize();
	let message = format!("sleep needs a unit: sleep {amount}s or sleep {amount}ms (a bare number is seconds)");
	crate::errors::diagnostic::report(&[crate::errors::diagnostic::Diagnostic::at(duration, message)])
}

/// A constant duration in whole milliseconds: `1000 ms`, `2 s`, `1 min`, `2 seconds` (a duration of the time module)
pub(crate) fn milliseconds(node: &Node) -> Option<i64> {
	if let Node::Data(data) = node.drop_meta() {
		return duration_milliseconds(data.downcast_ref::<crate::time::Duration>()?, "ms").ok().flatten();
	}
	match static_units::constant(&with_written_amount(node.clone())) {
		Ok(Some((seconds, signature))) if signature == [(Dimension::Time, 1)] => {
			whole(&static_units::exact_amount(&seconds)?.mul(&Rational::integer(MILLISECONDS_PER_SECOND)))
		}
		_ => None,
	}
}

fn needs_quantities(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(name) => unit_named(name).is_some(),
		// `3010 meters`: a long name counts after an amount
		Node::List(items, Bracket::None, _) if matches!(items.as_slice(), [_, unit] if target_unit(unit).is_some()) => {
			true
		}
		Node::Key(_, Op::PlusMinus, _) => true,
		other => other.parts().into_iter().any(needs_quantities),
	}
}

/// The unit words the program defines as variables, parameters or functions (`m = 2`, `f(s) := …`)
fn defined_unit_names(node: &Node) -> std::collections::HashSet<String> {
	let mut names = std::collections::HashSet::new();
	collect_defined_unit_names(node, &mut names);
	names
}

fn collect_defined_unit_names(node: &Node, names: &mut std::collections::HashSet<String>) {
	let mut add_unit = |node: &Node| {
		if let Node::Symbol(name) = node.drop_meta() {
			if units().iter().any(|unit| unit.name == name) || unit_aliases().iter().any(|(alias, _)| alias == name) {
				names.insert(name.clone());
			}
		}
	};
	let mut add_parameters = |head: &Node| match head.drop_meta() {
		Node::List(items, _, _) => items.iter().for_each(|item| match item.drop_meta() {
			Node::Key(name, Op::Colon, _) => add_unit(name),
			_ => add_unit(item),
		}),
		other => add_unit(other),
	};
	if let Some(definition) = crate::lowering::function_head(node) {
		definition.written_parameters.iter().for_each(&mut add_parameters);
	}
	match node.drop_meta() {
		Node::Key(head, Op::Assign | Op::Define, _) if matches!(head.drop_meta(), Node::List(..)) => {
			add_parameters(head)
		}
		Node::Key(target, Op::Assign | Op::Define | Op::Colon, _) => add_parameters(target),
		Node::Key(parameters, Op::Arrow, _) => add_parameters(parameters),
		// `for g in 0 to 2 {…}`: a loop variable named like a unit is the variable (card loop-variable)
		Node::List(words, _, _) => {
			if let Some(variable) = crate::lowering::nodes::for_in_variable(words) {
				add_unit(&Node::Symbol(variable))
			}
		}
		_ => {}
	}
	node.drop_meta().parts().into_iter().for_each(|child| collect_defined_unit_names(child, names));
}

/// Main-level variables holding a constant quantity, and its value: `x = 2 km`
type Constants = HashMap<String, Node>;
