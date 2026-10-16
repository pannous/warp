//! Dates and times (Footguns.md → Dates and time zones, notes/dates_at_run_time.md).
//! The parser lexes RFC 3339 / RFC 9557 literals and `1 month` style durations into data nodes. A program this
//! evaluator answers whole is answered here at compile time; in any other its constant time expressions fold to
//! `time_of(form, position)`, a Kind::Time node built when the program runs (wasm_emitter/times.rs), and `now` is
//! `instant_at(clock())` (system_values).

pub mod calendar;
pub mod partial_comparisons;

use crate::extensions::numbers::Number;
use crate::node::{error, Bracket, Node, Separator};
use crate::operators::Op;
use crate::wasm_emitter::time::time_zones::{EARLIER, LATER, REJECT};
pub use calendar::{with_rules, Disambiguation, Duration, Overflow, Time, TzRules, Zone, Zoned};
use std::collections::{HashMap, HashSet};

pub const NOW_WORD: &str = "now";
/// instant_at(milliseconds since 1970): the instant node `now` is at run time
pub const INSTANT_AT: &str = "instant_at";

/// time_of(form, position): a time constant at run time, the TimeForm and the calendar::Time position
pub const TIME_OF: &str = "time_of";
/// time_in_zone(time, zone index, choice): `t in "Europe/Berlin"` of a time only the run knows
pub use crate::wasm_emitter::time::time_zones::TIME_IN_ZONE;

/// The calls that build a time node at run time
pub const TIME_CALLS: [&str; 3] = [INSTANT_AT, TIME_OF, TIME_IN_ZONE];

/// `instant_at(clock)`, `time_of(…)`, `time_in_zone(…)`: a time the run builds
pub fn is_time_call(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, ..) if items.first().and_then(Node::symbol_name).is_some_and(|head| TIME_CALLS.contains(&head)))
}
/// A time call or arithmetic on one: `now`, `time_of(…) * n`, `d + 1 day` as the run builds it
pub fn is_time_expression(node: &Node) -> bool {
	holds_time(node, &HashSet::new())
}

/// A time expression, or arithmetic on a variable of `times` (time_variables): `x + 1 hour` of `x = now`
pub fn holds_time(node: &Node, times: &HashSet<String>) -> bool {
	match node.drop_meta() {
		Node::Key(left, op, right) if op.is_arithmetic() => holds_time(left, times) || holds_time(right, times),
		Node::List(items, Bracket::Round, _) if items.len() == 1 => holds_time(&items[0], times),
		Node::Symbol(name) => times.contains(name),
		_ => is_time_call(node),
	}
}

/// The variables every assignment gives a time: `x = now`, `later = x + 1 hour` (card variable-time)
pub fn time_variables(program: &Node) -> HashSet<String> {
	let mut times = HashSet::new();
	let mut others = HashSet::new();
	program.visit(&mut |part| {
		if let Node::Key(target, Op::Assign | Op::Define, value) = part {
			if let Node::Symbol(name) = target.drop_meta() {
				let assigned = if holds_time(value, &times) { &mut times } else { &mut others };
				assigned.insert(name.clone());
			}
		}
	});
	times.retain(|name| !others.contains(name));
	times
}
/// The one field of an instant that needs no time zone
pub const EPOCH_SECONDS: &str = "epoch_seconds";

/// The form of a run-time time node, in the info bits above Kind::Time; its $i64box holds the position: days since
/// 1970 for a date, nanoseconds since 1970 for the others (a local time as if it were UTC), the exact nanoseconds of a
/// duration. Above the form, a duration keeps its days and months, a zoned time its zone and offset (`time_info`)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimeForm {
	Instant = 0,
	Date = 1,
	Local = 2,
	Duration = 3,
	Zoned = 4,
	/// `now.time`: nanoseconds since midnight
	Clock = 5,
}
pub const TIME_FORMS: [TimeForm; 6] =
	[TimeForm::Instant, TimeForm::Date, TimeForm::Local, TimeForm::Duration, TimeForm::Zoned, TimeForm::Clock];
