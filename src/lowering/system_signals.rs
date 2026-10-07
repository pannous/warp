//! Timers (notes/system_signals.md): `on every 5 seconds {body}` at the main level is the handler function
//! `on·every·0() := {global …; body}` and the call `signal_every(0, 5000)` where it is written, which starts the timer;
//! the runtime runs the handler at the program's check points (crates/warp-runtime/src/system_signals.rs) and, in
//! `warp run`, after main while the timer lives. The duration is constant (`50 ms`, `5 seconds`, `1 min`).
//! `on file "notes.txt" change {body}` likewise is `on·file·0() := {…}` and `signal_watch(0, "notes.txt")`.
//! `exit` and `exit()` as statements are `exit(0)` (P121: the host word ends the run with that code).
//! Channels (src/channels.rs): `on message from "chat" {body}` (`on message {body}` for the channel "warp") is the timer
//! handler `on·every·0() := {global …; while channel_pending(0) > 0 { event = channel_next(0); body }}`, started by
//! `channel_listen(0, "chat")` and `signal_every(0, 20)`, so a program listening stays like one with a timer;
//! `broadcast value on "chat"` (`broadcast value`) is `channel_send("chat", value)`, and so is `send value to "chat"`.
//! Named events across programs (P129b): `broadcast stop the machine{reason: "heat"} on "chat"` is
//! `channel_send("chat/stop the machine", {reason: "heat"})`, heard by `on stop the machine from "chat" {…}`, a message
//! listener of that sub-channel; `send "file system full"` alone is that named event on "warp" without data, heard by
//! `on "file system full" {…}`. `raise` stays inside the program (event_signals.rs).

use crate::declarations::word;
use crate::diagnostic::Diagnostic;
use crate::extensions::numbers::Number;
use crate::event_signals::{function_with_globals, main_level_variables};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const ON_WORD: &str = "on";
const EVERY_WORD: &str = "every";
const DAY_WORD: &str = "day";
const AT_WORD: &str = "at";
const WEEKDAY_WORD: &str = "weekday";
const WEEKEND_WORD: &str = "weekend";
/// `9am`, `9pm`: the half of the day by its index
const HALVES: [&str; 2] = ["am", "pm"];
const FILE_WORD: &str = "file";
const CHANGE_WORDS: [&str; 2] = ["change", "changes"];
const MESSAGE_WORD: &str = "message";
const FROM_WORD: &str = "from";
const BROADCAST_WORD: &str = "broadcast";
const SEND_WORD: &str = "send";
/// `chat/stop the machine`: the named event `stop the machine` on the channel `chat`, a channel of its own
pub const EVENT_SEPARATOR: char = '/';
/// The channel of `on message {…}` and `broadcast value` without a channel name
const DEFAULT_CHANNEL: &str = "warp";
/// How often a listening program looks for messages
const CHANNEL_CHECK_MILLISECONDS: i64 = 20;
/// The template of a message handler's body
const MESSAGES_TEMPLATE: &str = "while channel_pending(ID) > 0 { event = channel_next(ID); BODY }";

pub fn lower(program: Node) -> Node {
	let program = if defines(&program, crate::host::EXIT) { program } else { bare_exits(program) };
	let verbs: Vec<&str> = [BROADCAST_WORD, SEND_WORD].into_iter().filter(|verb| !defines(&program, verb)).collect();
	let program = broadcasts(program, &verbs);
	lower_timers(program)
}

