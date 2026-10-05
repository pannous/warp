//! Unit words as values (wiki/unit.md): `3km` is `3*km`, `1 m + 1km` is `1001 m`.
//! Programs made of integers and units are evaluated here at compile time; sums convert to the finer unit.
//! `1950 ± 50` is a value with tolerance, `1900 - 2000 AD` a range; both print back in the form they were written.
//! The canonical unit of a result is the smallest unit involved: `3km+10m` is `3010 m`.
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

/// An integer amount of a unit to a power (m, m², m³); a quantity equals the bare number of its own unit: `3km+10m == 3010`
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quantity {
	pub amount: i64,
	unit: &'static Unit,
	power: u32,
}

const SUPERSCRIPT_DIGITS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];

impl fmt::Display for Quantity {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{} {}", self.amount, self.unit.name)?;
		if self.power != 1 {
			let exponent: String = self.power.to_string().chars().filter_map(|digit| digit.to_digit(10)).map(|digit| SUPERSCRIPT_DIGITS[digit as usize]).collect();
			write!(f, "{exponent}")?;
		}
		Ok(())
	}
}

impl Quantity {
	fn of(amount: i64, unit: &'static Unit) -> Quantity {
		Quantity { amount, unit, power: 1 }
	}

	/// The amount counted in `target` to the same power: 1 m² is 10000 cm²
	fn in_unit(&self, target: &'static Unit) -> Result<i64, String> {
		let scale = |factor: i64| factor.checked_pow(self.power).ok_or_else(|| overflow(self));
		let scaled = self.amount.checked_mul(scale(self.unit.factor)?).ok_or_else(|| overflow(self))?;
		Ok(scaled / scale(target.factor)?)
	}

	fn unit_text(&self) -> String {
		Quantity { amount: 0, ..*self }.to_string().trim_start_matches("0 ").to_string()
	}
}

fn overflow(quantity: &Quantity) -> String {
	format!("{quantity} overflows the integer range")
}

/// A quantity per unit of another dimension: `10 km / 2 h` is `5 km/h`
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rate {
	amount: i64,
	numerator: &'static Unit,
	denominator: &'static Unit,
}

impl fmt::Display for Rate {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{} {}/{}", self.amount, self.numerator.name, self.denominator.name)
	}
}

/// A converted quantity that is no whole number of its unit: `150 cm in m` is `3/2 m`, numerator and denominator coprime
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ratio {
	numerator: i64,
	denominator: i64,
	unit: &'static Unit,
}

impl fmt::Display for Ratio {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}/{} {}", self.numerator, self.denominator, self.unit.name)
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
	if let Some(rate) = data.downcast_ref::<Rate>() {
		return Some(rate.to_string());
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
	Rate(Rate),
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
		Ok(Value::Rate(rate)) => Some(Node::data(rate)),
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
		Node::Key(base, op @ (Op::Square | Op::Cube), nothing) if matches!(nothing.drop_meta(), Node::Empty) => {
			let exponent = if *op == Op::Square { 2 } else { 3 };
			arithmetic(evaluate_in(base, variables)?, Op::Pow, Value::Number(exponent))
		}
		Node::Key(quantity, Op::As, unit) if target_unit(unit).is_some() => convert(evaluate_in(quantity, variables)?, target_unit(unit).expect("guarded")),
		// `2 h * 10 km/h` reads `(2 h * 10 km) / h`: a product of two dimensions is taken as `2 h * (10 km / h)`
		Node::Key(product, Op::Div, divisor) if matches!(product.drop_meta(), Node::Key(_, Op::Mul, _)) => {
			let Node::Key(a, _, b) = product.drop_meta() else { unreachable!("guarded") };
			let (a, b, c) = (evaluate_in(a, variables)?, evaluate_in(b, variables)?, evaluate_in(divisor, variables)?);
			match (&a, &b) {
				(Value::Quantity(x), Value::Quantity(y)) if x.unit.dimension != y.unit.dimension => arithmetic(a, Op::Mul, arithmetic(b, Op::Div, c)?),
				_ => arithmetic(arithmetic(a, Op::Mul, b)?, Op::Div, c),
			}
		}
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
			[quantity, word, unit] if matches!(word.drop_meta(), Node::Symbol(w) if w == IN_WORD) && target_unit(unit).is_some() => {
				convert(evaluate_in(quantity, variables)?, target_unit(unit).expect("guarded"))
			}
			_ => Err(Stop::Unsupported),
		},
		_ => Err(Stop::Unsupported),
	}
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
		Value::Rate(rate) => Ok(Value::Rate(Rate { amount: -rate.amount, ..rate })),
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
		(Value::Number(from), Op::Sub, Value::Quantity(end)) => range(from, end),
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
		(Value::Quantity(a), Op::Div, Value::Quantity(b)) => quotient(a, b),
		(Value::Quantity(a), Op::Mul, Value::Quantity(b)) => product(a, b),
		(Value::Quantity(q), Op::Pow, Value::Number(n)) => power(q, n),
		(Value::Rate(rate), Op::Mul, Value::Quantity(q)) | (Value::Quantity(q), Op::Mul, Value::Rate(rate)) => rate_times(rate, q),
		(Value::Rate(rate), Op::Mul, Value::Number(n)) | (Value::Number(n), Op::Mul, Value::Rate(rate)) => rate
			.amount
			.checked_mul(n)
			.map(|amount| Value::Rate(Rate { amount, ..rate }))
			.ok_or_else(|| Stop::Error(format!("{rate} * {n} overflows the integer range"))),
		(Value::Number(_), Op::Add, Value::Quantity(other)) | (Value::Quantity(other), Op::Add | Op::Sub, Value::Number(_)) => {
			fail(format!("incompatible operands: a plain number and {}", other.unit.name))
		}
		(Value::Number(n), Op::Mul, Value::Quantity(q)) | (Value::Quantity(q), Op::Mul, Value::Number(n)) => q
			.amount
			.checked_mul(n)
			.map(|amount| Value::Quantity(Quantity { amount, ..q }))
			.ok_or_else(|| Stop::Error(overflow(&q))),
		(Value::Quantity(q), Op::Div, Value::Number(n)) if n != 0 && q.amount % n == 0 => {
			Ok(Value::Quantity(Quantity { amount: q.amount / n, ..q }))
		}
		_ => Err(Stop::Unsupported),
	}
}