/// The info bits of the form
pub const TIME_FORM_MASK: i64 = 7;
/// The type of every time, covering the type of each form (card anchor-time): `now is time`, `now is instant`
pub const TIME_WORD: &str = "time";
pub const TIME_FORM_BITS: i64 = 3;
/// A duration's months above its days, which take 32 bits
pub const MONTHS_SHIFT: i64 = TIME_FORM_BITS + 32;
/// A zoned time's index in calendar::ZONES, then its offset in minutes east of UTC
pub const ZONE_INDEX_MASK: i64 = 0xff;
pub const OFFSET_SHIFT: i64 = TIME_FORM_BITS + 8;

impl TimeForm {
	/// What `type(t)` says of a time of this form, wiki/time.md's words; each a type under TIME_WORD
	pub const fn name(self) -> &'static str {
		match self {
			TimeForm::Instant => "instant",
			TimeForm::Date => "date",
			TimeForm::Local => "local time",
			TimeForm::Duration => "duration",
			TimeForm::Zoned => "zoned time",
			TimeForm::Clock => calendar::CLOCK_KIND,
		}
	}

	/// Why a time of this form has no field or part `name`, as the compile-time evaluator says it
	pub fn refusal(self, name: &str) -> Option<String> {
		let example = match self {
			TimeForm::Instant => Time::Instant(0),
			TimeForm::Date => Time::Date(calendar::civil_from_days(0)),
			TimeForm::Local => {
				let (date, clock) = calendar::from_wall_nanos(0);
				Time::Local(date, clock)
			}
			TimeForm::Duration => return Duration::default().field(name).err(),
			TimeForm::Zoned => Time::Zoned(Zoned::at(0, &calendar::ZONES[0])),
			TimeForm::Clock => Time::Clock(calendar::from_wall_nanos(0).1),
		};
		match example.part(name) {
			Some(part) => part.err(),
			None => example.field(name).err(),
		}
	}
}

/// The info bits and the position of a time or duration at run time (TimeForm)
fn time_info(value: &Value) -> Option<(i64, i64)> {
	match value {
		Value::Time(Time::Instant(instant)) => Some((TimeForm::Instant as i64, *instant as i64)),
		Value::Time(time @ Time::Date(_)) => Some((TimeForm::Date as i64, time.position() as i64)),
		Value::Time(time @ Time::Local(..)) => Some((TimeForm::Local as i64, time.position() as i64)),
		Value::Time(time @ Time::Clock(_)) => Some((TimeForm::Clock as i64, time.position() as i64)),
		Value::Time(Time::Zoned(zoned)) => {
			let index = calendar::ZONES.iter().position(|zone| zone.name == zoned.zone.name)? as i64;
			Some((
				TimeForm::Zoned as i64 | index << TIME_FORM_BITS | (zoned.offset / calendar::MINUTE) << OFFSET_SHIFT,
				zoned.instant() as i64,
			))
		}
		Value::Duration(duration) => {
			let days = (duration.days as u32 as i64) << TIME_FORM_BITS;
			Some((TimeForm::Duration as i64 | days | duration.months << MONTHS_SHIFT, duration.nanos as i64))
		}
		_ => None,
	}
}

/// Seconds east of UTC in the environment's time zone at an instant: an instant's wall clock (the host word
/// local_offset at run time). UTC without a host
pub fn local_offset(instant: calendar::Instant) -> i64 {
	#[cfg(feature = "native")]
	return warp_runtime::system_signals::local_offset_at(instant.div_euclid(calendar::SECOND / 1000) as i64);
	#[allow(unreachable_code)]
	{
		let _ = instant;
		0
	}
}

/// `now` or an instant constant: its fields need the host's local_offset (wasm_emitter/times.rs time_field)
pub fn makes_instant(items: &[Node]) -> bool {
	matches!(items.first(), Some(head) if head.is_symbol(INSTANT_AT)) || constant_form(items) == Some(TimeForm::Instant)
}

