//! Unit words as values (wiki/unit.md): `3km` is `3*km`, `1 m + 1km` is `1001 m`.
//! Programs made of integers and units are evaluated here at compile time; sums convert to the finer unit.
//! `1950 ± 50` is a value with tolerance, `1900 - 2000 AD` a range; both print back in the form they were written.
//! The canonical unit of a result is the smallest unit involved: `3km+10m` is `3010 m`.
//! A quantity is an amount times units to powers: `6 m²`, `6 m·kg`, `5 km/h` (km·h⁻¹), `1 kg·m/s²`.
//! Anything else (floats, variables, functions) takes the normal path, where a unit word is an ordinary symbol.

use crate::extensions::numbers::Number;
use crate::meta::Dada;
use crate::node::{error, Bracket, Node, Separator};
use std::collections::HashMap;
use crate::operators::Op;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Dimension {
	Length,
	Mass,
	Era,
	Time,
}

#[derive(Debug, PartialEq)]
/// `factor` counts the smallest unit of the dimension: 1 km = 1_000_000 mm
struct Unit {
	name: &'static str,
	dimension: Dimension,
	factor: i64,
}

const UNITS: [Unit; 12] = [
	Unit { name: "ms", dimension: Dimension::Time, factor: 1 },
	Unit { name: "s", dimension: Dimension::Time, factor: 1_000 },
	Unit { name: "min", dimension: Dimension::Time, factor: 60_000 },
	Unit { name: "h", dimension: Dimension::Time, factor: 3_600_000 },
	Unit { name: "mm", dimension: Dimension::Length, factor: 1 },
	Unit { name: "cm", dimension: Dimension::Length, factor: 10 },
	Unit { name: "m", dimension: Dimension::Length, factor: 1_000 },
	Unit { name: "km", dimension: Dimension::Length, factor: 1_000_000 },
	Unit { name: "mg", dimension: Dimension::Mass, factor: 1 },
	Unit { name: "g", dimension: Dimension::Mass, factor: 1_000 },
	Unit { name: "kg", dimension: Dimension::Mass, factor: 1_000_000 },
	Unit { name: "AD", dimension: Dimension::Era, factor: 1 },
];

/// The long names a conversion target may use, singular or plural: `2 h in minutes` (P36). Quantities keep the short
/// names, `2 minutes` stays a duration of the time module.
const LONG_NAMES: [(&str, &str); 12] = [
	("millisecond", "ms"), ("second", "s"), ("minute", "min"), ("hour", "h"),
	("millimeter", "mm"), ("centimeter", "cm"), ("meter", "m"), ("metre", "m"), ("kilometer", "km"),
	("milligram", "mg"), ("gram", "g"), ("kilogram", "kg"),
];
/// `100 cm in m`: the word of a conversion written with spaces (`as` is an operator)
const IN_WORD: &str = "in";

fn unit_named(name: &str) -> Option<&'static Unit> {
	UNITS.iter().find(|unit| unit.name == name)
}

/// A conversion target: a unit by its short name or its long name, `minute` or `minutes`
fn target_unit(node: &Node) -> Option<&'static Unit> {
	let Node::Symbol(name) = node.drop_meta() else { return None };
	let singular = name.strip_suffix('s').unwrap_or(name);
	unit_named(name).or_else(|| LONG_NAMES.iter().find(|(long, _)| *long == singular || *long == name).and_then(|(_, short)| unit_named(short)))
}

pub fn is_unit(name: &str) -> bool {
	unit_named(name).is_some()
}

/// A unit to a power within a quantity: km¹, h⁻¹, m²
#[derive(Clone, Copy, Debug, PartialEq)]
struct Factor {
	unit: &'static Unit,
	power: i32,
}

