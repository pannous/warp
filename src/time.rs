//! Dates and times (Footguns.md → Dates and time zones).
//! The parser lexes RFC 3339 / RFC 9557 literals and `1 month` style durations into data nodes;
//! programs that use them are evaluated here at compile time. A WASM GC representation is future work.

pub mod calendar;

pub use calendar::{Duration, Overflow, Time};
use crate::extensions::numbers::Number;
use crate::node::{error, Node, Separator};
use crate::operators::Op;
use std::cmp::Ordering;
use std::collections::HashMap;

/// A date or time literal as written; validated when evaluated, like `date(2024,2,30)`
#[derive(Clone, Debug, PartialEq)]
pub struct TimeLiteral(pub String);

pub use calendar::{literal_len, unit_after};

#[derive(Clone, Debug, PartialEq)]
enum Value {
	Time(Time),
	Duration(Duration),
	Int(i64),
	Bool(bool),
	Text(String),
	Symbol(String),
}

type Scope = HashMap<String, Value>;

/// The value of a program that uses dates or times, None for any other program
pub fn answer(program: &Node) -> Option<Node> {
	if !mentions_time(program) {
		return None;
	}
	Some(match evaluate(program, &mut Scope::new()) {
		Ok(value) => value.into_node(),
		Err(message) => error(&message),
	})
}

fn time_data(node: &Node) -> bool {
	match node {
		Node::Data(data) => data.downcast_ref::<TimeLiteral>().is_some() || data.downcast_ref::<Duration>().is_some(),
		_ => false,
	}
}

fn is_symbol(node: &Node, name: &str) -> bool {
	matches!(node.drop_meta(), Node::Symbol(symbol) if symbol == name)
}

fn mentions_time(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(name) => name == "now",
		Node::Key(left, _, right) => mentions_time(left) || mentions_time(right),
		Node::List(items, _, _) => {
			items.first().is_some_and(|head| is_symbol(head, "date")) || items.iter().any(mentions_time)
		}
		other => time_data(other),
	}
}

fn unsupported(node: &Node) -> String {
	format!("dates and times are evaluated at compile time only; `{node}` is not supported with them yet")
}

fn now() -> Time {
	let since_epoch = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map(|elapsed| elapsed.as_nanos() as i128)
		.unwrap_or(0);
	Time::Instant(since_epoch)
}

fn evaluate(node: &Node, scope: &mut Scope) -> Result<Value, String> {
	match node.drop_meta() {
		Node::Data(data) => {
			if let Some(TimeLiteral(text)) = data.downcast_ref::<TimeLiteral>() {
				calendar::parse_literal(text).map(Value::Time)
			} else if let Some(duration) = data.downcast_ref::<Duration>() {
				Ok(Value::Duration(*duration))
			} else {
				Err(unsupported(node))
			}
		}
		Node::Number(Number::Int(n)) => Ok(Value::Int(*n)),
		Node::True => Ok(Value::Bool(true)),
		Node::False => Ok(Value::Bool(false)),
		Node::Text(text) => Ok(Value::Text(text.clone())),
		Node::Symbol(name) => Ok(match scope.get(name) {
			Some(value) => value.clone(),
			None if name == "now" => Value::Time(now()),
			None => Value::Symbol(name.clone()),
		}),
		Node::Key(left, op, right) => binary(left, *op, right, scope),
		Node::List(items, _, separator) => list(node, items, separator.clone(), scope),
		Node::Error(message) => Err(message.to_string()),
		_ => Err(unsupported(node)),
	}
}