/// The form of a constant `time_of(info, position)`
pub fn constant_form(items: &[Node]) -> Option<TimeForm> {
	match items {
		[head, info, _] if head.is_symbol(TIME_OF) => match info.drop_meta() {
			Node::Number(Number::Int(info)) => form_of(*info),
			_ => None,
		},
		_ => None,
	}
}

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
}

fn form_of(info: i64) -> Option<TimeForm> {
	TIME_FORMS.into_iter().find(|form| *form as i64 == info & TIME_FORM_MASK)
}

/// A time read back from the run-time node (Kind::Time): its info bits (TimeForm) and position
pub fn time_node(info: i64, position: Option<Number>) -> Node {
	let Some(Number::Int(position)) = position else {
		return error("unreadable time");
	};
	let position = position as i128;
	match form_of(info) {
		Some(TimeForm::Instant) => Node::data(Time::Instant(position)),
		Some(TimeForm::Date) => Node::data(Time::Date(calendar::civil_from_days(position as i64))),
		Some(TimeForm::Local) => {
			let (date, clock) = calendar::from_wall_nanos(position);
			Node::data(Time::Local(date, clock))
		}
		Some(TimeForm::Clock) => Node::data(Time::Clock(calendar::from_wall_nanos(position).1)),
		Some(TimeForm::Duration) => {
			let days = (info >> TIME_FORM_BITS) as i32 as i64;
			Node::data(Duration { months: info >> MONTHS_SHIFT, days, nanos: position })
		}
		Some(TimeForm::Zoned) => match calendar::ZONES.get(((info >> TIME_FORM_BITS) & ZONE_INDEX_MASK) as usize) {
			Some(zone) => {
				let offset = (info >> OFFSET_SHIFT) * calendar::MINUTE;
				let (date, clock) = calendar::local_at(position, offset);
				Node::data(Time::Zoned(Zoned { date, clock, offset, zone, rules: calendar::rules() }))
			}
			None => error("unreadable time zone"),
		},
		None => error("unreadable time"),
	}
}

/// A zoned time's field: the version of the zone rules it was made with
pub const TZDATA_FIELD: &str = "tzdata";

/// A program that uses dates or times, its constant time expressions folded for the run time (stage B4: no
/// program is answered whole at compile time any more)
pub fn lower(program: Node) -> Node {
	if !mentions_time(&program) {
		return program;
	}
	let times = temporal_variables(&program);
	let program = with_unit_durations(program, &times);
	if !needs_folding(&program) {
		return program;
	}
	let constants = time_constants(&program);
	fold_constants(program, &constants)
}

/// The variables a time or duration is assigned: `d = 1 day`, `t = now + 1 hour`
fn temporal_variables(program: &Node) -> HashSet<String> {
	let mut times = time_variables(program);
	program.visit(&mut |part| {
		if let Node::Key(target, Op::Assign | Op::Define, value) = part {
			if let (Node::Symbol(name), true) = (target.drop_meta(), mentions_time(value)) {
				times.insert(name.clone());
			}
		}
	});
	times
}

/// `2 hours + 1 h`, `now + 90 min`, `1 h == 60 minutes`: an amount of a time unit beside a time or a duration is the
/// exact duration it is (decision duration-units); elsewhere it stays a quantity of the units
fn with_unit_durations(node: Node, times: &HashSet<String>) -> Node {
	let node = node.map_children(|child| with_unit_durations(child, times));
	let Node::Key(left, op, right) = node.drop_meta() else {
		return node;
	};
	if !(op.is_arithmetic() || op.is_comparison()) {
		return node;
	}
	// `2 hours in minutes == 120 min` compares quantities
	let temporal = |side: &Node| (mentions_time(side) || holds_time(side, times)) && !is_unit_conversion(side);
	let as_duration = |side: &Node, other: &Node| match crate::numbers::units::duration_quantity(side) {
		Some(duration) if temporal(other) => Node::data(duration),
		_ => side.clone(),
	};
	let (new_left, new_right) = (as_duration(left, right), as_duration(right, left));
	if new_left == **left && new_right == **right {
		return node;
	}
	Node::Key(Box::new(new_left), *op, Box::new(new_right))
}