/// An integer amount of a product of units to powers: `3 m`, `6 m²`, `6 m·kg`, `5 km/h` (km·h⁻¹), `1 kg·m/s²`.
/// A quantity equals the bare number of its own unit: `3km+10m == 3010`
#[derive(Clone, Debug, PartialEq)]
pub struct Quantity {
	pub amount: i64,
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

impl fmt::Display for Quantity {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{} {}", self.amount, units_text(&self.factors))
	}
}

impl Quantity {
	fn of(amount: i64, unit: &'static Unit) -> Quantity {
		Quantity { amount, factors: vec![Factor { unit, power: 1 }] }
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

/// A converted quantity that is no whole number of its unit: `150 cm in m` is `3/2 m`, numerator and denominator coprime
#[derive(Clone, Debug, PartialEq)]
pub struct Ratio {
	numerator: i64,
	denominator: i64,
	factors: Vec<Factor>,
}

impl fmt::Display for Ratio {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}/{} {}", self.numerator, self.denominator, units_text(&self.factors))
	}
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
		self.unit.map_or(Ok(()), |unit| write!(f, " {}", unit.name))
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
		write!(f, "{} - {} {}", self.from, self.to, self.unit.name)
	}
}

/// The text of a unit result held in a `Node::Data`, None for any other data
pub fn describe(data: &Dada) -> Option<String> {
	if let Some(quantity) = data.downcast_ref::<Quantity>() {
		return Some(quantity.to_string());
	}
	if let Some(tolerance) = data.downcast_ref::<Tolerance>() {
		return Some(tolerance.to_string());
	}
	if let Some(ratio) = data.downcast_ref::<Ratio>() {
		return Some(ratio.to_string());
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
	Ratio(Ratio),
	Tolerance(Tolerance),
	Range(Range),
}

enum Stop {
	/// not a program of integers and units: compile it normally
	Unsupported,
	Error(String),
}

type Evaluated = Result<Value, Stop>;

fn fail<T>(message: impl Into<String>) -> Result<T, Stop> {
	Err(Stop::Error(message.into()))
}

/// The value of a program that uses units, None for any other program
pub fn answer(program: &Node) -> Option<Node> {
	if !needs_quantities(program) || defines_unit_name(program) {
		return None;
	}
	match evaluate(program) {
		Ok(Value::Number(n)) => Some(Node::int(n)),
		Ok(Value::Quantity(quantity)) => Some(Node::data(quantity)),
		Ok(Value::Ratio(ratio)) => Some(Node::data(ratio)),
		Ok(Value::Tolerance(tolerance)) => Some(Node::data(tolerance)),
		Ok(Value::Range(range)) => Some(Node::data(range)),
		Err(Stop::Error(message)) => Some(error(&message)),
		Err(Stop::Unsupported) => None,
	}
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

/// `m=5;3m`: a variable of that name shadows the unit, and so does a parameter: `s => s + t`, `f(s) := …`
fn defines_unit_name(node: &Node) -> bool {
	let names_unit = |node: &Node| matches!(node.drop_meta(), Node::Symbol(name) if unit_named(name).is_some());
	let names_parameter = |head: &Node| match head.drop_meta() {
		Node::List(items, _, _) => items.iter().any(|item| names_unit(item) || matches!(item.drop_meta(), Node::Key(name, Op::Colon, _) if names_unit(name))),
		other => names_unit(other),
	};
	let defines = match node.drop_meta() {
		Node::Key(head, Op::Assign | Op::Define, _) if matches!(head.drop_meta(), Node::List(..)) => names_parameter(head),
		Node::Key(target, Op::Assign | Op::Define | Op::Colon, _) => names_unit(target),
		Node::Key(parameters, Op::Arrow | Op::FatArrow, _) => names_parameter(parameters),
		_ => false,
	};
	defines || children(node).into_iter().any(defines_unit_name)
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
		Node::Key(left, op, right) => arithmetic(evaluate_in(left, variables)?, *op, evaluate_in(right, variables)?),
		Node::List(items, Bracket::Round, _) if items.len() == 1 => evaluate_in(&items[0], variables),
		Node::List(items, Bracket::None, Separator::Semicolon | Separator::Newline) if items.len() > 1 => {
			items.iter().try_fold(Value::Number(0), |_, item| evaluate_in(item, variables))
		}
		Node::List(items, Bracket::None, _) => match items.as_slice() {
			[single] => evaluate_in(single, variables),
			[count, unit] if is_unit_word(unit) => arithmetic(evaluate_in(count, variables)?, Op::Mul, evaluate_in(unit, variables)?),
			// `3010 meters`, and a unit after an expression belongs to its last amount: `3km+10m == 3010 meters`
			[amount, unit] if target_unit(unit).is_some() => match amount.drop_meta() {
				Node::Key(..) => evaluate_in(&with_unit(amount, unit), variables),
				_ => arithmetic(evaluate_in(amount, variables)?, Op::Mul, Value::Quantity(Quantity::of(1, target_unit(unit).expect("guarded")))),
			},
			[quantity, word, unit] if matches!(word.drop_meta(), Node::Symbol(w) if w == IN_WORD) && unit_expression(unit).is_some() => {
				convert(evaluate_in(quantity, variables)?, unit_expression(unit).expect("guarded"))
			}
			_ => Err(Stop::Unsupported),
		},
		_ => Err(Stop::Unsupported),
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
		Value::Quantity(quantity) => Ok(Value::Quantity(Quantity { amount: -quantity.amount, ..quantity })),
		Value::Ratio(ratio) => Ok(Value::Ratio(Ratio { numerator: -ratio.numerator, ..ratio })),
		Value::Tolerance(_) | Value::Range(_) => fail("cannot negate a value with tolerance or a range"),
	}
}

fn arithmetic(left: Value, op: Op, right: Value) -> Evaluated {
	match (left, op, right) {
		(Value::Tolerance(_) | Value::Range(_), _, _) | (_, _, Value::Tolerance(_) | Value::Range(_)) => {
			fail(format!("arithmetic on a value with tolerance or a range is not supported: {op}"))
		}
		(Value::Ratio(ratio), _, _) | (_, _, Value::Ratio(ratio)) => fail(format!("arithmetic on the converted quantity {ratio} is not supported yet: {op}")),
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
			let what = || format!("{a} * {b}");
			combine(a.amount as i128 * b.amount as i128, 1, [a.factors.clone(), b.factors.clone()].concat(), what)
		}
		(Value::Quantity(a), Op::Div, Value::Quantity(b)) if b.amount != 0 => {
			let what = || format!("{a} / {b}");
			combine(a.amount as i128, b.amount as i128, [a.factors.clone(), inverse(&b.factors)].concat(), what)
		}
		(Value::Quantity(_), Op::Div, Value::Quantity(_)) => fail("division by zero"),
		(Value::Quantity(q), Op::Pow, Value::Number(n)) => power(q, n),
		(Value::Number(_), Op::Add, Value::Quantity(other)) | (Value::Quantity(other), Op::Add | Op::Sub, Value::Number(_)) => {
			fail(format!("incompatible operands: a plain number and {}", units_text(&other.factors)))
		}
		(Value::Number(n), Op::Mul, Value::Quantity(q)) | (Value::Quantity(q), Op::Mul, Value::Number(n)) => match q.amount.checked_mul(n) {
			Some(amount) => Ok(Value::Quantity(Quantity { amount, ..q })),
			None => fail(format!("{q} * {n} overflows the integer range")),
		},
		(Value::Quantity(q), Op::Div, Value::Number(n)) if n != 0 && q.amount % n == 0 => {
			Ok(Value::Quantity(Quantity { amount: q.amount / n, ..q }))
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
	let (numerator, denominator) = rescaled(quantity.amount as i128, 1, &quantity.factors, in_target).ok_or_else(|| Stop::Error(overflow(&quantity)))?;
	let small = |n: i128| i64::try_from(n).map_err(|_| Stop::Error(overflow(&quantity)));
	match denominator {
		1 => Ok(Value::Quantity(Quantity { amount: small(numerator)?, factors: target })),
		_ => Ok(Value::Ratio(Ratio { numerator: small(numerator)?, denominator: small(denominator)?, factors: target })),
	}
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
	if duration.months != 0 {
		return Some(error(&format!("DimensionError: {duration} has no fixed length in {}", target.name)));
	}
	let nanos = duration.nanos + duration.days as i128 * DAY_MILLISECONDS * NANOS_PER_MILLISECOND;
	if nanos % NANOS_PER_MILLISECOND != 0 {
		return Some(error(&format!("{duration} is no whole number of milliseconds")));
	}
	let milliseconds = i64::try_from(nanos / NANOS_PER_MILLISECOND).ok()?;
	let quantity = Quantity::of(milliseconds, unit_named("ms").expect("a unit"));
	Some(match convert(Value::Quantity(quantity), vec![Factor { unit: target, power: 1 }]) {
		Ok(Value::Quantity(quantity)) => Node::data(quantity),
		Ok(Value::Ratio(ratio)) => Node::data(ratio),
		Err(Stop::Error(message)) => error(&message),
		_ => return None,
	})
}

fn gcd(a: i64, b: i64) -> i64 {
	if b == 0 { a.abs().max(1) } else { gcd(b, a % b) }
}

fn overflow(quantity: &Quantity) -> String {
	format!("{quantity} overflows the integer range")
}

fn gcd128(a: i128, b: i128) -> i128 {
	if b == 0 { a.abs().max(1) } else { gcd128(b, a % b) }
}

/// numerator/denominator · Π (unit/target)^power over the factors, in lowest terms (denominator > 0); None on overflow
fn rescaled(numerator: i128, denominator: i128, factors: &[Factor], target: impl Fn(&Factor) -> &'static Unit) -> Option<(i128, i128)> {
	let (mut numerator, mut denominator) = (numerator, denominator);
	for factor in factors {
		let (from, to) = (factor.unit.factor as i128, target(factor).factor as i128);
		let (up, down) = if factor.power > 0 { (from, to) } else { (to, from) };
		let power = factor.power.unsigned_abs();
		numerator = numerator.checked_mul(up.checked_pow(power)?)?;
		denominator = denominator.checked_mul(down.checked_pow(power)?)?;
		let divisor = gcd128(numerator, denominator);
		(numerator, denominator) = (numerator / divisor, denominator / divisor);
	}
	if denominator < 0 {
		(numerator, denominator) = (-numerator, -denominator);
	}
	Some((numerator, denominator))
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

/// numerator/denominator times the factors as one quantity: each dimension counts in its finest unit, its powers add
/// (`3 m * 2 m` is `6 m²`, `2 m * 3 kg` is `6 m·kg`, `10 km/h * 30 min` is `5 km`); no unit left is a plain number.
/// A result that is no whole number of its units is not supported yet
fn combine(numerator: i128, denominator: i128, factors: Vec<Factor>, what: impl Fn() -> String) -> Evaluated {
	let finest = finest_units(&factors);
	let result: Vec<Factor> = signature(&factors).into_iter()
		.map(|(dimension, power)| Factor { unit: finest.iter().find(|unit| unit.dimension == dimension).copied().expect("a unit per dimension"), power })
		.collect();
	let result = order_of_appearance(result, &factors);
	let (numerator, denominator) = rescaled(numerator, denominator, &factors, finest_of(&finest)).ok_or_else(|| Stop::Error(format!("{} overflows the integer range", what())))?;
	if denominator != 1 {
		return fail(format!("{} is not a whole number of {} yet", what(), units_text(&result)));
	}
	let amount = i64::try_from(numerator).map_err(|_| Stop::Error(format!("{} overflows the integer range", what())))?;
	Ok(if result.is_empty() { Value::Number(amount) } else { Value::Quantity(Quantity { amount, factors: result }) })
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
	let amount = base.amount.checked_pow(exponent).ok_or_else(|| Stop::Error(overflow(&base)))?;
	let factors = base.factors.iter().map(|factor| Factor { power: factor.power * exponent as i32, ..*factor }).collect();
	Ok(Value::Quantity(Quantity { amount, factors }))
}

/// Both amounts counted in the finer unit of each dimension: `m²` and `cm²` count in cm², `m²` and `m` do not mix
fn aligned(left: &Quantity, right: &Quantity) -> Result<(i64, i64, Vec<Factor>), Stop> {
	if signature(&left.factors) != signature(&right.factors) {
		return fail(format!("DimensionError: incompatible units: {} and {}", units_text(&left.factors), units_text(&right.factors)));
	}
	let finest = finest_units(&[left.factors.clone(), right.factors.clone()].concat());
	let target = finest_of(&finest);
	let factors: Vec<Factor> = left.factors.iter().map(|factor| Factor { unit: target(factor), ..*factor }).collect();
	let amount = |quantity: &Quantity| -> Result<i64, Stop> {
		match rescaled(quantity.amount as i128, 1, &quantity.factors, &target) {
			Some((amount, 1)) => i64::try_from(amount).map_err(|_| Stop::Error(overflow(quantity))),
			Some(_) => fail(format!("{quantity} is not a whole number of {} yet", units_text(&factors))),
			None => fail(overflow(quantity)),
		}
	};
	Ok((amount(left)?, amount(right)?, factors.clone()))
}

/// Comparison of two quantities in their finer units: `3km == 3000m`, answered 1 or 0
fn compare(left: Quantity, op: Op, right: Quantity) -> Evaluated {
	let (x, y, _) = aligned(&left, &right)?;
	let holds = match op {
		Op::Eq => x == y,
		Op::Ne => x != y,
		Op::Lt => x < y,
		Op::Gt => x > y,
		Op::Le => x <= y,
		_ => x >= y,
	};
	Ok(Value::Number(holds as i64))
}

fn sum(left: Quantity, op: Op, right: Quantity) -> Evaluated {
	let (x, y, factors) = aligned(&left, &right)?;
	let amount = if op == Op::Add { x.checked_add(y) } else { x.checked_sub(y) };
	match amount {
		Some(amount) => Ok(Value::Quantity(Quantity { amount, factors })),
		None => fail(format!("{left} {op} {right} overflows the integer range")),
	}
}

/// `1950 ± 50`, `1950 cm ± 50` and `1950 ± 50 cm` all count in the unit that is present
fn tolerance(center: Value, spread: Value) -> Evaluated {
	let plain = |quantity: &Quantity| quantity.single_unit().ok_or_else(|| Stop::Error(format!("a tolerance applies to numbers and plain quantities, not {quantity}")));
	let (value, tolerance, unit) = match (center, spread) {
		(Value::Number(value), Value::Number(tolerance)) => (value, tolerance, None),
		(Value::Quantity(value), Value::Number(tolerance)) => (value.amount, tolerance, Some(plain(&value)?)),
		(Value::Number(value), Value::Quantity(tolerance)) => (value, tolerance.amount, Some(plain(&tolerance)?)),
		(Value::Quantity(value), Value::Quantity(tolerance)) => {
			plain(&value)?;
			plain(&tolerance)?;
			let (value, tolerance, factors) = aligned(&value, &tolerance)?;
			(value, tolerance, Some(factors[0].unit))
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
	if from > end.amount {
		return fail(format!("descending range: {from} - {end}"));
	}
	Ok(Value::Range(Range { from, to: end.amount, unit: end.single_unit().expect("guarded by arithmetic") }))
}
