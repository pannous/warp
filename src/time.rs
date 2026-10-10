//! Dates and times (Footguns.md → Dates and time zones).
//! The parser lexes RFC 3339 / RFC 9557 literals and `1 month` style durations into data nodes;
//! programs that use them are evaluated here at compile time. `now` alone runs: system_values reads it as
//! `instant_at(clock())`, an instant node (Kind::Time) built when the program runs (wasm_emitter/times.rs).

pub mod calendar;

pub use calendar::{with_rules, Disambiguation, Duration, Overflow, Time, TzRules, Zone, Zoned};
use crate::extensions::numbers::Number;
use crate::node::{error, Node, Separator};
use crate::operators::Op;
use std::collections::HashMap;

pub const NOW_WORD: &str = "now";
/// instant_at(milliseconds since 1970): the instant node `now` is at run time
pub const INSTANT_AT: &str = "instant_at";

/// `now` as the program reads it: the host's clock when it runs
pub fn now_call() -> Node {
	crate::lowering::nodes::call(INSTANT_AT, vec![crate::lowering::nodes::call(crate::host::CLOCK, vec![])])
}

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
	/// a duration in a unit, `2 hours in minutes` (units::duration_in)
	Converted(Node),
	/// `time := now` (a getter since P71, getters.rs: `(time) := now`): its body, evaluated at every read
	Getter(Node),
}

type Scope = HashMap<String, Value>;

/// An instant read back from the run-time node (Kind::Time): its nanoseconds since 1970 UTC
pub fn instant_node(nanoseconds: Option<Number>) -> Node {
	match nanoseconds {
		Some(Number::Int(nanoseconds)) => Node::data(Time::Instant(nanoseconds as i128)),
		_ => error("unreadable instant"),
	}
}

/// The value of a program that uses dates or times, None for any other program
pub fn answer(program: &Node) -> Option<Node> {
	if !mentions_time(program) || !needs_folding(program) {
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

fn is_call_of(node: &Node, name: &str) -> bool {
	matches!(node.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(|head| is_symbol(head, name)))
}

/// What only this compile-time evaluator knows so far: literals, durations, `date(…)`, `t in "Europe/Berlin"`;
/// a program with none of them reads `now` at run time
fn needs_folding(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= time_data(part) || is_call_of(part, "date") || places_in_zone(part));
	found
}

fn places_in_zone(node: &Node) -> bool {
	matches!(node, Node::List(items, _, _) if matches!(items.as_slice(), [_, keyword, zone]
		if is_symbol(keyword, "in") && matches!(zone.drop_meta(), Node::Text(name) if calendar::zone_named(name).is_ok())))
}

fn mentions_time(node: &Node) -> bool {
	match node.drop_meta() {
		// a key or member name (`{date: 5}`, `x.date`) is no use of the time
		Node::Key(key, Op::Colon, value) if matches!(key.drop_meta(), Node::Symbol(_)) => mentions_time(value),
		Node::Key(owner, Op::Dot, field) if matches!(field.drop_meta(), Node::Symbol(_)) => mentions_time(owner),
		Node::Key(left, _, right) => mentions_time(left) || mentions_time(right),
		Node::List(items, _, _) => {
			items.first().is_some_and(|head| is_symbol(head, "date") || is_symbol(head, INSTANT_AT)) || items.iter().any(mentions_time)
		}
		other => time_data(other),
	}
}

/// `(time)`, the head of a getter definition
fn getter_name(head: &Node) -> Option<&str> {
	match head.drop_meta() {
		Node::List(items, crate::node::Bracket::Round, _) => match items.as_slice() {
			[Node::Symbol(name)] => Some(name),
			[single] => match single.drop_meta() { Node::Symbol(name) => Some(name), _ => None },
			_ => None,
		},
		_ => None,
	}
}

fn unsupported(node: &Node) -> String {
	format!("dates and times are evaluated at compile time only; `{node}` is not supported with them yet")
}

fn now() -> Time {
	#[cfg(all(target_arch = "wasm32", not(feature = "native")))]
	return Time::Instant(crate::web::now_nanos());
	#[allow(unreachable_code)]
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
			Some(Value::Getter(body)) => return evaluate(&body.clone(), scope),
			Some(value) => value.clone(),
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
		[head, ..] if is_symbol(head, INSTANT_AT) => Ok(Value::Time(now())),
		[single] => evaluate(single, scope),
		[time, keyword, zone] if is_symbol(keyword, "in") => place_in_zone(time, zone, scope),
		[head, args @ ..] if separator == Separator::None && is_symbol(head, "date") => construct_date(args, scope),
		[head, args @ ..] if separator == Separator::None && is_symbol(head, "add") => add(args, scope),
		[head, args @ ..] if separator == Separator::None && is_symbol(head, "zoned") => zoned(args, scope),
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
		Value::Duration(duration) => return converted(&duration, zone),
		other => return Err(format!("`in` places a time in a zone, got {}", other.kind())),
	};
	time.in_zone(zone_argument("`in`", zone, scope)?).map(Value::Time)
}