/// The variables assigned a constant time expression once and never a parameter: `d = 1 day` (card
/// duration-var-in-unit), so `d in h` converts at compile time like `1 day in h`
fn time_constants(program: &Node) -> HashMap<String, Node> {
	let mut assigned: HashMap<String, Vec<Node>> = HashMap::new();
	let mut parameters = HashSet::new();
	program.visit(&mut |part| match part {
		Node::Key(head, Op::Define, _) if matches!(head.drop_meta(), Node::List(..)) => head.visit(&mut |word| {
			if let Node::Symbol(name) = word {
				parameters.insert(name.clone());
			}
		}),
		Node::Key(target, op, value) if matches!(op, Op::Assign | Op::Define) || op.is_compound_assign() => {
			if let Node::Symbol(name) = target.drop_meta() {
				assigned.entry(name.clone()).or_default().push(value.as_ref().clone());
			}
		}
		_ => {}
	});
	assigned
		.into_iter()
		.filter(|(name, values)| !parameters.contains(name) && values.len() == 1)
		.filter_map(|(name, mut values)| {
			let value = values.remove(0);
			(mentions_time(&value) && only_time_words(&value)).then_some((name, value))
		})
		.collect()
}

/// `d in h` of a time constant d: `<its value> in h`
fn with_constant_subject(node: Node, constants: &HashMap<String, Node>) -> Node {
	match node {
		Node::List(mut items, bracket, separator) if converted_to_unit(&items) => {
			if let Node::Symbol(name) = items[0].drop_meta() {
				if let Some(value) = constants.get(name) {
					items[0] = value.clone();
				}
			}
			Node::List(items, bracket, separator)
		}
		Node::Key(subject, Op::As, unit) if matches!(subject.drop_meta(), Node::Symbol(name) if constants.contains_key(name)) => {
			Node::Key(Box::new(constants[&subject.name()].clone()), Op::As, unit)
		}
		other => other,
	}
}

/// The largest expressions of time literals and time words alone, folded: what remains runs
fn fold_constants(node: Node, constants: &HashMap<String, Node>) -> Node {
	let node = with_constant_subject(node, constants);
	if mentions_time(&node) && only_time_words(&node) {
		match evaluate(&node) {
			// a quantity, `2 hours in minutes`: written as `120min` for the units' lowering
			Ok(Value::Converted(quantity)) if !matches!(quantity, Node::Error(_)) => {
				return crate::warp_parser::parse(&quantity.serialize());
			}
			Ok(value) => return run_time_constant(value),
			Err(message) if !message.starts_with(UNSUPPORTED) => return error(&message),
			Err(_) => {}
		}
	}
	match node {
		Node::Data(_) if time_data(&node) => not_at_run_time_yet(&node.serialize()),
		// `(t in "UTC").year`: the group around a placement is its call's own
		Node::List(mut items, Bracket::Round, _) if items.len() == 1 && zone_placement(&items[0]).is_some() => {
			fold_constants(items.remove(0), constants)
		}
		Node::List(..) if zone_placement(&node).is_some() => {
			let (time, zone, choice) = zone_placement(&node).expect("guarded");
			let number = |n: i64| Node::Number(Number::Int(n));
			crate::lowering::nodes::call(
				TIME_IN_ZONE,
				vec![fold_constants(time, constants), number(zone), number(choice)],
			)
		}
		other => other.map_children(|child| fold_constants(child, constants)),
	}
}

fn run_time_constant(value: Value) -> Node {
	match time_info(&value) {
		Some((info, position)) => {
			let number = |n: i64| Node::Number(Number::Int(n));
			crate::lowering::nodes::call(TIME_OF, vec![number(info), number(position)])
		}
		None if matches!(value, Value::Time(_) | Value::Duration(_)) => {
			not_at_run_time_yet(&value.into_node().serialize())
		}
		None => value.into_node(),
	}
}