fn lower_timers(program: Node) -> Node {
	let (statements, bracket, separator) = crate::variable_signals::main_statements(&program);
	if !statements.iter().any(|statement| one_shot(statement).is_some() || timer(statement).is_some() || file_watch(statement).is_some() || message_listener(statement).is_some()) {
		return program;
	}
	let main_variables = main_level_variables(&statements);
	let mut lowered = vec![];
	let (mut count, mut watches) = (0, 0);
	for statement in statements {
		if let Some((channel, body)) = message_listener(&statement) {
			let handler = format!("{}{count}", crate::host::TIMER_HANDLER_PREFIX);
			let id = Node::int(count as i64);
			let messages = crate::law::substitute(&crate::wasp_parser::parse(MESSAGES_TEMPLATE), &std::collections::HashMap::from([("ID".to_string(), id.clone()), ("BODY".to_string(), body)]));
			lowered.push(function_with_globals(&handler, false, &[messages.drop_meta().clone()], &main_variables));
			lowered.push(call(crate::host::CHANNEL_LISTEN, vec![id.clone(), channel]));
			lowered.push(call(crate::host::SIGNAL_EVERY, vec![id, Node::int(CHANNEL_CHECK_MILLISECONDS)]));
			count += 1;
			continue;
		}
		if let Some((path, body)) = file_watch(&statement) {
			let handler = format!("{}{watches}", crate::host::FILE_HANDLER_PREFIX);
			lowered.push(function_with_globals(&handler, false, &[body], &main_variables));
			let start = [Node::Symbol(crate::host::SIGNAL_WATCH.to_string()), Node::int(watches as i64), path];
			lowered.push(Node::List(start.to_vec(), Bracket::Round, Separator::None));
			watches += 1;
			continue;
		}
		if let Some((time, body)) = one_shot(&statement) {
			let Some(minute_of_day) = time else { return no_time_of_day(&statement, AT_WORD) };
			let handler = format!("{}{count}", crate::host::TIMER_HANDLER_PREFIX);
			lowered.push(function_with_globals(&handler, false, &[body], &main_variables));
			lowered.push(call(crate::host::SIGNAL_AT, vec![Node::int(count as i64), Node::int(minute_of_day)]));
			count += 1;
			continue;
		}
		let Some((duration, body)) = timer(&statement) else {
			lowered.push(statement);
			continue;
		};
		if let Some(schedule) = clock_schedule(&duration) {
			let (minute_of_day, weekdays) = match schedule {
				Ok(schedule) => schedule,
				Err(ClockError::Days(days)) => {
					return Diagnostic::at(&statement, format!("on every … at needs day, a weekday (monday … sunday), weekday or weekend, got {days}")).into_error();
				}
				Err(ClockError::Time) => return no_time_of_day(&statement, "on every day at"),
			};
			let handler = format!("{}{count}", crate::host::TIMER_HANDLER_PREFIX);
			lowered.push(function_with_globals(&handler, false, &[body], &main_variables));
			lowered.push(call(crate::host::SIGNAL_DAILY, vec![Node::int(count as i64), Node::int(minute_of_day), Node::int(weekdays)]));
			count += 1;
			continue;
		}
		let Some(milliseconds) = crate::units::milliseconds(&duration) else {
			return Diagnostic::at(&statement, format!("on every needs a constant duration like `on every 5 seconds {{…}}`, got {}", duration.serialize().trim())).into_error();
		};
		let handler = format!("{}{count}", crate::host::TIMER_HANDLER_PREFIX);
		lowered.push(function_with_globals(&handler, false, &[body], &main_variables));
		let start = [Node::Symbol(crate::host::SIGNAL_EVERY.to_string()), Node::int(count as i64), Node::int(milliseconds)];
		lowered.push(Node::List(start.to_vec(), Bracket::Round, Separator::None));
		count += 1;
	}
	Node::List(lowered, bracket, separator)
}

/// `on message {body}`, `on message from "chat" {body}`, and named events from other programs:
/// `on stop the machine from "chat" {body}`, `on "file system full" {body}`: the channel (a text) and the body
fn message_listener(statement: &Node) -> Option<(Node, Node)> {
	let Node::List(items, _, _) = statement.drop_meta() else { return None };
	let (on, rest) = items.split_first()?;
	if word(on) != ON_WORD {
		return None;
	}
	let (words, body) = words_and_body(rest)?;
	let (name, channel) = match words.as_slice() {
		[name @ .., from, channel] if word(from) == FROM_WORD && !name.is_empty() => (name, text(channel)?),
		name => (name, DEFAULT_CHANNEL.to_string()),
	};
	let from_written = name.len() < words.len();
	let channel = match name {
		[message] if word(message) == MESSAGE_WORD => channel,
		[event] if text(event).is_some() => event_channel(&channel, &text(event)?),
		words if from_written => event_channel(&channel, &event_name(words)?),
		_ => return None,
	};
	Some((Node::Text(channel), body))
}