fn list(node: &Node, items: &[Node], separator: Separator, scope: &mut Scope) -> Result<Value, String> {
	match items {
		[single] => evaluate(single, scope),
		[time, keyword, zone] if is_symbol(keyword, "in") => place_in_zone(time, zone, scope),
		[head, args @ ..] if separator == Separator::None && is_symbol(head, "date") => construct_date(args, scope),
		[head, args @ ..] if separator == Separator::None && is_symbol(head, "add") => add(args, scope),
		_ if matches!(separator, Separator::Semicolon | Separator::Newline) => {
			let mut last = Err(unsupported(node));
			for statement in items {
				last = Ok(evaluate(statement, scope)?);
			}
			last
		}
		_ => Err(unsupported(node)),
	}
}

/// `t in "Europe/Berlin"`: the only way an instant gets a wall clock
fn place_in_zone(time: &Node, zone: &Node, scope: &mut Scope) -> Result<Value, String> {
	let time = match evaluate(time, scope)? {
		Value::Time(time) => time,
		other => return Err(format!("`in` places a time in a zone, got {}", other.kind())),
	};
	let zone = match evaluate(zone, scope)? {
		Value::Text(name) | Value::Symbol(name) => calendar::zone_named(&name)?,
		other => return Err(format!("`in` needs a zone name like \"Europe/Berlin\", got {}", other.kind())),
	};
	time.in_zone(zone).map(Value::Time)
}

fn int_argument(node: &Node, scope: &mut Scope) -> Result<i64, String> {
	match evaluate(node, scope)? {
		Value::Int(n) => Ok(n),
		other => Err(format!("expected an integer, got {}", other.kind())),
	}
}

/// `date(2024,2,29)`: fields are validated, never rolled over
fn construct_date(args: &[Node], scope: &mut Scope) -> Result<Value, String> {
	let [year, month, day] = args else {
		return Err(format!("date(year, month, day) takes 3 arguments, got {}", args.len()));
	};
	let date = calendar::Date::new(int_argument(year, scope)?, int_argument(month, scope)?, int_argument(day, scope)?)?;
	Ok(Value::Time(Time::Date(date)))
}

/// `add(2024-01-31, 1 month, overflow: clamp)`: calendar overflow spelled out
fn add(args: &[Node], scope: &mut Scope) -> Result<Value, String> {
	let (time, duration, options) = match args {
		[time, duration, options @ ..] => (evaluate(time, scope)?, evaluate(duration, scope)?, options),
		_ => return Err("add(time, duration, overflow: clamp) takes a time and a duration".to_string()),
	};
	let mut overflow = Overflow::Reject;
	for option in options {
		let Node::Key(name, Op::Colon, value) = option.drop_meta() else {
			return Err(unsupported(option));
		};
		if !is_symbol(name, "overflow") {
			return Err(format!("add has no option {name}"));
		}
		overflow = match evaluate(value, scope)? {
			Value::Symbol(mode) | Value::Text(mode) if mode == "clamp" || mode == "constrain" => Overflow::Clamp,
			Value::Symbol(mode) | Value::Text(mode) if mode == "reject" => Overflow::Reject,
			other => return Err(format!("overflow is clamp or reject, got {other:?}")),
		};
	}
	match (time, duration) {
		(Value::Time(time), Value::Duration(duration)) => time.add(duration, overflow).map(Value::Time),
		(time, duration) => Err(format!("add needs a time and a duration, got {} and {}", time.kind(), duration.kind())),
	}
}

fn binary(left: &Node, op: Op, right: &Node, scope: &mut Scope) -> Result<Value, String> {
	match op {
		Op::Assign | Op::Define => {
			let Node::Symbol(name) = left.drop_meta() else {
				return Err(unsupported(left));
			};
			let value = evaluate(right, scope)?;
			scope.insert(name.clone(), value.clone());
			Ok(value)
		}
		Op::Dot => {
			let Node::Symbol(field) = right.drop_meta() else {
				return Err(unsupported(right));
			};
			match evaluate(left, scope)? {
				Value::Time(time) => time.field(field).map(Value::Int),
				Value::Duration(duration) => duration.field(field).map(Value::Int),
				other => Err(format!("{} has no field {field}", other.kind())),
			}
		}
		_ => {
			let left = evaluate(left, scope)?;
			let right = evaluate(right, scope)?;
			apply(left, op, right)
		}
	}
}