fn not_at_run_time_yet(written: &str) -> Node {
	error(&format!("{written}: this time in a program that runs is not supported yet (card run-time-dates)"))
}

/// Words a constant time expression may hold: no variable, which only the run knows
const TIME_WORDS: [&str; 12] = [
	"in",
	"as",
	"date",
	"add",
	"zoned",
	"overflow",
	"disambiguation",
	"clamp",
	"constrain",
	"reject",
	"earlier",
	"later",
];

fn is_time_word(word: &str) -> bool {
	TIME_WORDS.contains(&word) || calendar::unit_named(word).is_some()
}

fn only_time_words(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(word) => is_time_word(word),
		// a field name: `2024-02-29.month`
		Node::Key(owner, Op::Dot, field) if matches!(field.drop_meta(), Node::Symbol(_)) => only_time_words(owner),
		Node::Key(left, Op::As, unit) if matches!(unit.drop_meta(), Node::Symbol(word) if crate::numbers::units::names_unit(word)) => {
			only_time_words(left)
		}
		Node::Key(left, _, right) => only_time_words(left) && only_time_words(right),
		// a duration in a unit of the units module: `1 day in h`
		Node::List(items, _, _) if converted_to_unit(items) => only_time_words(&items[0]),
		Node::List(items, _, _) => items.iter().all(only_time_words),
		Node::Number(_) | Node::Text(_) | Node::Data(_) => true,
		_ => false,
	}
}

/// `2 hours in minutes`, `d as h`: a duration converted to a unit, a quantity
fn is_unit_conversion(node: &Node) -> bool {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 1 => is_unit_conversion(&items[0]),
		Node::List(items, _, _) => converted_to_unit(items),
		Node::Key(_, Op::As, unit) => {
			matches!(unit.drop_meta(), Node::Symbol(word) if crate::numbers::units::names_unit(word))
		}
		_ => false,
	}
}

fn converted_to_unit(items: &[Node]) -> bool {
	match items {
		[_, keyword, unit] => {
			(keyword.is_symbol("in") || keyword.is_symbol("as"))
				&& matches!(unit.drop_meta(), Node::Symbol(word) if crate::numbers::units::names_unit(word) || is_time_word(word))
		}
		_ => false,
	}
}

fn time_data(node: &Node) -> bool {
	match node {
		Node::Data(data) => data.downcast_ref::<TimeLiteral>().is_some() || data.downcast_ref::<Duration>().is_some(),
		_ => false,
	}
}

/// What only this compile-time evaluator knows so far: literals, durations, `date(…)`, `t in "Europe/Berlin"`;
/// a program with none of them reads `now` at run time
fn needs_folding(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= time_data(part) || part.is_call_of("date") || places_in_zone(part));
	found
}

fn places_in_zone(node: &Node) -> bool {
	zone_placement(node).is_some()
}

/// `t in "Europe/Berlin"` or `zoned(t, "Europe/Berlin", disambiguation: later)` of a named zone: the time, the zone's
/// index in calendar::ZONES and the choice (wasm_emitter/time_zones.rs REJECT, EARLIER, LATER)
fn zone_placement(node: &Node) -> Option<(Node, i64, i64)> {
	let Node::List(items, _, separator) = node.drop_meta() else {
		return None;
	};
	let (time, zone, choices) = match items.as_slice() {
		[time, keyword, zone] if keyword.is_symbol("in") => (time, zone, &[][..]),
		[head, time, zone, choices @ ..] if *separator == Separator::None && head.is_symbol("zoned") => {
			(time, zone, choices)
		}
		_ => return None,
	};
	let Node::Text(name) = zone.drop_meta() else {
		return None;
	};
	let index = calendar::ZONES.iter().position(|known| known.name == *name)?;
	let choice = match choices {
		[] => REJECT,
		[option] => match option.drop_meta() {
			Node::Key(key, Op::Colon, mode) if key.drop_meta().is_symbol("disambiguation") => match mode.drop_meta() {
				Node::Symbol(mode) if mode == "earlier" => EARLIER,
				Node::Symbol(mode) if mode == "later" => LATER,
				Node::Symbol(mode) if mode == "reject" => REJECT,
				_ => return None,
			},
			_ => return None,
		},
		_ => return None,
	};
	Some((time.clone(), index as i64, choice))
}