/// `100 cm in m` is `1 m`, `150 cm in m` is `3/2 m`; a unit of another dimension is the DimensionError
fn convert(value: Value, target: &'static Unit) -> Evaluated {
	let Value::Quantity(quantity) = value else { return Err(Stop::Unsupported) };
	if quantity.power != 1 {
		return fail(format!("converting {quantity} to {} is not supported yet", target.name));
	}
	if quantity.unit.dimension != target.dimension {
		return fail(format!("DimensionError: {quantity} cannot be converted to {}", target.name));
	}
	let scaled = quantity.amount.checked_mul(quantity.unit.factor).ok_or_else(|| Stop::Error(overflow(&quantity)))?;
	let divisor = gcd(scaled, target.factor);
	match target.factor / divisor {
		1 => Ok(Value::Quantity(Quantity::of(scaled / target.factor, target))),
		denominator => Ok(Value::Ratio(Ratio { numerator: scaled / divisor, denominator, unit: target })),
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
	Some(match convert(Value::Quantity(quantity), target) {
		Ok(Value::Quantity(quantity)) => Node::data(quantity),
		Ok(Value::Ratio(ratio)) => Node::data(ratio),
		Err(Stop::Error(message)) => error(&message),
		_ => return None,
	})
}

fn gcd(a: i64, b: i64) -> i64 {
	if b == 0 { a.abs().max(1) } else { gcd(b, a % b) }
}

/// The finer of two units of one dimension: the canonical unit of a result
fn finer_unit(a: &'static Unit, b: &'static Unit) -> Result<&'static Unit, Stop> {
	if a.dimension != b.dimension {
		return fail(format!("DimensionError: incompatible units: {} and {}", a.name, b.name));
	}
	Ok(if a.factor <= b.factor { a } else { b })
}

/// The finer unit of two quantities of one dimension and power: `m²` and `cm²` count in cm², `m²` and `m` do not mix
fn common_unit(left: &Quantity, right: &Quantity) -> Result<&'static Unit, Stop> {
	if left.power != right.power && left.unit.dimension == right.unit.dimension {
		return fail(format!("DimensionError: incompatible units: {} and {}", left.unit_text(), right.unit_text()));
	}
	finer_unit(left.unit, right.unit)
}

/// `3 m * 2 m` is `6 m²`: the factors count in their finer unit, the powers add; other dimensions are not supported yet
fn product(left: Quantity, right: Quantity) -> Evaluated {
	if left.unit.dimension != right.unit.dimension {
		return fail(format!("{left} * {right}: a product of different dimensions is not supported yet"));
	}
	let finer = finer_unit(left.unit, right.unit)?;
	let (x, y) = (left.in_unit(finer).map_err(Stop::Error)?, right.in_unit(finer).map_err(Stop::Error)?);
	match x.checked_mul(y) {
		Some(amount) => Ok(Value::Quantity(Quantity { amount, unit: finer, power: left.power + right.power })),
		None => fail(format!("{left} * {right} overflows the integer range")),
	}
}

/// `(2 km)²` is `4 km²`
fn power(base: Quantity, exponent: i64) -> Evaluated {
	let exponent = u32::try_from(exponent).ok().filter(|exponent| *exponent >= 1).ok_or(Stop::Unsupported)?;
	let amount = base.amount.checked_pow(exponent).ok_or_else(|| Stop::Error(overflow(&base)))?;
	Ok(Value::Quantity(Quantity { amount, unit: base.unit, power: base.power * exponent }))
}