/// The zone a word's argument names: "Europe/Berlin"
fn zone_argument(word: &str, zone: &Node, scope: &mut Scope) -> Result<&'static calendar::Zone, String> {
	match evaluate(zone, scope)? {
		Value::Text(name) | Value::Symbol(name) => calendar::zone_named(&name),
		other => Err(format!("{word} needs a zone name like \"Europe/Berlin\", got {}", other.kind())),
	}
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

/// Options of `add` and `zoned`: `overflow: clamp`, `disambiguation: earlier`; both reject by default
fn options(function: &str, options: &[Node], scope: &mut Scope) -> Result<(Overflow, Disambiguation), String> {
	let (mut overflow, mut choice) = (Overflow::Reject, Disambiguation::Reject);
	for option in options {
		let Node::Key(name, Op::Colon, value) = option.drop_meta() else {
			return Err(unsupported(option));
		};
		let mode = match evaluate(value, scope)? {
			Value::Symbol(mode) | Value::Text(mode) => mode,
			other => return Err(format!("{name} takes a word like reject, got {}", other.kind())),
		};
		match (name.drop_meta(), mode.as_str()) {
			(Node::Symbol(name), "clamp" | "constrain") if name == "overflow" && function == "add" => overflow = Overflow::Clamp,
			(Node::Symbol(name), "reject") if name == "overflow" && function == "add" => overflow = Overflow::Reject,
			(Node::Symbol(name), _) if name == "overflow" && function == "add" => return Err(format!("overflow is clamp or reject, got {mode}")),
			(Node::Symbol(name), "earlier") if name == "disambiguation" => choice = Disambiguation::Earlier,
			(Node::Symbol(name), "later") if name == "disambiguation" => choice = Disambiguation::Later,
			(Node::Symbol(name), "reject") if name == "disambiguation" => choice = Disambiguation::Reject,
			(Node::Symbol(name), _) if name == "disambiguation" => {
				return Err(format!("disambiguation is earlier, later or reject, got {mode}"))
			}
			_ => return Err(format!("{function} has no option {name}")),
		}
	}
	Ok((overflow, choice))
}

/// `add(2024-01-31, 1 month, overflow: clamp)`: calendar overflow spelled out,
/// `add(t, 1 day, disambiguation: later)` when the day lands on a repeated or skipped wall time
fn add(args: &[Node], scope: &mut Scope) -> Result<Value, String> {
	let (time, duration, rest) = match args {
		[time, duration, rest @ ..] => (evaluate(time, scope)?, evaluate(duration, scope)?, rest),
		_ => return Err("add(time, duration, overflow: clamp) takes a time and a duration".to_string()),
	};
	let (overflow, choice) = options("add", rest, scope)?;
	match (time, duration) {
		(Value::Time(time), Value::Duration(duration)) => time.add(duration, overflow, choice).map(Value::Time),
		(time, duration) => Err(format!("add needs a time and a duration, got {} and {}", time.kind(), duration.kind())),
	}
}

/// `zoned(2030-10-27T02:30, "Europe/Berlin", disambiguation: earlier)`: a local time in a zone, the choice spelled out
fn zoned(args: &[Node], scope: &mut Scope) -> Result<Value, String> {
	let [time, zone, rest @ ..] = args else {
		return Err("zoned(local time, \"Zone/Name\", disambiguation: earlier) takes a local time and a zone".to_string());
	};
	let zone = zone_argument("zoned", zone, scope)?;
	let (_, choice) = options("zoned", rest, scope)?;
	match evaluate(time, scope)? {
		Value::Time(Time::Local(date, clock)) => zone.resolve(date, clock, choice).map(|zoned| Value::Time(Time::Zoned(zoned))),
		Value::Time(time) => time.in_zone(zone).map(Value::Time),
		other => Err(format!("zoned needs a local time, got {}", other.kind())),
	}
}