/// Holds a date, time or duration of the time module (a key or member name aside)
pub(crate) fn mentions_time(node: &Node) -> bool {
	match node.drop_meta() {
		// a key or member name (`{date: 5}`, `x.date`) is no use of the time
		Node::Key(key, Op::Colon, value) if matches!(key.drop_meta(), Node::Symbol(_)) => mentions_time(value),
		Node::Key(owner, Op::Dot, field) if matches!(field.drop_meta(), Node::Symbol(_)) => mentions_time(owner),
		Node::Key(left, _, right) => mentions_time(left) || mentions_time(right),
		Node::List(items, _, _) => {
			items.first().is_some_and(|head| head.is_symbol("date") || head.is_symbol(INSTANT_AT))
				|| items.iter().any(mentions_time)
		}
		other => time_data(other),
	}
}

/// The start of what the evaluator does not know: the run time takes those programs
const UNSUPPORTED: &str = "not evaluated at compile time:";

fn not_evaluated(node: &Node) -> String {
	format!("{UNSUPPORTED} `{node}`")
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

fn evaluate(node: &Node) -> Result<Value, String> {
	match node.drop_meta() {
		Node::Data(data) => {
			if let Some(TimeLiteral(text)) = data.downcast_ref::<TimeLiteral>() {
				calendar::parse_literal(text).map(Value::Time)
			} else if let Some(duration) = data.downcast_ref::<Duration>() {
				Ok(Value::Duration(*duration))
			} else {
				Err(not_evaluated(node))
			}
		}
		Node::Number(Number::Int(n)) => Ok(Value::Int(*n)),
		Node::Text(text) => Ok(Value::Text(text.clone())),
		// a unit word only where only_time_words lets one stand: `1 day as h`
		Node::Symbol(name) if is_time_word(name) || crate::numbers::units::names_unit(name) => {
			Ok(Value::Symbol(name.clone()))
		}
		Node::Key(left, op, right) => binary(left, *op, right),
		Node::List(items, _, separator) => list(node, items, separator.clone()),
		_ => Err(not_evaluated(node)),
	}
}

fn list(node: &Node, items: &[Node], separator: Separator) -> Result<Value, String> {
	match items {
		[head, ..] if head.is_symbol(INSTANT_AT) => Ok(Value::Time(now())),
		[head, ..] if head.is_symbol(TIME_OF) => Err(not_evaluated(node)),
		[single] => evaluate(single),
		[time, keyword, zone] if keyword.is_symbol("in") => place_in_zone(time, zone),
		[head, args @ ..] if separator == Separator::None && head.is_symbol("date") => construct_date(args),
		[head, args @ ..] if separator == Separator::None && head.is_symbol("add") => add(args),
		[head, args @ ..] if separator == Separator::None && head.is_symbol("zoned") => zoned(args),
		_ => Err(not_evaluated(node)),
	}
}

/// `t in "Europe/Berlin"`: the only way an instant gets a wall clock
fn place_in_zone(time: &Node, zone: &Node) -> Result<Value, String> {
	let time = match evaluate(time)? {
		Value::Time(time) => time,
		Value::Duration(duration) => return converted(&duration, zone),
		other => return Err(format!("`in` places a time in a zone, got {}", other.kind())),
	};
	time.in_zone(zone_argument("`in`", zone)?).map(Value::Time)
}

/// The zone a word's argument names: "Europe/Berlin"
fn zone_argument(word: &str, zone: &Node) -> Result<&'static calendar::Zone, String> {
	match evaluate(zone)? {
		Value::Text(name) | Value::Symbol(name) => calendar::zone_named(&name),
		other => Err(format!("{word} needs a zone name like \"Europe/Berlin\", got {}", other.kind())),
	}
}