fn text(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Text(text) => Some(text.clone()),
		_ => None,
	}
}

/// The words of an event joined by spaces; None unless all are plain words
fn event_name(words: &[Node]) -> Option<String> {
	let words: Vec<String> = words.iter().map(word).collect();
	(!words.is_empty() && words.iter().all(|word| !word.is_empty())).then(|| words.join(" "))
}

/// The channel of a named event: its own sub-channel, so a listener hears only that event
fn event_channel(channel: &str, event: &str) -> String {
	format!("{channel}{EVENT_SEPARATOR}{event}")
}

/// `stop the machine{reason: "heat"}`, `stop the machine`, `alarm{}`: a named event's name and data (ø without);
/// a single word without data stays a value (`broadcast total`)
fn named_event(value: &[Node]) -> Option<(String, Node)> {
	let (last, words) = value.split_last()?;
	let (last_word, data) = match last.drop_meta() {
		Node::Key(name, Op::Colon, data) if matches!(data.drop_meta(), Node::List(_, Bracket::Curly, _)) => (name.as_ref().clone(), Some(data.as_ref().clone())),
		_ => (last.clone(), None),
	};
	if words.is_empty() && data.is_none() {
		return None;
	}
	let name = event_name(&[words, &[last_word]].concat())?;
	Some((name, data.unwrap_or(Node::Empty)))
}

/// `broadcast value on "chat"`, `send value to "chat"`, `broadcast value`: `channel_send("chat", value)`; a named
/// event goes to its own sub-channel with its data, and `send "file system full"` alone is that named event
fn broadcasts(node: Node, verbs: &[&str]) -> Node {
	let sent = |items: &[Node]| -> Option<Node> {
		let (first, rest) = items.split_first()?;
		let verb = word(first);
		if !verbs.contains(&verb.as_str()) || rest.is_empty() {
			return None;
		}
		let (value, channel) = match (verb.as_str(), rest) {
			(BROADCAST_WORD, [value @ .., on, channel]) if word(on) == ON_WORD && !value.is_empty() => (value.to_vec(), Some(text(channel)?)),
			(SEND_WORD, [value @ .., last]) => match last.drop_meta() {
				Node::Key(left, Op::To, channel) => ([value, &[left.as_ref().clone()]].concat(), Some(text(channel)?)),
				// `send alarm{…}` without `to` is emit, inside the program (P163, event_signals.rs); `send "disk full"`
				// stays the named event of the machine (P129)
				_ if !matches!(rest, [name] if text(name).is_some()) => return None,
				_ => (rest.to_vec(), None),
			},
			_ => (rest.to_vec(), None),
		};
		let named = match value.as_slice() {
			[name] if verb == SEND_WORD && channel.is_none() => text(name).map(|name| (name, Node::Empty)),
			value => named_event(value),
		};
		let channel = channel.unwrap_or(DEFAULT_CHANNEL.to_string());
		let (channel, value) = match named {
			Some((name, data)) => (event_channel(&channel, &name), data),
			None => (channel, match value.as_slice() {
				[single] => single.clone(),
				several => Node::List(several.to_vec(), Bracket::None, Separator::Space),
			}),
		};
		Some(call(crate::host::CHANNEL_SEND, vec![Node::Text(channel), broadcasts(value, verbs)]))
	};
	match node {
		Node::List(items, _, _) if sent(&items).is_some() => sent(&items).expect("checked"),
		other => other.map_children(|child| broadcasts(child, verbs)),
	}
}