fn binary(left: &Node, op: Op, right: &Node, scope: &mut Scope) -> Result<Value, String> {
	match op {
		Op::Define if getter_name(left).is_some() => {
			let getter = Value::Getter(right.clone());
			scope.insert(getter_name(left).expect("guarded").to_string(), getter.clone());
			Ok(getter)
		}
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
				// The time zone rules version a zoned time was resolved with
				Value::Time(Time::Zoned(zoned)) if field == "tzdata" => Ok(Value::Text(zoned.rules.version.to_string())),
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

fn apply(left: Value, op: Op, right: Value) -> Result<Value, String> {
	match (left, op, right) {
		(Value::Time(time), Op::Add, Value::Duration(duration)) | (Value::Duration(duration), Op::Add, Value::Time(time)) => {
			time.add(duration, Overflow::Reject, Disambiguation::Reject).map(Value::Time)
		}
		(Value::Time(time), Op::Sub, Value::Duration(duration)) => {
			time.add(duration.times(-1), Overflow::Reject, Disambiguation::Reject).map(Value::Time)
		}
		(Value::Time(later), Op::Sub, Value::Time(earlier)) => Ok(match later.since(earlier)? {
			Ok(days) => Value::Int(days),
			Err(duration) => Value::Duration(duration),
		}),
		(Value::Time(a), op, Value::Time(b)) if op.is_comparison() => Ok(Value::Bool(op.holds(a.compare(b)?))),
		(Value::Duration(a), Op::Add, Value::Duration(b)) => Ok(Value::Duration(a.plus(b))),
		(Value::Duration(a), Op::Sub, Value::Duration(b)) => Ok(Value::Duration(a.plus(b.times(-1)))),
		(Value::Duration(duration), Op::Mul, Value::Int(n)) | (Value::Int(n), Op::Mul, Value::Duration(duration)) => Ok(Value::Duration(duration.times(n))),
		(Value::Duration(a), Op::Eq, Value::Duration(b)) => a.equals(b).map(Value::Bool),
		(Value::Duration(a), Op::Ne, Value::Duration(b)) => a.equals(b).map(|equal| Value::Bool(!equal)),
		(Value::Int(a), op, Value::Int(b)) if op.is_comparison() => Ok(Value::Bool(op.holds(a.cmp(&b)))),
		(Value::Int(a), Op::Add, Value::Int(b)) => Ok(Value::Int(a + b)),
		(Value::Int(a), Op::Sub, Value::Int(b)) => Ok(Value::Int(a - b)),
		(Value::Int(a), Op::Mul, Value::Int(b)) => Ok(Value::Int(a * b)),
		(Value::Bool(a), Op::And, Value::Bool(b)) => Ok(Value::Bool(a && b)),
		(Value::Bool(a), Op::Or, Value::Bool(b)) => Ok(Value::Bool(a || b)),
		(Value::Bool(a), Op::Eq, Value::Bool(b)) => Ok(Value::Bool(a == b)),
		(Value::Bool(a), Op::Ne, Value::Bool(b)) => Ok(Value::Bool(a != b)),
		(Value::Duration(duration), Op::As, Value::Symbol(unit)) => converted(&duration, &Node::Symbol(unit)),
		(left, op, right) => Err(format!("cannot apply {op} to a {} and a {}", left.kind(), right.kind())),
	}
}

/// `2 hours in minutes`, `90 minutes as hours`: the duration in a time unit (P36)
fn converted(duration: &Duration, unit: &Node) -> Result<Value, String> {
	let Node::Symbol(word) = unit.drop_meta() else { return Err(format!("{duration} converts to a time unit, got {}", unit.serialize())) };
	match crate::units::duration_in(duration, word) {
		Some(Node::Error(message)) => Err(message.to_string()),
		Some(node) => Ok(Value::Converted(node)),
		None => Err(format!("{duration} converts to a time unit like minutes, got {word}")),
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
			Value::Converted(_) => "quantity",
			Value::Getter(_) => "getter",
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
			Value::Converted(node) => node,
			Value::Getter(_) => Node::Empty, // a definition as the last statement
		}
	}
}