fn int_argument(node: &Node) -> Result<i64, String> {
	match evaluate(node)? {
		Value::Int(n) => Ok(n),
		other => Err(format!("expected an integer, got {}", other.kind())),
	}
}

/// `date(2024,2,29)`: fields are validated, never rolled over
fn construct_date(args: &[Node]) -> Result<Value, String> {
	let [year, month, day] = args else {
		return Err(format!("date(year, month, day) takes 3 arguments, got {}", args.len()));
	};
	let date = calendar::Date::new(int_argument(year)?, int_argument(month)?, int_argument(day)?)?;
	Ok(Value::Time(Time::Date(date)))
}

/// Options of `add` and `zoned`: `overflow: clamp`, `disambiguation: earlier`; both reject by default
fn options(function: &str, options: &[Node]) -> Result<(Overflow, Disambiguation), String> {
	let (mut overflow, mut choice) = (Overflow::Reject, Disambiguation::Reject);
	for option in options {
		let Node::Key(name, Op::Colon, value) = option.drop_meta() else {
			return Err(not_evaluated(option));
		};
		let mode = match evaluate(value)? {
			Value::Symbol(mode) | Value::Text(mode) => mode,
			other => return Err(format!("{name} takes a word like reject, got {}", other.kind())),
		};
		match (name.drop_meta(), mode.as_str()) {
			(Node::Symbol(name), "clamp" | "constrain") if name == "overflow" && function == "add" => {
				overflow = Overflow::Clamp
			}
			(Node::Symbol(name), "reject") if name == "overflow" && function == "add" => overflow = Overflow::Reject,
			(Node::Symbol(name), _) if name == "overflow" && function == "add" => {
				return Err(format!("overflow is clamp or reject, got {mode}"));
			}
			(Node::Symbol(name), "earlier") if name == "disambiguation" => choice = Disambiguation::Earlier,
			(Node::Symbol(name), "later") if name == "disambiguation" => choice = Disambiguation::Later,
			(Node::Symbol(name), "reject") if name == "disambiguation" => choice = Disambiguation::Reject,
			(Node::Symbol(name), _) if name == "disambiguation" => {
				return Err(format!("disambiguation is earlier, later or reject, got {mode}"));
			}
			_ => return Err(format!("{function} has no option {name}")),
		}
	}
	Ok((overflow, choice))
}

/// `add(2024-01-31, 1 month, overflow: clamp)`: calendar overflow spelled out,
/// `add(t, 1 day, disambiguation: later)` when the day lands on a repeated or skipped wall time
fn add(args: &[Node]) -> Result<Value, String> {
	let (time, duration, rest) = match args {
		[time, duration, rest @ ..] => (evaluate(time)?, evaluate(duration)?, rest),
		_ => return Err("add(time, duration, overflow: clamp) takes a time and a duration".to_string()),
	};
	let (overflow, choice) = options("add", rest)?;
	match (time, duration) {
		(Value::Time(time), Value::Duration(duration)) => time.add(duration, overflow, choice).map(Value::Time),
		(time, duration) => {
			Err(format!("add needs a time and a duration, got {} and {}", time.kind(), duration.kind()))
		}
	}
}

/// `zoned(2030-10-27T02:30, "Europe/Berlin", disambiguation: earlier)`: a local time in a zone, the choice spelled out
fn zoned(args: &[Node]) -> Result<Value, String> {
	let [time, zone, rest @ ..] = args else {
		return Err(
			"zoned(local time, \"Zone/Name\", disambiguation: earlier) takes a local time and a zone".to_string()
		);
	};
	let zone = zone_argument("zoned", zone)?;
	let (_, choice) = options("zoned", rest)?;
	match evaluate(time)? {
		Value::Time(Time::Local(date, clock)) => {
			zone.resolve(date, clock, choice).map(|zoned| Value::Time(Time::Zoned(zoned)))
		}
		Value::Time(time) => time.in_zone(zone).map(Value::Time),
		other => Err(format!("zoned needs a local time, got {}", other.kind())),
	}
}