/// `10 km/h * 30 min` is `5 km`: the duration counted in the rate's unit; a result that is no whole number is not supported yet
fn rate_times(rate: Rate, quantity: Quantity) -> Evaluated {
	if quantity.unit.dimension != rate.denominator.dimension || quantity.power != 1 {
		return fail(format!("DimensionError: {rate} * {quantity}: {quantity} is no {}", rate.denominator.name));
	}
	let scaled = rate.amount.checked_mul(quantity.amount).and_then(|amount| amount.checked_mul(quantity.unit.factor));
	let scaled = scaled.ok_or_else(|| Stop::Error(format!("{rate} * {quantity} overflows the integer range")))?;
	if scaled % rate.denominator.factor != 0 {
		return fail(format!("{rate} * {quantity} is not a whole number of {} yet", rate.numerator.name));
	}
	Ok(Value::Quantity(Quantity::of(scaled / rate.denominator.factor, rate.numerator)))
}

/// Comparison of two quantities in their finer unit: `3km == 3000m`, answered 1 or 0
fn compare(left: Quantity, op: Op, right: Quantity) -> Evaluated {
	let finer = common_unit(&left, &right)?;
	let (x, y) = (left.in_unit(finer).map_err(Stop::Error)?, right.in_unit(finer).map_err(Stop::Error)?);
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

/// `6 m / 2 m` is the number 3; `10 km / 2 h` is the rate 5 km/h. A result that is no whole number is not supported yet.
fn quotient(dividend: Quantity, divisor: Quantity) -> Evaluated {
	if divisor.amount == 0 {
		return fail("division by zero");
	}
	if dividend.unit.dimension == divisor.unit.dimension && dividend.power > divisor.power {
		// `6 m² / 2 m` is `3 m`
		let finer = finer_unit(dividend.unit, divisor.unit)?;
		let (x, y) = (dividend.in_unit(finer).map_err(Stop::Error)?, divisor.in_unit(finer).map_err(Stop::Error)?);
		if y == 0 || x % y != 0 {
			return fail(format!("{dividend} / {divisor} is not a whole number yet"));
		}
		return Ok(Value::Quantity(Quantity { amount: x / y, unit: finer, power: dividend.power - divisor.power }));
	}
	if dividend.unit.dimension != divisor.unit.dimension {
		if dividend.power != 1 || divisor.power != 1 {
			return fail(format!("{dividend} / {divisor}: a rate of powers is not supported yet"));
		}
		if dividend.amount % divisor.amount != 0 {
			return fail(format!("{dividend} / {divisor} is not a whole number of {}/{} yet", dividend.unit.name, divisor.unit.name));
		}
		return Ok(Value::Rate(Rate { amount: dividend.amount / divisor.amount, numerator: dividend.unit, denominator: divisor.unit }));
	}
	let finer = common_unit(&dividend, &divisor)?;
	let (x, y) = (dividend.in_unit(finer).map_err(Stop::Error)?, divisor.in_unit(finer).map_err(Stop::Error)?);
	if y == 0 || x % y != 0 {
		return fail(format!("{dividend} / {divisor} is not a whole number yet"));
	}
	Ok(Value::Number(x / y))
}

fn sum(left: Quantity, op: Op, right: Quantity) -> Evaluated {
	let finer = common_unit(&left, &right)?;
	let (x, y) = (left.in_unit(finer).map_err(Stop::Error)?, right.in_unit(finer).map_err(Stop::Error)?);
	let amount = if op == Op::Add { x.checked_add(y) } else { x.checked_sub(y) };
	match amount {
		Some(amount) => Ok(Value::Quantity(Quantity { amount, unit: finer, power: left.power })),
		None => fail(format!("{left} {op} {right} overflows the integer range")),
	}
}

/// `1950 ± 50`, `1950 cm ± 50` and `1950 ± 50 cm` all count in the unit that is present
fn tolerance(center: Value, spread: Value) -> Evaluated {
	let (value, tolerance, unit) = match (center, spread) {
		(Value::Number(value), Value::Number(tolerance)) => (value, tolerance, None),
		(Value::Quantity(value), Value::Number(tolerance)) => (value.amount, tolerance, Some(value.unit)),
		(Value::Number(value), Value::Quantity(tolerance)) => (value, tolerance.amount, Some(tolerance.unit)),
		(Value::Quantity(value), Value::Quantity(tolerance)) => {
			let finer = finer_unit(value.unit, tolerance.unit)?;
			(value.in_unit(finer).map_err(Stop::Error)?, tolerance.in_unit(finer).map_err(Stop::Error)?, Some(finer))
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
	Ok(Value::Range(Range { from, to: end.amount, unit: end.unit }))
}
