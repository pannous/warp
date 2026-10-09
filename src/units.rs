//! Unit words as values (wiki/unit.md): `3km` is `3*km`, `1 m + 1km` is `1001 m`.
//! Programs made of integers and units are evaluated here at compile time; sums convert to the finer unit.
//! `1950 ± 50` is a value with tolerance, `1900 - 2000 AD` a range; both print back in the form they were written.
//! The canonical unit of a result is the smallest unit involved: `3km+10m` is `3010 m`.
//! A quantity is an amount times units to powers: `6 m²`, `6 m·kg`, `5 km/h` (km·h⁻¹), `1 kg·m/s²`.
//! Anything else (floats, variables, functions) takes the normal path, where a unit word is an ordinary symbol.

use crate::extensions::numbers::Number;
use crate::meta::DataValue;
use crate::node::{error, Bracket, Node, Separator};
use std::collections::HashMap;
use crate::operators::Op;
pub mod static_units;

use std::fmt;
use crate::extensions::reals::Rational;
use num_bigint::BigInt;
use num_traits::ToPrimitive;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Dimension {
	Length,
	Mass,
	Era,
	Time,
}

#[derive(Debug, PartialEq)]
/// `factor` counts the smallest step of the dimension, so every unit is a whole number of it: time in ms, length in
/// 0.1 mm (1 ft = 3048, 1 km = 10_000_000), mass in 10 µg (1 lb = 45_359_237)
struct Unit {
	name: &'static str,
	dimension: Dimension,
	factor: i64,
}

const UNITS: [Unit; 17] = [
	Unit { name: "ms", dimension: Dimension::Time, factor: 1 },
	Unit { name: "s", dimension: Dimension::Time, factor: 1_000 },
	Unit { name: "min", dimension: Dimension::Time, factor: 60_000 },
	Unit { name: "h", dimension: Dimension::Time, factor: 3_600_000 },
	Unit { name: "mm", dimension: Dimension::Length, factor: 10 },
	Unit { name: "cm", dimension: Dimension::Length, factor: 100 },
	Unit { name: "m", dimension: Dimension::Length, factor: 10_000 },
	Unit { name: "km", dimension: Dimension::Length, factor: 10_000_000 },
	// `inch`, not `in`: `x in xs` and `100 cm in m` keep the word
	Unit { name: "inch", dimension: Dimension::Length, factor: 254 },
	Unit { name: "ft", dimension: Dimension::Length, factor: 3_048 },
	Unit { name: "yd", dimension: Dimension::Length, factor: 9_144 },
	Unit { name: "mi", dimension: Dimension::Length, factor: 16_093_440 },
	Unit { name: "mg", dimension: Dimension::Mass, factor: 100 },
	Unit { name: "g", dimension: Dimension::Mass, factor: 100_000 },
	Unit { name: "kg", dimension: Dimension::Mass, factor: 100_000_000 },
	Unit { name: "lb", dimension: Dimension::Mass, factor: 45_359_237 },
	Unit { name: "AD", dimension: Dimension::Era, factor: 1 },
];

/// Unit words standing for a unit expression: `60 mph` is `60 mi/h`
const UNIT_ALIASES: [(&str, &str); 1] = [("mph", "mi/h")];

/// The long names a conversion target may use, singular or plural: `2 h in minutes` (P36). Quantities keep the short
/// names, `2 minutes` stays a duration of the time module.
const LONG_NAMES: [(&str, &str); 19] = [
	("millisecond", "ms"), ("second", "s"), ("minute", "min"), ("hour", "h"),
	("millimeter", "mm"), ("centimeter", "cm"), ("meter", "m"), ("metre", "m"), ("kilometer", "km"),
	("inches", "inch"), ("foot", "ft"), ("feet", "ft"), ("yard", "yd"), ("mile", "mi"),
	("milligram", "mg"), ("gram", "g"), ("kilogram", "kg"), ("pound", "lb"), ("lbs", "lb"),
];
/// `100 cm in m`: the word of a conversion written with spaces (`as` is an operator)
const IN_WORD: &str = "in";
/// `for g in 0 to 2 {…}`, `for each g in …`: a loop variable named like a unit is the variable (card loop-variable)
const FOR_WORD: &str = "for";
const EACH_WORD: &str = "each";
/// The local time of day, unless the program names something so (card time-day-value)
const TIME_WORD: &str = "time";

thread_local! {
	/// The unit words the program being lowered defines itself (`m = 2`, `g(x) := …`): no units while it is lowered
	static SHADOWED_UNITS: std::cell::RefCell<std::collections::HashSet<String>> = std::cell::RefCell::new(Default::default());
}

fn unit_named(name: &str) -> Option<&'static Unit> {
	if SHADOWED_UNITS.with(|shadowed| shadowed.borrow().contains(name)) {
		return None;
	}
	UNITS.iter().find(|unit| unit.name == name)
}

/// `run` with the unit words `program` defines as its names, not units: `g(x) := x + 1 m` keeps m a unit
fn shadowing<T>(program: &Node, run: impl FnOnce() -> T) -> T {
	let outer = SHADOWED_UNITS.with(|shadowed| shadowed.replace(defined_unit_names(program)));
	let result = run();
	SHADOWED_UNITS.with(|shadowed| *shadowed.borrow_mut() = outer);
	result
}

/// A conversion target: a unit by its short name or its long name, `minute` or `minutes`
fn target_unit(node: &Node) -> Option<&'static Unit> {
	let Node::Symbol(name) = node.drop_meta() else { return None };
	let singular = name.strip_suffix('s').unwrap_or(name);
	unit_named(name).or_else(|| LONG_NAMES.iter().find(|(long, _)| *long == singular || *long == name).and_then(|(_, short)| unit_named(short)))
}

/// The unit words as the units passes read them: aliases expanded (`60 mph`), the whole unit after `as` (`q as km/h`)
pub fn lower_unit_words(program: Node) -> Node {
	let program = lower_unit_aliases(program);
	shadowing(&program.clone(), || with_whole_conversion_units(program))
}

/// `60 mi/h as km/h` parses `(60 mi/h as km)/h`: the unit after `as` regrouped whole, as after `in`, when it is one
fn with_whole_conversion_units(node: Node) -> Node {
	let node = match node.map_children(with_whole_conversion_units) {
		Node::Key(conversion, op @ (Op::Div | Op::Mul), rest) => match conversion.drop_meta() {
			Node::Key(quantity, Op::As, unit) if unit_expression(&Node::Key(unit.clone(), op, rest.clone())).is_some() => {
				Node::Key(quantity.clone(), Op::As, Box::new(Node::Key(unit.clone(), op, rest)))
			}
			_ => Node::Key(conversion, op, rest),
		},
		other => other,
	};
	with_converted_last_operand(node)
}