/// `function(arguments…)`
pub(crate) fn call(function: &str, arguments: Vec<Node>) -> Node {
	Node::List([vec![Node::Symbol(function.to_string())], arguments].concat(), Bracket::Round, Separator::None)
}

/// `on file "notes.txt" change {body}`: the path (a text) and the body
fn file_watch(statement: &Node) -> Option<(Node, Node)> {
	let Node::List(items, _, _) = statement.drop_meta() else { return None };
	let [on, file, path, change, body] = items.as_slice() else { return None };
	let watched = word(on) == ON_WORD && word(file) == FILE_WORD && CHANGE_WORDS.contains(&word(change).as_str());
	let path_is_text = matches!(path.drop_meta(), Node::Text(_));
	(watched && path_is_text && matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _))).then(|| (path.clone(), body.clone()))
}

/// `on every 50 ms {body}`, `on every 5 seconds: body`: the duration and the body
fn timer(statement: &Node) -> Option<(Node, Node)> {
	let Node::List(items, _, _) = statement.drop_meta() else { return None };
	let [on, every, rest @ ..] = items.as_slice() else { return None };
	if word(on) != ON_WORD || word(every) != EVERY_WORD {
		return None;
	}
	let (words, body) = words_and_body(rest)?;
	let duration = match words.as_slice() {
		[] => return None,
		[single] => single.clone(),
		several => Node::List(several.to_vec(), Bracket::None, Separator::Space),
	};
	Some((duration, body))
}

/// The words before the body of `… {body}` or `…: body`, and the body
fn words_and_body(rest: &[Node]) -> Option<(Vec<Node>, Node)> {
	let (last, words) = rest.split_last()?;
	if matches!(last.drop_meta(), Node::List(_, Bracket::Curly, _)) {
		return Some((words.to_vec(), last.clone()));
	}
	// `on every 5 seconds: print n` arrives as `on every (5 seconds: print) n`: the words after the colon are the body's
	let colon = rest.iter().position(|item| crate::declarations::handler_parts(item).is_some())?;
	let (end, body) = crate::declarations::handler_parts(&rest[colon])?;
	let body = match &rest[colon + 1..] {
		[] => body,
		after => Node::List([vec![body], after.to_vec()].concat(), Bracket::None, Separator::Space),
	};
	Some((rest[..colon].iter().cloned().chain([end]).collect(), body))
}

/// `at 9:00 {body}`, `at 9pm: body`: the minute of the day (None for a time that is no time of day) and the body;
/// None for any other use of `at`
fn one_shot(statement: &Node) -> Option<(Option<i64>, Node)> {
	let Node::List(items, _, _) = statement.drop_meta() else { return None };
	let (at, rest) = items.split_first()?;
	if word(at) != AT_WORD {
		return None;
	}
	let (words, body) = words_and_body(rest)?;
	Some((minute_of_day(&words)?, body))
}

fn no_time_of_day(statement: &Node, form: &str) -> Node {
	Diagnostic::at(statement, format!("{form} needs a time of day from 0:00 to 23:59 or 12am to 11:59pm")).into_error()
}

enum ClockError {
	Days(String),
	Time,
}

/// `day at 9:00` of `on every day at 9:00 {…}` (`monday at 9:30pm`, `weekday at 7am`): the minute of the day and the
/// weekdays (bit 0 Sunday); None when the duration has no `at`
fn clock_schedule(duration: &Node) -> Option<Result<(i64, i64), ClockError>> {
	let Node::List(words, _, _) = duration.drop_meta() else { return None };
	let [days, at, time @ ..] = words.as_slice() else { return None };
	if word(at) != AT_WORD {
		return None;
	}
	let Some(weekdays) = weekdays(&word(days)) else { return Some(Err(ClockError::Days(days.serialize().trim().to_string()))) };
	Some(minute_of_day(time).flatten().map(|minute| (minute, weekdays)).ok_or(ClockError::Time))
}

const WEEKDAY_NAMES: [&str; 7] = ["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"];
const EVERY_DAY: i64 = 0b111_1111;
const WORKDAYS: i64 = 0b011_1110;
const WEEKEND: i64 = 0b100_0001;

