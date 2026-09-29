//! Unit words as values (wiki/unit.md): `3km` is `3*km`, `1 m + 1km` is `1001 m`.
//! Programs made of integers and units are evaluated here at compile time; sums convert to the finer unit.
//! Anything else (floats, variables, functions) takes the normal path, where a unit word is an ordinary symbol.

use crate::extensions::numbers::Number;
use crate::node::{error, Bracket, Node};
use crate::operators::Op;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Dimension {
	Length,
	Mass,
}

#[derive(Debug, PartialEq)]
/// `factor` counts the smallest unit of the dimension: 1 km = 1_000_000 mm
struct Unit {
	name: &'static str,
	dimension: Dimension,
	factor: i64,
}

const UNITS: [Unit; 7] = [
	Unit { name: "mm", dimension: Dimension::Length, factor: 1 },
	Unit { name: "cm", dimension: Dimension::Length, factor: 10 },
	Unit { name: "m", dimension: Dimension::Length, factor: 1_000 },
	Unit { name: "km", dimension: Dimension::Length, factor: 1_000_000 },
	Unit { name: "mg", dimension: Dimension::Mass, factor: 1 },
	Unit { name: "g", dimension: Dimension::Mass, factor: 1_000 },
	Unit { name: "kg", dimension: Dimension::Mass, factor: 1_000_000 },
];

fn unit_named(name: &str) -> Option<&'static Unit> {
	UNITS.iter().find(|unit| unit.name == name)
}

pub fn is_unit(name: &str) -> bool {
	unit_named(name).is_some()
}

/// An integer amount of a unit; a quantity equals the bare number of its own unit: `3km+10m == 3010`
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quantity {
	pub amount: i64,
	unit: &'static Unit,
}

impl fmt::Display for Quantity {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{} {}", self.amount, self.unit.name)
	}
}

impl Quantity {
	fn in_unit(&self, target: &'static Unit) -> Result<i64, String> {
		let scaled = self.amount.checked_mul(self.unit.factor).ok_or_else(|| overflow(self))?;
		Ok(scaled / target.factor)
	}
}

fn overflow(quantity: &Quantity) -> String {
	format!("{quantity} overflows the integer range")
}

#[derive(Clone, Debug)]
enum Value {
	Number(i64),
	Quantity(Quantity),
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
	if !mentions_unit(program) || defines_unit_name(program) {
		return None;
	}
	match evaluate(program) {
		Ok(Value::Number(n)) => Some(Node::int(n)),
		Ok(Value::Quantity(quantity)) => Some(Node::data(quantity)),
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

fn mentions_unit(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(name) => unit_named(name).is_some(),
		other => children(other).into_iter().any(mentions_unit),
	}
}

/// `m=5;3m`: a variable of that name shadows the unit
fn defines_unit_name(node: &Node) -> bool {
	let defines = match node.drop_meta() {
		Node::Key(target, Op::Assign | Op::Define | Op::Colon, _) => {
			matches!(target.drop_meta(), Node::Symbol(name) if unit_named(name).is_some())
		}
		_ => false,
	};
	defines || children(node).into_iter().any(defines_unit_name)
}

fn evaluate(node: &Node) -> Evaluated {
	match node.drop_meta() {
		Node::Number(Number::Int(n)) => Ok(Value::Number(*n)),
		Node::Symbol(name) => unit_named(name)
			.map(|unit| Value::Quantity(Quantity { amount: 1, unit }))
			.ok_or(Stop::Unsupported),
		Node::Key(left, Op::Neg, right) if matches!(left.as_ref(), Node::Empty) => negate(evaluate(right)?),
		Node::Key(left, op, right) => arithmetic(evaluate(left)?, *op, evaluate(right)?),
		Node::List(items, Bracket::None, _) => match items.as_slice() {
			[single] => evaluate(single),
			[count, unit] if is_unit_word(unit) => arithmetic(evaluate(count)?, Op::Mul, evaluate(unit)?),
			_ => Err(Stop::Unsupported),
		},
		_ => Err(Stop::Unsupported),
	}
}

fn is_unit_word(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(name) if unit_named(name).is_some())
}

fn negate(value: Value) -> Evaluated {
	match value {
		Value::Number(n) => Ok(Value::Number(-n)),
		Value::Quantity(quantity) => Ok(Value::Quantity(Quantity { amount: -quantity.amount, ..quantity })),
	}
}

fn arithmetic(left: Value, op: Op, right: Value) -> Evaluated {
	match (left, op, right) {
		(Value::Number(a), Op::Add | Op::Sub | Op::Mul, Value::Number(b)) => {
			let result = match op {
				Op::Add => a.checked_add(b),
				Op::Sub => a.checked_sub(b),
				_ => a.checked_mul(b),
			};
			result.map(Value::Number).ok_or(Stop::Unsupported)
		}
		(Value::Quantity(a), Op::Add | Op::Sub, Value::Quantity(b)) => sum(a, op, b),
		(Value::Number(_), Op::Add | Op::Sub, Value::Quantity(other)) | (Value::Quantity(other), Op::Add | Op::Sub, Value::Number(_)) => {
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

fn sum(left: Quantity, op: Op, right: Quantity) -> Evaluated {
	let (a, b) = (left.unit, right.unit);
	if a.dimension != b.dimension {
		return fail(format!("incompatible units: {} and {}", a.name, b.name));
	}
	let finer = if a.factor <= b.factor { a } else { b };
	let (x, y) = (left.in_unit(finer).map_err(Stop::Error)?, right.in_unit(finer).map_err(Stop::Error)?);
	let amount = if op == Op::Add { x.checked_add(y) } else { x.checked_sub(y) };
	match amount {
		Some(amount) => Ok(Value::Quantity(Quantity { amount, unit: finer })),
		None => fail(format!("{left} {op} {right} overflows the integer range")),
	}
}