/// `"a: " + x as km` parses `("a: " + x) as km`: a text has no unit to convert, so `as` takes the operand after the
/// last `+` (card print-km)
fn with_converted_last_operand(node: Node) -> Node {
	match node {
		Node::Key(sum, Op::As, unit) if unit_expression(&unit).is_some() => match sum.drop_meta() {
			Node::Key(text, Op::Add, last) if joins_text(text) => Node::Key(text.clone(), Op::Add, Box::new(Node::Key(last.clone(), Op::As, unit))),
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
	let aliases: Vec<(&str, &str)> = UNIT_ALIASES.into_iter().filter(|(alias, _)| mentions_word(&program, alias)).collect();
	if aliases.is_empty() {
		return program;
	}
	let defined = defined_unit_names(&program);
	let aliases: Vec<(&str, &str)> = aliases.into_iter().filter(|(alias, _)| !defined.contains(*alias)).collect();
	with_unit_aliases(program, &aliases)
}

fn with_unit_aliases(node: Node, aliases: &[(&str, &str)]) -> Node {
	let expansion = |unit: &Node| match unit.drop_meta() {
		Node::Symbol(name) => aliases.iter().find(|(alias, _)| alias == name).map(|(_, expression)| crate::warp_parser::parse(expression)),
		_ => None,
	};
	match node {
		// `60 mph`, `60*mph`: the amount joins the first unit, `60 mi / h`, as a written `60 mi/h` reads
		Node::List(items, Bracket::None, Separator::Space) if items.len() == 2 && expansion(&items[1]).is_some() => {
			let [amount, unit] = <[Node; 2]>::try_from(items).expect("two items");
			times_first_unit(with_unit_aliases(amount, aliases), expansion(&unit).expect("guarded"))
		}
		Node::Key(amount, Op::Mul, unit) if expansion(&unit).is_some() => times_first_unit(with_unit_aliases(*amount, aliases), expansion(&unit).expect("guarded")),
		Node::Symbol(name) => expansion(&Node::Symbol(name.clone())).unwrap_or(Node::Symbol(name)),
		// a field's type, `speed: mph`
		Node::Type { name, body } if matches!(body.drop_meta(), Node::Empty) && expansion(&name).is_some() => expansion(&name).expect("guarded"),
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
		other => children(other).into_iter().any(|child| mentions_word(child, word)),
	}
}

fn times_first_unit(amount: Node, expression: Node) -> Node {
	match expression.drop_meta() {
		Node::Key(first, op @ (Op::Mul | Op::Div), rest) => Node::Key(Box::new(times_first_unit(amount, *first.clone())), *op, rest.clone()),
		_ => Node::Key(Box::new(amount), Op::Mul, Box::new(expression)),
	}
}

/// A unit word's base dimension and how many of the dimension's smallest steps it is (`km`: Length, 10_000_000)
pub fn dimension_and_factor(name: &str) -> Option<(String, i64)> {
	unit_named(name).map(|unit| (format!("{:?}", unit.dimension), unit.factor))
}

pub fn is_unit(name: &str) -> bool {
	unit_named(name).is_some()
}

/// A unit or a unit alias: the parser binds it to the amount before it, `60 mph` is `60*mph`
pub fn names_unit(name: &str) -> bool {
	is_unit(name) || UNIT_ALIASES.iter().any(|(alias, _)| *alias == name)
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
	Node::List(vec![Node::Symbol(RUN_TIME_QUANTITY_WORD.to_string()), amount, Node::Text(unit)], Bracket::Round, Separator::None)
}

/// `5 m ± 1 cm` in a program the compile-time evaluation cannot answer (arithmetic with it, a function given it) as the
/// run-time `quantity(5 ± 1/100, "m")`: its amount an interval in the unit of the value (card plus-minus-units). A
/// tolerance the evaluation answers (`2m ± 5cm`, `1950 AD ± 50 == 1900 - 2000 AD`) stays the compile-time Tolerance
pub fn lower_run_time_tolerances(program: Node) -> Node {
	shadowing(&program.clone(), || match !has_unit_tolerance(&program) || answers_at_compile_time(&program) {
		true => program,
		false => with_run_time_tolerances(program),
	})
}

fn has_unit_tolerance(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(value, Op::PlusMinus, spread) if unit_literal(value).is_some() || unit_literal(spread).is_some() => true,
		other => children(other).into_iter().any(has_unit_tolerance),
	}
}

fn answers_at_compile_time(program: &Node) -> bool {
	match evaluate(program) {
		Ok(_) => true,
		Err(Stop::Error(message)) => !RUN_TIME_TOLERANCE_ERRORS.iter().any(|refusal| message.starts_with(refusal)),
		Err(Stop::Unsupported) => false,
	}
}

fn with_run_time_tolerances(node: Node) -> Node {
	match node {
		Node::Key(value, Op::PlusMinus, spread) if unit_literal(&value).is_some() || unit_literal(&spread).is_some() => {
			run_time_tolerance(&value, &spread).unwrap_or_else(|message| error(&message))
		}
		other => other.map_children(with_run_time_tolerances),
	}
}

/// `5 m ± 1 cm` is `quantity(5 ± 1/100, "m")`, `5 m ± 1` counts the 1 in m, `5 ± 1 cm` the 5 in cm
fn run_time_tolerance(value: &Node, spread: &Node) -> Result<Node, String> {
	let interval = |amount: Node, spread: Node| Node::Key(Box::new(amount), Op::PlusMinus, Box::new(spread));
	let (amount, unit) = match (unit_literal(value), unit_literal(spread)) {
		(Some((amount, unit)), Some(_)) => {
			let (Ok(Value::Quantity(value)), Ok(Value::Quantity(spread))) = (evaluate(value), evaluate(spread)) else { return Err(format!("a tolerance applies to numbers and quantities: {}", spread.serialize())) };
			aligned(&value, &spread).map_err(|stop| match stop { Stop::Error(message) => message, Stop::Unsupported => "incompatible units".to_string() })?;
			let unit_of = |factor: &Factor| value.factors.iter().find(|own| own.unit.dimension == factor.unit.dimension).map_or(factor.unit, |own| own.unit);
			(interval(amount, quotient_node(&spread.amount.mul(&scale(&spread.factors, unit_of)))), unit)
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
		Node::Key(amount, Op::Mul, unit) if matches!(amount.drop_meta(), Node::Number(_)) => Some((amount.drop_meta().clone(), unit_text(unit)?)),
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

const SUPERSCRIPT_DIGITS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
/// Between the units of a product: `m·kg`
const UNIT_PRODUCT: &str = "·";

fn superscript(n: u32) -> String {
	n.to_string().chars().filter_map(|digit| digit.to_digit(10)).map(|digit| SUPERSCRIPT_DIGITS[digit as usize]).collect()
}

/// `m·kg/s²`: the units with a positive power, then those with a negative one after a slash
fn units_text(factors: &[Factor]) -> String {
	let side = |positive: bool| {
		let units = factors.iter().filter(|factor| (factor.power > 0) == positive).map(|factor| match factor.power.unsigned_abs() {
			1 => factor.unit.name.to_string(),
			power => format!("{}{}", factor.unit.name, superscript(power)),
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
	amount: crate::uncertain::Uncertain,
	factors: Vec<Factor>,
}

impl UncertainQuantity {
	pub(crate) fn new(amount: crate::uncertain::Uncertain, factors: &[Factor]) -> UncertainQuantity {
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
	fn of(amount: i64, unit: &'static Unit) -> Quantity {
		Quantity { amount: Rational::integer(amount), factors: vec![Factor { unit, power: 1 }] }
	}

	/// Is the amount the whole number n: a quantity equals the bare number of its own unit (`3km+10m == 3010`)
	pub fn is_amount(&self, n: i64) -> bool {
		self.amount == Rational::integer(n)
	}

	fn with_amount(&self, amount: Rational) -> Quantity {
		Quantity { amount, factors: self.factors.clone() }
	}

	/// The one unit of a plain quantity (`3 m`), None for a power or a product
	fn single_unit(&self) -> Option<&'static Unit> {
		match self.factors.as_slice() {
			[Factor { unit, power: 1 }] => Some(unit),
			_ => None,
		}
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

/// `1950 ± 50 cm`: the unit is absent for plain numbers
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tolerance {
	value: i64,
	tolerance: i64,
	unit: Option<&'static Unit>,
}

impl fmt::Display for Tolerance {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{} ± {}", self.value, self.tolerance)?;
		self.unit.map_or(Ok(()), |unit| write!(f, "{}", unit.name))
	}
}

/// `1900 - 2000 AD`: both ends in one unit
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Range {
	from: i64,
	to: i64,
	unit: &'static Unit,
}

impl fmt::Display for Range {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{} - {}{}", self.from, self.to, self.unit.name)
	}
}

/// The text of a unit result held in a `Node::Data`, None for any other data
pub fn describe(data: &DataValue) -> Option<String> {
	if let Some(quantity) = data.downcast_ref::<Quantity>() {
		return Some(quantity.to_string());
	}
	if let Some(tolerance) = data.downcast_ref::<Tolerance>() {
		return Some(tolerance.to_string());
	}
	if let Some(uncertain) = data.downcast_ref::<crate::uncertain::Uncertain>() {
		return Some(uncertain.to_string());
	}
	if let Some(uncertain) = data.downcast_ref::<UncertainQuantity>() {
		return Some(uncertain.to_string());
	}
	if let Some(duration) = data.downcast_ref::<crate::time::Duration>() {
		return Some(duration.to_string());
	}
	if let Some(time) = data.downcast_ref::<crate::time::Time>() {
		return Some(time.to_string());
	}
	data.downcast_ref::<Range>().map(|range| range.to_string())
}

#[derive(Clone, Debug)]
enum Value {
	Number(i64),
	Quantity(Quantity),
	Tolerance(Tolerance),
	Range(Range),
}

enum Stop {
	/// not a program of integers and units: compile it normally
	Unsupported,
	Error(String),
}

type Evaluated = Result<Value, Stop>;

/// The refusals of the compile-time evaluation that a run-time quantity answers (lower_run_time_tolerances)
const TOLERANCE_ARITHMETIC: &str = "arithmetic on a value with tolerance or a range is not supported";
const WHOLE_TOLERANCE: &str = "a tolerance counts whole numbers";
const PLAIN_TOLERANCE: &str = "a tolerance applies to numbers and plain quantities";
const RUN_TIME_TOLERANCE_ERRORS: [&str; 3] = [TOLERANCE_ARITHMETIC, WHOLE_TOLERANCE, PLAIN_TOLERANCE];

fn fail<T>(message: impl Into<String>) -> Result<T, Stop> {
	Err(Stop::Error(message.into()))
}

/// The value of a program that uses units, None for any other program
pub fn answer(program: &Node) -> Option<Node> {
	shadowing(program, || answer_shadowed(program))
}

fn answer_shadowed(program: &Node) -> Option<Node> {
	if !needs_quantities(program) {
		return None;
	}
	match evaluate(program) {
		Ok(Value::Number(n)) => Some(Node::int(n)),
		// a ratio of two quantities of one dimension that is no whole number: `1 m / 3 m` is 1/3
		Ok(Value::Quantity(quantity)) if quantity.factors.is_empty() => Some(quotient_node(&quantity.amount)),
		Ok(Value::Quantity(quantity)) => Some(Node::data(quantity)),
		Ok(Value::Tolerance(tolerance)) => Some(Node::data(tolerance)),
		Ok(Value::Range(range)) => Some(Node::data(range)),
		Err(Stop::Error(message)) => Some(error(&message)),
		Err(Stop::Unsupported) => None,
	}
}

/// `sleep(1000 ms)`, `sleep 1 s`, `sleep(2 seconds)`: a constant duration is its milliseconds, what the host word takes
/// (async agent, 2026-10-06: quantities do not reach run time yet, notes/units_runtime.md)
pub fn lower_sleep_durations(program: Node) -> Node {
	// a duration whose unit word is one of the program's names (`s = 1; sleep 5 s`) stays as written; `m = 2` leaves
	// `sleep 5 ms` a duration (card sleep-unit)
	fn lower(node: Node, shadowed: &std::collections::HashSet<String>) -> Node {
		match node {
			// the word itself: `(sleep 300); 1` is a statement list whose first item merely starts with sleep
			Node::List(items, bracket, separator) if items.len() >= 2 && matches!(items[0].drop_meta(), Node::Symbol(word) if word == crate::host::SLEEP) => {
				let duration = match &items[1..] {
					[single] => single.clone(),
					several => Node::List(several.to_vec(), Bracket::None, Separator::Space),
				};
				let names_shadowed = |node: &Node| {
					let mut found = false;
					node.visit(&mut |part| found |= matches!(part, Node::Symbol(name) if shadowed.contains(name)));
					found
				};
				match milliseconds(&duration).filter(|_| !names_shadowed(&duration)) {
					Some(amount) => Node::List(vec![items[0].clone(), Node::int(amount)], Bracket::Round, Separator::None),
					None => {
						if let Err(error) = warn_bare_duration(&duration) {
							return error;
						}
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
/// local time of day, compared with a constant duration in milliseconds: `time < 24h` is
/// `system·time of day < 86400000` (lowering/system_values.rs reads it, so `whenever time > 18h {…}` listens too)
pub fn lower_quantity_comparisons(program: Node) -> Node {
	shadowing(&program.clone(), || quantity_comparisons_folded(program))
}

fn quantity_comparisons_folded(program: Node) -> Node {
	if !(needs_quantities(&program) || mentions_clock(&program)) {
		return program;
	}
	let bound = crate::system_values::bound_names(&program);
	let constants = constant_quantities(&program);
	let clock = !bound.contains(TIME_WORD);
	let folded = fold_comparisons(program, &constants, clock);
	without_unread_constants(folded, &constants)
}

fn mentions_clock(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= matches!(part, Node::Symbol(name) if name == TIME_WORD));
	found
}

/// The main-level variables assigned a quantity once, and nowhere else: `x = 2 km`
fn constant_quantities(program: &Node) -> Variables {
	let mut assignments: HashMap<String, usize> = HashMap::new();
	program.visit(&mut |part| if let Node::Key(target, Op::Assign | Op::Define | Op::Arrow | Op::FatArrow, _) | Node::Key(target, Op::AddAssign | Op::SubAssign | Op::MulAssign | Op::DivAssign, _) = part {
		crate::variable_signals::symbols(target).into_iter().for_each(|name| *assignments.entry(name).or_default() += 1);
	});
	let mut constants = Variables::new();
	for statement in crate::variable_signals::main_statements(program).0 {
		let Node::Key(target, Op::Assign, value) = statement.drop_meta() else { continue };
		let Node::Symbol(name) = target.drop_meta() else { continue };
		if assignments.get(name) == Some(&1) && needs_quantities(value) {
			if let Ok(quantity @ Value::Quantity(_)) = evaluate_in(value, &mut constants.clone()) {
				constants.insert(name.clone(), quantity);
			}
		}
	}
	constants
}

fn fold_comparisons(node: Node, constants: &Variables, clock: bool) -> Node {
	let Node::Key(left, op, right) = node.drop_meta() else { return node.map_children(|child| fold_comparisons(child, constants, clock)) };
	if !op.is_comparison() {
		return node.map_children(|child| fold_comparisons(child, constants, clock));
	}
	let is_clock = |side: &Node| clock && matches!(side.drop_meta(), Node::Symbol(name) if name == TIME_WORD);
	if is_clock(left) || is_clock(right) {
		let (duration, clock_left) = if is_clock(left) { (right, true) } else { (left, false) };
		return match clock_milliseconds(duration, constants) {
			Ok(Some(amount)) => {
				let time_of_day = Node::Symbol(format!("{}{}", crate::system_values::SYSTEM_PREFIX, warp_runtime::host_words::TIME_OF_DAY));
				let (left, right) = if clock_left { (time_of_day, Node::int(amount)) } else { (Node::int(amount), time_of_day) };
				Node::Key(Box::new(left), *op, Box::new(right))
			}
			Ok(None) => node,
			Err(message) => error(&message),
		};
	}
	let reads_quantities = needs_quantities(&node) || constants.keys().any(|name| crate::warp_parser::mentions(&node, name));
	match evaluate_in(&node, &mut constants.clone()).ok().filter(|_| reads_quantities) {
		Some(Value::Number(answer)) => Node::int(answer),
		_ => match evaluate_in(&node, &mut constants.clone()) {
			Err(Stop::Error(message)) if reads_quantities => error(&message),
			_ => node.map_children(|child| fold_comparisons(child, constants, clock)),
		},
	}
}

/// What the time of day is compared with, in milliseconds; None when it is no constant (left as written, loud later)
fn clock_milliseconds(duration: &Node, constants: &Variables) -> Result<Option<i64>, String> {
	if let Some(amount) = milliseconds(duration) {
		return Ok(Some(amount));
	}
	match evaluate_in(duration, &mut constants.clone()) {
		Ok(Value::Quantity(quantity)) => match quantity.factors.as_slice() {
			[factor] if factor.unit.dimension == Dimension::Time && factor.power == 1 => Ok(whole(&quantity.amount.mul(&Rational::integer(factor.unit.factor)))),
			_ => Err(format!("DimensionError: the time of day is a duration, {quantity} is none")),
		},
		Ok(Value::Number(amount)) => Err(format!("the time of day is a duration: compare it with one, e.g. time < {amount}h")),
		_ => Ok(None),
	}
}

/// The program without the assignments of constant quantities no other statement reads (all compared away); the last
/// statement stays, it is the program's value
fn without_unread_constants(program: Node, constants: &Variables) -> Node {
	if constants.is_empty() {
		return program;
	}
	let (statements, bracket, separator) = crate::variable_signals::main_statements(&program);
	let assigns = |statement: &Node, name: &str| matches!(statement.drop_meta(), Node::Key(target, Op::Assign, _) if matches!(target.drop_meta(), Node::Symbol(assigned) if assigned == name));
	let unread = |name: &str| statements.iter().all(|statement| assigns(statement, name) || !crate::warp_parser::mentions(statement, name));
	let last = statements.len().saturating_sub(1);
	let kept: Vec<Node> = statements.iter().enumerate()
		.filter(|(index, statement)| *index == last || !constants.keys().any(|name| assigns(statement, name) && unread(name)))
		.map(|(_, statement)| statement.clone()).collect();
	if kept.len() == statements.len() {
		return program;
	}
	Node::List(kept, bracket, separator)
}

/// User #17: `sleep(1)` reads as milliseconds but says nothing; a bare number gets the warning that names the units
fn warn_bare_duration(duration: &Node) -> Result<(), Node> {
	if !matches!(duration.drop_meta(), Node::Number(_)) {
		return Ok(());
	}
	let amount = duration.serialize();
	let message = format!("sleep needs a unit: sleep {amount} second or sleep {amount} ms (a bare number is milliseconds)");
	crate::diagnostic::report(&[crate::diagnostic::Diagnostic::at(duration, message)])
}

/// A constant duration in whole milliseconds: `1000 ms`, `2 s`, `1 min`, `2 seconds` (a duration of the time module)
pub(crate) fn milliseconds(node: &Node) -> Option<i64> {
	if let Node::Data(data) = node.drop_meta() {
		return duration_milliseconds(data.downcast_ref::<crate::time::Duration>()?, "ms").ok().flatten();
	}
	let Ok(Value::Quantity(quantity)) = evaluate(node) else { return None };
	let [factor] = quantity.factors.as_slice() else { return None };
	if factor.unit.dimension != Dimension::Time || factor.power != 1 {
		return None;
	}
	whole(&quantity.amount.mul(&Rational::integer(factor.unit.factor)))
}

fn children(node: &Node) -> Vec<&Node> {
	match node.drop_meta() {
		Node::Key(left, _, right) => vec![left, right],
		Node::List(items, _, _) => items.iter().collect(),
		_ => vec![],
	}
}

fn needs_quantities(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(name) => unit_named(name).is_some(),
		// `3010 meters`: a long name counts after an amount
		Node::List(items, Bracket::None, _) if matches!(items.as_slice(), [_, unit] if target_unit(unit).is_some()) => true,
		Node::Key(_, Op::PlusMinus, _) => true,
		other => children(other).into_iter().any(needs_quantities),
	}
}

/// The unit words the program defines as variables, parameters or functions (`m = 2`, `f(s) := …`)
fn defined_unit_names(node: &Node) -> std::collections::HashSet<String> {
	let mut names = std::collections::HashSet::new();
	collect_defined_unit_names(node, &mut names);
	names
}

/// The variable a loop binds, `g` of `for g in 0 to 2 {…}` and of `for each g in …`
fn loop_variable(words: &[Node]) -> Option<&Node> {
	let word = |index: usize| words.get(index).map(|word| word.drop_meta().name());
	match (word(0)?.as_str(), word(1)?.as_str()) {
		(FOR_WORD, EACH_WORD) => words.get(2),
		(FOR_WORD, _) => words.get(1),
		_ => None,
	}
}

fn collect_defined_unit_names(node: &Node, names: &mut std::collections::HashSet<String>) {
	let mut add_unit = |node: &Node| if let Node::Symbol(name) = node.drop_meta() {
		if UNITS.iter().any(|unit| unit.name == name) || UNIT_ALIASES.iter().any(|(alias, _)| alias == name) {
			names.insert(name.clone());
		}
	};
	let mut add_parameters = |head: &Node| match head.drop_meta() {
		Node::List(items, _, _) => items.iter().for_each(|item| match item.drop_meta() {
			Node::Key(name, Op::Colon, _) => add_unit(name),
			_ => add_unit(item),
		}),
		other => add_unit(other),
	};
	match node.drop_meta() {
		Node::Key(head, Op::Assign | Op::Define, _) if matches!(head.drop_meta(), Node::List(..)) => add_parameters(head),
		Node::Key(target, Op::Assign | Op::Define | Op::Colon, _) => add_parameters(target),
		Node::Key(parameters, Op::Arrow | Op::FatArrow, _) => add_parameters(parameters),
		Node::List(words, _, _) => if let Some(variable) = loop_variable(words) { add_unit(variable) },
		_ => {}
	}
	children(node).into_iter().for_each(|child| collect_defined_unit_names(child, names));
}

/// `f(s) := s * 2; f(3 s)`: a parameter named like a unit the program also writes outside the function is a name clash
/// (card static-units-parameter); the parameter shadows the unit, so `3 s` would be an undefined variable
pub fn check_unit_parameter_clashes(program: &Node) -> Option<crate::diagnostic::Diagnostic> {
	let mut parameter_of = HashMap::new();
	program.visit(&mut |node| if let Some((function, parameters)) = function_head(node) {
		for parameter in parameters.iter().map(Node::name).filter(|name| is_unit(name)) {
			parameter_of.insert(parameter, function.name());
		}
	});
	let outside = without_function_definitions(program.clone());
	let names_outside = defined_unit_names(&outside);
	parameter_of.retain(|parameter, _| !names_outside.contains(parameter));
	let used = first_word_of(&outside, &parameter_of)?;
	let name = used.drop_meta().name();
	Some(crate::diagnostic::Diagnostic::at(used, format!("{name} is a parameter of {} and a unit here: rename the parameter", parameter_of[&name])))
}

/// The name and parameters of a function definition `f(a, b) := …`
fn function_head(node: &Node) -> Option<(&Node, &[Node])> {
	let Node::Key(head, Op::Define | Op::Assign, _) = node.drop_meta() else { return None };
	let Node::List(items, Bracket::Round, _) = head.drop_meta() else { return None };
	items.split_first().map(|(function, parameters)| (function, parameters))
}

fn without_function_definitions(node: Node) -> Node {
	match function_head(&node) {
		Some(_) => Node::Empty,
		None => node.map_children(without_function_definitions),
	}
}

/// The first of `words` the code writes, not as a field name (`q.s`)
fn first_word_of<'a>(node: &'a Node, words: &HashMap<String, String>) -> Option<&'a Node> {
	match node.drop_meta() {
		Node::Symbol(name) if words.contains_key(name) => Some(node),
		Node::Key(object, Op::Dot, _) => first_word_of(object, words),
		_ => children(node).into_iter().find_map(|child| first_word_of(child, words)),
	}
}

/// Quantities assigned to variables earlier in the program: `x = 2 km; x + 1 m`
type Variables = HashMap<String, Value>;

fn evaluate(node: &Node) -> Evaluated {
	evaluate_in(node, &mut Variables::new())
}

fn evaluate_in(node: &Node, variables: &mut Variables) -> Evaluated {
	match node.drop_meta() {
		Node::Number(Number::Int(n)) => Ok(Value::Number(*n)),
		Node::Symbol(name) => match variables.get(name) {
			Some(value) => Ok(value.clone()),
			None => unit_named(name).map(|unit| Value::Quantity(Quantity::of(1, unit))).ok_or(Stop::Unsupported),
		},
		Node::Key(target, Op::Assign | Op::Define, value) => {
			let Node::Symbol(name) = target.drop_meta() else { return Err(Stop::Unsupported) };
			let value = evaluate_in(value, variables)?;
			variables.insert(name.clone(), value.clone());
			Ok(value)
		}
		Node::Key(left, Op::Neg, right) if matches!(left.as_ref(), Node::Empty) => negate(evaluate_in(right, variables)?),
		// `6 m² as cm²` reads `(6 m² as cm)²`: the power belongs to the target unit
		Node::Key(conversion, op @ (Op::Square | Op::Cube), nothing) if matches!(nothing.drop_meta(), Node::Empty) && powered_conversion(conversion, *op).is_some() => {
			let (quantity, target) = powered_conversion(conversion, *op).expect("guarded");
			convert(evaluate_in(quantity, variables)?, target)
		}
		Node::Key(base, op @ (Op::Square | Op::Cube), nothing) if matches!(nothing.drop_meta(), Node::Empty) => {
			let exponent = if *op == Op::Square { 2 } else { 3 };
			arithmetic(evaluate_in(base, variables)?, Op::Pow, Value::Number(exponent))
		}
		Node::Key(quantity, Op::As, unit) if unit_expression(unit).is_some() => convert(evaluate_in(quantity, variables)?, unit_expression(unit).expect("guarded")),
		Node::Key(amount, Op::Mul, unit) if is_unit_word(unit) => amount_times(amount, evaluate_in(unit, variables)?, variables),
		Node::Key(left, op, right) => arithmetic(evaluate_in(left, variables)?, *op, evaluate_in(right, variables)?),
		Node::List(items, Bracket::Round, _) if items.len() == 1 => evaluate_in(&items[0], variables),
		Node::List(items, Bracket::None, Separator::Semicolon | Separator::Newline) if items.len() > 1 => {
			items.iter().try_fold(Value::Number(0), |_, item| evaluate_in(item, variables))
		}
		Node::List(items, Bracket::None, _) => match items.as_slice() {
			[single] => evaluate_in(single, variables),
			[count, unit] if is_unit_word(unit) => amount_times(count, evaluate_in(unit, variables)?, variables),
			// `3010 meters`, and a unit after an expression belongs to its last amount: `3km+10m == 3010 meters`
			[amount, unit] if target_unit(unit).is_some() => match amount.drop_meta() {
				Node::Key(..) => evaluate_in(&with_unit(amount, unit), variables),
				_ => amount_times(amount, Value::Quantity(Quantity::of(1, target_unit(unit).expect("guarded"))), variables),
			},
			// `(23/18)m/s`, as a fraction amount shows: the amount in parentheses, then compound units
			[amount, unit] if matches!(amount.drop_meta(), Node::List(_, Bracket::Round, _)) && unit_expression(unit).is_some() => {
				let factors = unit_expression(unit).expect("guarded");
				match written_fraction(amount) {
					Some(fraction) => Ok(Value::Quantity(Quantity { amount: fraction, factors })),
					None => amount_times(amount, Value::Quantity(Quantity { amount: Rational::integer(1), factors }), variables),
				}
			}
			[quantity, word, unit] if matches!(word.drop_meta(), Node::Symbol(w) if w == IN_WORD) && unit_expression(unit).is_some() => {
				convert(evaluate_in(quantity, variables)?, unit_expression(unit).expect("guarded"))
			}
			_ => Err(Stop::Unsupported),
		},
		_ => Err(Stop::Unsupported),
	}
}

/// `amount unit`: a decimal amount is exact (`0.3 s` is 3/10 s, card fractional-durations), any other computes as usual
fn amount_times(amount: &Node, unit: Value, variables: &mut Variables) -> Evaluated {
	let decimal = match amount.drop_meta() {
		Node::Number(Number::Float(value)) => Rational::of_decimal(*value),
		_ => None,
	};
	match (decimal, unit) {
		(Some(exact), Value::Quantity(quantity)) => Ok(Value::Quantity(quantity.with_amount(quantity.amount.mul(&exact)))),
		(_, unit) => arithmetic(evaluate_in(amount, variables)?, Op::Mul, unit),
	}
}

/// `(23/18)`, the exact fraction a quantity shows as its amount
fn written_fraction(amount: &Node) -> Option<Rational> {
	let Node::List(items, Bracket::Round, _) = amount.drop_meta() else { return None };
	let [Node::Key(numerator, Op::Div, denominator)] = items.as_slice().iter().map(Node::drop_meta).collect::<Vec<_>>()[..] else { return None };
	match (numerator.drop_meta(), denominator.drop_meta()) {
		(Node::Number(Number::Int(n)), Node::Number(Number::Int(d))) if *d != 0 => Some(Rational::new(BigInt::from(*n), BigInt::from(*d))),
		_ => None,
	}
}

/// `q as cm` under a power: the quantity and the powered target units
fn powered_conversion(conversion: &Node, op: Op) -> Option<(&Node, Vec<Factor>)> {
	let Node::Key(quantity, Op::As, unit) = conversion.drop_meta() else { return None };
	let exponent = if op == Op::Square { 2 } else { 3 };
	let target = unit_expression(unit)?.into_iter().map(|factor| Factor { power: factor.power * exponent, ..factor }).collect();
	Some((quantity, target))
}

/// A conversion target written as units: `m`, `minutes`, `cm²`, `km/h`, `kg*m/s²`
fn unit_expression(node: &Node) -> Option<Vec<Factor>> {
	match node.drop_meta() {
		Node::Key(base, op @ (Op::Square | Op::Cube), nothing) if matches!(nothing.drop_meta(), Node::Empty) => {
			let exponent = if *op == Op::Square { 2 } else { 3 };
			Some(unit_expression(base)?.into_iter().map(|factor| Factor { power: factor.power * exponent, ..factor }).collect())
		}
		Node::Key(left, Op::Mul, right) => Some([unit_expression(left)?, unit_expression(right)?].concat()),
		Node::Key(left, Op::Div, right) => Some([unit_expression(left)?, inverse(&unit_expression(right)?)].concat()),
		Node::List(items, Bracket::Round, _) if items.len() == 1 => unit_expression(&items[0]),
		other => target_unit(other).map(|unit| vec![Factor { unit, power: 1 }]),
	}
}

/// `q as km/h`, `q in km/h`: the quantity converted and the units it converts to
pub(crate) fn conversion(node: &Node) -> Option<(&Node, Vec<Factor>)> {
	match node.drop_meta() {
		Node::Key(quantity, Op::As, unit) => Some((quantity, unit_expression(unit)?)),
		Node::List(items, Bracket::None, Separator::Space) => match items.as_slice() {
			[quantity, word, unit] if matches!(word.drop_meta(), Node::Symbol(w) if w == IN_WORD) => Some((quantity, unit_expression(unit)?)),
			_ => None,
		},
		_ => None,
	}
}

fn inverse(factors: &[Factor]) -> Vec<Factor> {
	factors.iter().map(|factor| Factor { power: -factor.power, ..*factor }).collect()
}


/// The unit given to the last amount of an expression: `a == 3010` with `meters` is `a == (3010 meters)`
fn with_unit(expression: &Node, unit: &Node) -> Node {
	match expression.drop_meta() {
		Node::Key(left, op, right) => Node::Key(left.clone(), *op, Box::new(with_unit(right, unit))),
		amount => Node::List(vec![amount.clone(), unit.clone()], Bracket::None, Separator::Space),
	}
}

fn is_unit_word(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(name) if unit_named(name).is_some())
}

fn negate(value: Value) -> Evaluated {
	match value {
		Value::Number(n) => Ok(Value::Number(-n)),
		Value::Quantity(quantity) => Ok(Value::Quantity(quantity.with_amount(quantity.amount.neg()))),
		Value::Tolerance(_) | Value::Range(_) => fail("cannot negate a value with tolerance or a range"),
	}
}

fn arithmetic(left: Value, op: Op, right: Value) -> Evaluated {
	match (left, op, right) {
		(left @ (Value::Tolerance(_) | Value::Range(_)), Op::Eq | Op::Ne, right @ (Value::Tolerance(_) | Value::Range(_))) => same_span(left, op, right),
		// `(5 ± 1) * 2` without units: an uncertain value, propagated at run time (wasm_emitter/uncertain.rs)
		(left, Op::Add | Op::Sub | Op::Mul | Op::Div, right) if [&left, &right].iter().all(|value| unitless_number(value))
			&& [&left, &right].iter().any(|value| matches!(value, Value::Tolerance(_))) => Err(Stop::Unsupported),
		(Value::Tolerance(_) | Value::Range(_), _, _) | (_, _, Value::Tolerance(_) | Value::Range(_)) => {
			fail(format!("{TOLERANCE_ARITHMETIC}: {op}"))
		}
		(left, Op::PlusMinus, right) => tolerance(left, right),
		(Value::Number(from), Op::Sub, Value::Quantity(end)) if end.single_unit().is_some() => range(from, end),
		(Value::Number(a), Op::Add | Op::Sub | Op::Mul, Value::Number(b)) => {
			let result = match op {
				Op::Add => a.checked_add(b),
				Op::Sub => a.checked_sub(b),
				_ => a.checked_mul(b),
			};
			result.map(Value::Number).ok_or(Stop::Unsupported)
		}
		(Value::Quantity(a), Op::Add | Op::Sub, Value::Quantity(b)) => sum(a, op, b),
		(Value::Quantity(a), Op::Eq | Op::Ne | Op::Lt | Op::Gt | Op::Le | Op::Ge, Value::Quantity(b)) => compare(a, op, b),
		(Value::Quantity(a), Op::Mul, Value::Quantity(b)) => {
			combine(a.amount.mul(&b.amount), [a.factors.clone(), b.factors.clone()].concat())
		}
		(Value::Quantity(a), Op::Div, Value::Quantity(b)) if !b.amount.is_zero() => {
			combine(a.amount.mul(&b.amount.inverse().expect("not zero")), [a.factors.clone(), inverse(&b.factors)].concat())
		}
		(Value::Quantity(_), Op::Div, Value::Quantity(_)) => fail("division by zero"),
		(Value::Quantity(q), Op::Pow, Value::Number(n)) => power(q, n),
		(Value::Number(_), Op::Add, Value::Quantity(other)) | (Value::Quantity(other), Op::Add | Op::Sub, Value::Number(_)) => {
			fail(format!("incompatible operands: a plain number and {}", units_text(&other.factors)))
		}
		(Value::Number(n), Op::Mul, Value::Quantity(q)) | (Value::Quantity(q), Op::Mul, Value::Number(n)) => {
			Ok(Value::Quantity(q.with_amount(q.amount.mul(&Rational::integer(n)))))
		}
		(Value::Quantity(q), Op::Div, Value::Number(n)) if n != 0 => {
			Ok(Value::Quantity(q.with_amount(q.amount.mul(&Rational::new(BigInt::from(1), BigInt::from(n))))))
		}
		_ => Err(Stop::Unsupported),
	}
}

/// `150 cm in m` is `3/2 m`, `6 m² in cm²` is `60000 cm²`, `36 km/h in m/s` is `10 m/s`; other dimensions: DimensionError
fn convert(value: Value, target: Vec<Factor>) -> Evaluated {
	let Value::Quantity(quantity) = value else { return Err(Stop::Unsupported) };
	if signature(&quantity.factors) != signature(&target) {
		return fail(format!("DimensionError: {quantity} cannot be converted to {}", units_text(&target)));
	}
	let in_target = |factor: &Factor| target.iter().find(|wanted| wanted.unit.dimension == factor.unit.dimension).map_or(factor.unit, |wanted| wanted.unit);
	Ok(Value::Quantity(Quantity { amount: quantity.amount.mul(&scale(&quantity.factors, in_target)), factors: target }))
}


/// Milliseconds in a day, a duration's `days` counted at their nominal length
const DAY_MILLISECONDS: i128 = 86_400_000;
const NANOS_PER_MILLISECOND: i128 = 1_000_000;

/// A duration of the time module in a time unit: `2 hours in minutes` is `120 min` (P36). None when the word names no
/// time unit; an error for months (no fixed length) or a part of a millisecond
pub fn duration_in(duration: &crate::time::Duration, unit_word: &str) -> Option<Node> {
	let target = target_unit(&Node::Symbol(unit_word.to_string()))?;
	if target.dimension != Dimension::Time {
		return Some(error(&format!("DimensionError: {duration} cannot be converted to {}", target.name)));
	}
	let milliseconds = match duration_milliseconds(duration, target.name) {
		Ok(milliseconds) => milliseconds?,
		Err(message) => return Some(error(&message)),
	};
	let quantity = Quantity::of(milliseconds, unit_named("ms").expect("a unit"));
	Some(match convert(Value::Quantity(quantity), vec![Factor { unit: target, power: 1 }]) {
		Ok(Value::Quantity(quantity)) => Node::data(quantity),
		Err(Stop::Error(message)) => error(&message),
		_ => return None,
	})
}

/// A duration of the time module in whole milliseconds; an error for months (no fixed length in `target`) or a part of
/// a millisecond, None beyond the 64-bit integers
fn duration_milliseconds(duration: &crate::time::Duration, target: &str) -> Result<Option<i64>, String> {
	if duration.months != 0 {
		return Err(format!("DimensionError: {duration} has no fixed length in {target}"));
	}
	let nanos = duration.nanos + duration.days as i128 * DAY_MILLISECONDS * NANOS_PER_MILLISECOND;
	if nanos % NANOS_PER_MILLISECOND != 0 {
		return Err(format!("{duration} is no whole number of milliseconds"));
	}
	Ok(i64::try_from(nanos / NANOS_PER_MILLISECOND).ok())
}

/// The amount of a quantity in other units, as a factor: Π (unit/target)^power over its factors (1 km/h in m/s is 5/18)
fn scale(factors: &[Factor], target: impl Fn(&Factor) -> &'static Unit) -> Rational {
	factors.iter().fold(Rational::integer(1), |product, factor| {
		let ratio = Rational::new(BigInt::from(factor.unit.factor), BigInt::from(target(factor).factor));
		let ratio = if factor.power > 0 { ratio } else { ratio.inverse().expect("unit factors are positive") };
		(0..factor.power.unsigned_abs()).fold(product, |product, _| product.mul(&ratio))
	})
}

/// `1 m / 3 m` is the number 1/3
fn quotient_node(amount: &Rational) -> Node {
	Node::Number(Number::from_rational(amount.clone()))
}

fn whole(amount: &Rational) -> Option<i64> {
	amount.is_integer().then(|| amount.numerator.to_i64()).flatten()
}

/// The finest unit of each dimension among the factors: the unit a result counts in ("unit sums use the finer unit")
fn finest_units(factors: &[Factor]) -> Vec<&'static Unit> {
	let mut finest: Vec<&'static Unit> = vec![];
	for factor in factors {
		match finest.iter_mut().find(|unit| unit.dimension == factor.unit.dimension) {
			Some(unit) if factor.unit.factor < unit.factor => *unit = factor.unit,
			Some(_) => {}
			None => finest.push(factor.unit),
		}
	}
	finest
}

fn finest_of<'a>(finest: &'a [&'static Unit]) -> impl Fn(&Factor) -> &'static Unit + 'a {
	move |factor| finest.iter().find(|unit| unit.dimension == factor.unit.dimension).copied().unwrap_or(factor.unit)
}

/// An amount times the factors as one quantity: each dimension counts in its finest unit, its powers add
/// (`3 m * 2 m` is `6 m²`, `2 m * 3 kg` is `6 m·kg`, `10 km/h * 30 min` is `5 km`); no unit left is a plain number
fn combine(amount: Rational, factors: Vec<Factor>) -> Evaluated {
	let finest = finest_units(&factors);
	let result: Vec<Factor> = signature(&factors).into_iter()
		.map(|(dimension, power)| Factor { unit: finest.iter().find(|unit| unit.dimension == dimension).copied().expect("a unit per dimension"), power })
		.collect();
	let result = order_of_appearance(result, &factors);
	let amount = amount.mul(&scale(&factors, finest_of(&finest)));
	Ok(match whole(&amount) {
		Some(number) if result.is_empty() => Value::Number(number),
		_ => Value::Quantity(Quantity { amount, factors: result }),
	})
}

/// The factors in the order their dimensions were first written: `2 m * 3 kg` is `m·kg`
fn order_of_appearance(mut result: Vec<Factor>, written: &[Factor]) -> Vec<Factor> {
	let first = |dimension: Dimension| written.iter().position(|factor| factor.unit.dimension == dimension).unwrap_or(usize::MAX);
	result.sort_by_key(|factor| first(factor.unit.dimension));
	result
}

/// `(2 km)²` is `4 km²`
fn power(base: Quantity, exponent: i64) -> Evaluated {
	let exponent = u32::try_from(exponent).ok().filter(|exponent| *exponent >= 1).ok_or(Stop::Unsupported)?;
	let amount = (1..exponent).fold(base.amount.clone(), |amount, _| amount.mul(&base.amount));
	let factors = base.factors.iter().map(|factor| Factor { power: factor.power * exponent as i32, ..*factor }).collect();
	Ok(Value::Quantity(Quantity { amount, factors }))
}

/// Both amounts counted in the finer unit of each dimension: `m²` and `cm²` count in cm², `m²` and `m` do not mix
fn aligned(left: &Quantity, right: &Quantity) -> Result<(Rational, Rational, Vec<Factor>), Stop> {
	if signature(&left.factors) != signature(&right.factors) {
		return fail(format!("DimensionError: incompatible units: {} and {}", units_text(&left.factors), units_text(&right.factors)));
	}
	let finest = finest_units(&[left.factors.clone(), right.factors.clone()].concat());
	let target = finest_of(&finest);
	let factors: Vec<Factor> = left.factors.iter().map(|factor| Factor { unit: target(factor), ..*factor }).collect();
	let amount = |quantity: &Quantity| quantity.amount.mul(&scale(&quantity.factors, &target));
	Ok((amount(left), amount(right), factors))
}

/// Comparison of two quantities in their finer units: `3km == 3000m`, answered 1 or 0
fn compare(left: Quantity, op: Op, right: Quantity) -> Evaluated {
	let (x, y, _) = aligned(&left, &right)?;
	let order = x.compare(&y);
	let holds = match op {
		Op::Eq => order.is_eq(),
		Op::Ne => order.is_ne(),
		Op::Lt => order.is_lt(),
		Op::Gt => order.is_gt(),
		Op::Le => order.is_le(),
		_ => order.is_ge(),
	};
	Ok(Value::Number(holds as i64))
}

fn unitless_number(value: &Value) -> bool {
	matches!(value, Value::Number(_) | Value::Tolerance(Tolerance { unit: None, .. }))
}

/// A range or a value with tolerance as the closed span it covers, and its unit: `1950 ± 50 AD` is 1900 to 2000 AD
fn span(value: &Value) -> Option<(i64, i64, Option<&'static str>)> {
	match value {
		Value::Range(range) => Some((range.from, range.to, Some(range.unit.name))),
		Value::Tolerance(tolerance) => Some((tolerance.value - tolerance.tolerance, tolerance.value + tolerance.tolerance, tolerance.unit.map(|unit| unit.name))),
		_ => None,
	}
}

/// `1900 - 2000 AD == 1950 AD ± 50`: the same span in the same unit
fn same_span(left: Value, op: Op, right: Value) -> Evaluated {
	let (Some((from, to, unit)), Some((other_from, other_to, other_unit))) = (span(&left), span(&right)) else { return Err(Stop::Unsupported) };
	if unit != other_unit {
		return fail(format!("cannot compare spans in different units: {} and {}", unit.unwrap_or("no unit"), other_unit.unwrap_or("no unit")));
	}
	let same = (from, to) == (other_from, other_to);
	Ok(Value::Number((same == (op == Op::Eq)) as i64))
}

fn sum(left: Quantity, op: Op, right: Quantity) -> Evaluated {
	let (x, y, factors) = aligned(&left, &right)?;
	let amount = if op == Op::Add { x.add(&y) } else { x.add(&y.neg()) };
	Ok(Value::Quantity(Quantity { amount, factors }))
}

/// `1950 ± 50`, `1950 cm ± 50` and `1950 ± 50 cm` all count in the unit that is present
fn tolerance(center: Value, spread: Value) -> Evaluated {
	let plain = |quantity: &Quantity| quantity.single_unit().ok_or_else(|| Stop::Error(format!("{PLAIN_TOLERANCE}, not {quantity}")));
	let counted = |amount: &Rational| whole(amount).ok_or_else(|| Stop::Error(format!("{WHOLE_TOLERANCE}, not {amount}")));
	let (value, tolerance, unit) = match (center, spread) {
		(Value::Number(value), Value::Number(tolerance)) => (value, tolerance, None),
		(Value::Quantity(value), Value::Number(tolerance)) => (counted(&value.amount)?, tolerance, Some(plain(&value)?)),
		(Value::Number(value), Value::Quantity(tolerance)) => (value, counted(&tolerance.amount)?, Some(plain(&tolerance)?)),
		(Value::Quantity(value), Value::Quantity(tolerance)) => {
			plain(&value)?;
			plain(&tolerance)?;
			let (value, tolerance, factors) = aligned(&value, &tolerance)?;
			(counted(&value)?, counted(&tolerance)?, Some(factors[0].unit))
		}
		_ => return fail("a tolerance applies to numbers and quantities"),
	};
	if tolerance < 0 {
		return fail(format!("the tolerance must not be negative: {tolerance}"));
	}
	Ok(Value::Tolerance(Tolerance { value, tolerance, unit }))
}

/// A plain number minus a quantity is a range in that unit: `1900 - 2000 AD`
fn range(from: i64, end: Quantity) -> Evaluated {
	let to = whole(&end.amount).ok_or_else(|| Stop::Error(format!("a range counts whole numbers, not {end}")))?;
	if from > to {
		return fail(format!("descending range: {from} - {end}"));
	}
	Ok(Value::Range(Range { from, to, unit: end.single_unit().expect("guarded by arithmetic") }))
}