/// `day`, `monday`, `mondays`, `weekday`, `weekend`: the mask of weekdays (bit 0 Sunday)
fn weekdays(name: &str) -> Option<i64> {
	let name = name.to_lowercase();
	let singular = name.strip_suffix('s').unwrap_or(&name);
	match singular {
		DAY_WORD => Some(EVERY_DAY),
		WEEKDAY_WORD => Some(WORKDAYS),
		WEEKEND_WORD => Some(WEEKEND),
		_ => WEEKDAY_NAMES.iter().position(|day| *day == singular).map(|day| 1 << day),
	}
}

/// `9:00`, `21:30`, `9:30pm`, `9:30 pm`, `9pm`, `9 pm`: the minute of the day, Some(None) for a clock time that is
/// no time of day (`25:00`, `13pm`), None for words that are no clock time
fn minute_of_day(words: &[Node]) -> Option<Option<i64>> {
	let int = |node: &Node| match node.drop_meta() {
		Node::Number(Number::Int(value)) => Some(*value),
		_ => None,
	};
	let half = |node: &Node| HALVES.iter().position(|half| *half == word(node).to_lowercase());
	let with_half = |node: &Node| match node.drop_meta() {
		Node::Key(amount, Op::Mul, unit) => Some((int(amount)?, half(unit)?)),
		_ => None,
	};
	let hour_minute = |node: &Node| match node.drop_meta() {
		Node::Key(hour, Op::Colon, minute) => {
			let hour = int(hour)?;
			int(minute).map(|minute| (hour, minute, None)).or_else(|| with_half(minute).map(|(minute, half)| (hour, minute, Some(half))))
		}
		_ => None,
	};
	let (hour, minute, half) = match words {
		[time] => hour_minute(time).or_else(|| with_half(time).map(|(hour, half)| (hour, 0, Some(half))))?,
		[time, spoken_half] => {
			let half = Some(half(spoken_half)?);
			hour_minute(time).filter(|(_, _, written)| written.is_none()).map(|(hour, minute, _)| (hour, minute, half)).or_else(|| Some((int(time)?, 0, half)))?
		}
		_ => return None,
	};
	let hour = match half {
		None => Some(hour).filter(|hour| (0..24).contains(hour)),
		Some(half) => Some(hour).filter(|hour| (1..=12).contains(hour)).map(|hour| hour % 12 + 12 * half as i64),
	};
	Some(hour.filter(|_| (0..60).contains(&minute)).map(|hour| hour * 60 + minute))
}

/// `exit` or `exit()` as a statement: `exit(0)`
fn bare_exits(node: Node) -> Node {
	let is_exit = |item: &Node| match item.drop_meta() {
		Node::Symbol(name) => name == crate::host::EXIT,
		Node::List(items, Bracket::Round, _) => items.len() == 1 && word(&items[0]) == crate::host::EXIT,
		_ => false,
	};
	let exit_zero = || Node::List(vec![Node::Symbol(crate::host::EXIT.to_string()), Node::int(0)], Bracket::Round, Separator::None);
	match node {
		Node::List(items, bracket, separator) if crate::variable_signals::is_statement_list(&bracket, &separator) || bracket == Bracket::Curly => {
			Node::List(items.into_iter().map(|item| if is_exit(&item) { exit_zero() } else { bare_exits(item) }).collect(), bracket, separator)
		}
		other => other.map_children(bare_exits),
	}
}

/// Does the program define or assign the name itself (`exit := …`, `def exit(…)`, `exit = …`)
fn defines(program: &Node, name: &str) -> bool {
	let mut defined = false;
	program.visit(&mut |part| if let Node::Key(target, op, _) = part {
		let head = match target.drop_meta() {
			Node::List(items, _, _) => items.first().map(word).unwrap_or_default(),
			other => word(other),
		};
		defined |= head == name && matches!(op, crate::operators::Op::Assign | crate::operators::Op::Define);
	});
	defined
}