fn binary(left: &Node, op: Op, right: &Node) -> Result<Value, String> {
	match op {
		Op::Dot => {
			let Node::Symbol(field) = right.drop_meta() else {
				return Err(not_evaluated(right));
			};
			match evaluate(left)? {
				// The time zone rules version a zoned time was resolved with
				Value::Time(Time::Zoned(zoned)) if field == TZDATA_FIELD => {
					Ok(Value::Text(zoned.rules.version.to_string()))
				}
				// the wall clock of an instant is the one where the program runs
				Value::Time(Time::Instant(_)) if field != EPOCH_SECONDS => Err(not_evaluated(right)),
				Value::Time(time) => match time.part(field) {
					Some(part) => part.map(Value::Time),
					None => time.field(field).map(Value::Int),
				},
				Value::Duration(duration) => duration.field(field).map(Value::Int),
				other => Err(format!("{} has no field {field}", other.kind())),
			}
		}
		_ => {
			let left = evaluate(left)?;
			let right = evaluate(right)?;
			apply(left, op, right)
		}
	}
}

fn apply(left: Value, op: Op, right: Value) -> Result<Value, String> {
	match (left, op, right) {
		(Value::Time(time), Op::Add, Value::Duration(duration))
		| (Value::Duration(duration), Op::Add, Value::Time(time)) => {
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
		(Value::Duration(duration), Op::Mul, Value::Int(n)) | (Value::Int(n), Op::Mul, Value::Duration(duration)) => {
			Ok(Value::Duration(duration.times(n)))
		}
		(Value::Duration(a), op, Value::Duration(b)) if op.is_comparison() => a.compares(op, b).map(Value::Bool),
		(Value::Duration(duration), Op::As, Value::Symbol(unit)) => converted(&duration, &Node::Symbol(unit)),
		// `"on " + 2024-02-29`: a text joins the other's text, as list_join does at run time
		(left, Op::Add, right) if matches!(left, Value::Text(_)) || matches!(right, Value::Text(_)) => {
			match (joined_text(&left), joined_text(&right)) {
				(Some(left), Some(right)) => Ok(Value::Text(left + &right)),
				_ => Err(format!("cannot apply + to a {} and a {}", left.kind(), right.kind())),
			}
		}
		// `d is date` tests a type (card anchor-time), lowering's type tests answer it
		(_, Op::Eq | Op::Ne, Value::Symbol(_)) => Err(UNSUPPORTED.to_string()),
		// numbers and truths the run computes, the times in them folded
		(left, _, right) if !left.is_temporal() && !right.is_temporal() => Err(UNSUPPORTED.to_string()),
		(left, op, right) => Err(format!("cannot apply {op} to a {} and a {}", left.kind(), right.kind())),
	}
}

fn joined_text(value: &Value) -> Option<String> {
	match value {
		Value::Text(text) => Some(text.clone()),
		Value::Time(time) => Some(time.to_string()),
		Value::Duration(duration) => Some(duration.to_string()),
		Value::Int(number) => Some(number.to_string()),
		Value::Converted(quantity) => Some(quantity.serialize().trim().to_string()),
		_ => None,
	}
}

/// `2 hours in minutes`, `90 minutes as hours`: the duration in a time unit (P36)
fn converted(duration: &Duration, unit: &Node) -> Result<Value, String> {
	let Node::Symbol(word) = unit.drop_meta() else {
		return Err(format!("{duration} converts to a time unit, got {}", unit.serialize()));
	};
	match crate::numbers::units::duration_in(duration, word) {
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
		}
	}

	fn is_temporal(&self) -> bool {
		matches!(self, Value::Time(_) | Value::Duration(_))
	}

	fn into_node(self) -> Node {
		match self {
			Value::Time(time) => Node::data(time),
			Value::Duration(duration) => Node::data(duration),
			Value::Int(n) => Node::Number(Number::Int(n)),
			Value::Bool(true) => Node::True,
			Value::Bool(false) => Node::False,
			Value::Text(text) => Node::Text(text),
			Value::Symbol(name) => Node::Symbol(name),
			Value::Converted(node) => node,
		}
	}
}