fn compared(op: Op, ordering: Ordering) -> bool {
	match op {
		Op::Eq => ordering == Ordering::Equal,
		Op::Ne => ordering != Ordering::Equal,
		Op::Lt => ordering == Ordering::Less,
		Op::Le => ordering != Ordering::Greater,
		Op::Gt => ordering == Ordering::Greater,
		_ => ordering != Ordering::Less, // Ge
	}
}

fn apply(left: Value, op: Op, right: Value) -> Result<Value, String> {
	match (left, op, right) {
		(Value::Time(time), Op::Add, Value::Duration(duration)) | (Value::Duration(duration), Op::Add, Value::Time(time)) => {
			time.add(duration, Overflow::Reject).map(Value::Time)
		}
		(Value::Time(time), Op::Sub, Value::Duration(duration)) => time.add(duration.times(-1), Overflow::Reject).map(Value::Time),
		(Value::Time(later), Op::Sub, Value::Time(earlier)) => Ok(match later.since(earlier)? {
			Ok(days) => Value::Int(days),
			Err(duration) => Value::Duration(duration),
		}),
		(Value::Time(a), op, Value::Time(b)) if op.is_comparison() => Ok(Value::Bool(compared(op, a.compare(b)?))),
		(Value::Duration(a), Op::Add, Value::Duration(b)) => Ok(Value::Duration(a.plus(b))),
		(Value::Duration(a), Op::Sub, Value::Duration(b)) => Ok(Value::Duration(a.plus(b.times(-1)))),
		(Value::Duration(duration), Op::Mul, Value::Int(n)) | (Value::Int(n), Op::Mul, Value::Duration(duration)) => Ok(Value::Duration(duration.times(n))),
		(Value::Duration(a), Op::Eq, Value::Duration(b)) => Ok(Value::Bool(a == b)),
		(Value::Duration(a), Op::Ne, Value::Duration(b)) => Ok(Value::Bool(a != b)),
		(Value::Int(a), op, Value::Int(b)) if op.is_comparison() => Ok(Value::Bool(compared(op, a.cmp(&b)))),
		(Value::Int(a), Op::Add, Value::Int(b)) => Ok(Value::Int(a + b)),
		(Value::Int(a), Op::Sub, Value::Int(b)) => Ok(Value::Int(a - b)),
		(Value::Int(a), Op::Mul, Value::Int(b)) => Ok(Value::Int(a * b)),
		(Value::Bool(a), Op::And, Value::Bool(b)) => Ok(Value::Bool(a && b)),
		(Value::Bool(a), Op::Or, Value::Bool(b)) => Ok(Value::Bool(a || b)),
		(Value::Bool(a), Op::Eq, Value::Bool(b)) => Ok(Value::Bool(a == b)),
		(Value::Bool(a), Op::Ne, Value::Bool(b)) => Ok(Value::Bool(a != b)),
		(left, op, right) => Err(format!("cannot apply {op} to a {} and a {}", left.kind(), right.kind())),
	}
}

impl Value {
	fn kind(&self) -> &'static str {
		match self {
			Value::Time(time) => time.kind(),
			Value::Duration(_) => "duration",
			Value::Int(_) => "number",
			Value::Bool(_) => "boolean",
			Value::Text(_) => "text",
			Value::Symbol(_) => "symbol",
		}
	}

	fn into_node(self) -> Node {
		match self {
			Value::Time(time) => Node::data(time),
			Value::Duration(duration) => Node::data(duration),
			Value::Int(n) => Node::Number(Number::Int(n)),
			Value::Bool(truth) => Node::Number(Number::Int(truth as i64)), // eval encodes booleans as Int 1/0
			Value::Text(text) => Node::Text(text),
			Value::Symbol(name) => Node::Symbol(name),
		}
	}
}
