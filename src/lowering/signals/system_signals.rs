//! Timers (notes/system_signals.md): `on every 5 seconds {body}` at the main level is the handler function
//! `on·every·0() := {global …; body}` and the call `signal_listen(0, 0, 5000)` (LISTEN_EVERY) where it is written, which starts the timer;
//! the runtime runs the handler at the program's check points (crates/warp-runtime/src/system_signals.rs) and, in
//! `warp run`, after main while the timer lives. The duration is constant (`50 ms`, `5 seconds`, `1 min`).
//! `on file "notes.txt" change {body}` likewise is `on·file·0() := {…}` and `signal_watch(0, "notes.txt")`.
//! Frames (card frame-event): `on frame {body}` is the handler `on·every·N` started by `signal_listen(1, N, 16)` (LISTEN_FRAME), which
//! runs it each frame while the paint window shows and ends with it (once without a window). Natively, `on key {…}`
//! (card native-keys) gets the keys pressed over the paint window: a frame handler reads each press
//! (`system_value("key_pressed")`) and emits `key{key: "a"}` as the playground page does, arrows and Enter by name;
//! `on click {…}` (card native-window) likewise gets the clicks, `click{x: 30, y: 40}` in the painted frame's pixels.
//! `exit` and `exit()` as statements are `exit(0)` (P121: the host word ends the run with that code).
//! Channels (src/host/channels.rs): `on message from "chat" {body}` (`on message {body}` for the channel "warp") is the timer
//! handler `on·every·0() := {global …; while channel_pending(0) > 0 { event = channel_next(0); body }}`, started by
//! `channel_listen(0, "chat")` and `signal_listen(0, 0, 20)`, so a program listening stays like one with a timer;
//! `broadcast value on "chat"` (`broadcast value`) is `channel_send("chat", value)`, and so is `send value to "chat"`.
//! Named events across programs (P129b): `broadcast stop the machine{reason: "heat"} on "chat"` is
//! `channel_send("chat/stop the machine", {reason: "heat"})`, heard by `on stop the machine from "chat" {…}`, a message
//! listener of that sub-channel; `send "file system full"` alone is that named event on "warp" without data, heard by
//! `on "file system full" {…}`. `raise` stays inside the program (event_signals.rs).

use super::listeners;
use super::nodes::call;
use crate::declarations::word;
use crate::errors::diagnostic::Diagnostic;
use crate::event_signals::{event_name, function_with_globals, main_level_variables};
use crate::extensions::numbers::Number;
use crate::node::{parens, symbol, Bracket, Node};
use crate::operators::Op;
use crate::words::{AT_WORD, FROM_WORD, ON_WORD, SEND_WORD};
use warp_runtime::host_words::{CLOCK_WEEKDAYS, LISTEN_CLOCK, LISTEN_EVERY, LISTEN_FRAME};

const EVERY_WORD: &str = "every";
const FRAME_WORD: &str = "frame";
/// A frame's milliseconds: 60 frames a second
const FRAME_MILLISECONDS: i64 = 16;
#[cfg(feature = "native")]
const KEY_EVENT: &str = "key";
/// Each key pressed over the paint window, as the page's KeyboardEvent.key names it: the character, or a key's name
#[cfg(feature = "native")]
const KEYS_TEMPLATE: &str = "on frame {
	CODE = system_value(\"key_pressed\")
	while CODE != 0 {
		emit key{key: NAMED}
		CODE = system_value(\"key_pressed\")
	}
}";
/// Each click over the paint window, where the button went down
#[cfg(feature = "native")]
const CLICK_EVENT: &str = "click";
#[cfg(feature = "native")]
const CLICKS_TEMPLATE: &str = "on frame {
	while system_value(\"click_pressed\") != 0 {
		emit click{x: system_value(\"click_x\"), y: system_value(\"click_y\")}
	}
}";
/// The keys the page names (src/graphics/paint_window.rs key_code, canvas.js KEY_CODES give these codes); 0 is none
const KEY_NAMES: [(i64, &str); 9] = [
	(0, ""),
	(0xF700, "ArrowUp"),
	(0xF701, "ArrowDown"),
	(0xF702, "ArrowLeft"),
	(0xF703, "ArrowRight"),
	(13, "Enter"),
	(27, "Escape"),
	(8, "Backspace"),
	(9, "Tab"),
];
const DAY_WORD: &str = "day";
const WEEKDAY_WORD: &str = "weekday";
const WEEKEND_WORD: &str = "weekend";
/// `9am`, `9pm`: the half of the day by its index
const HALVES: [&str; 2] = ["am", "pm"];
const FILE_WORD: &str = "file";
pub(crate) const CHANGE_WORDS: [&str; 2] = ["change", "changes"];
const MESSAGE_WORD: &str = "message";
const BROADCAST_WORD: &str = "broadcast";
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
	lower_timers(with_window_events(program))
}

/// Natively, a program with `on key {…}` or `on click {…}` gets the paint window's key presses (KEYS_TEMPLATE) and
/// clicks (CLICKS_TEMPLATE); a page raises them itself
#[cfg(feature = "native")]
fn with_window_events(program: Node) -> Node {
	let (mut statements, bracket, separator) = crate::variable_signals::main_statements(&program);
	if crate::pipeline::compile_mode().for_a_page || crate::event_signals::serves_its_page(&statements) {
		return program;
	}
	let mut listened = false;
	for (event, reader) in [(KEY_EVENT, keys_reader as fn() -> Node), (CLICK_EVENT, clicks_reader)] {
		let listener = statements.iter().position(|statement| event_listener(statement, event).is_some());
		if let Some(listener) = listener {
			// right after the listener: the program's last statement stays its value
			statements.insert(listener + 1, reader().drop_meta().clone());
			listened = true;
		}
	}
	if !listened {
		return program;
	}
	Node::List(statements, bracket, separator)
}

#[cfg(feature = "native")]
fn keys_reader() -> Node {
	super::nodes::filled(KEYS_TEMPLATE, [("CODE", symbol("key·code")), ("NAMED", key_named(symbol("key·code")))])
}

/// The name of the key with that code, as the page's KeyboardEvent.key: a key's name (KEY_NAMES), else its character
pub fn key_named(code: Node) -> Node {
	let choices: String =
		KEY_NAMES.iter().map(|(number, name)| format!("if CODE == {number} then \"{name}\" else ")).collect();
	super::nodes::filled(&format!("{choices}\"\" + chr(CODE)"), [("CODE", code)])
}

/// The code of the key of that name, as key_named names it, -1 (never held) for a name of none: a given name at once
pub fn key_code(name: Node) -> Node {
	if let Node::Text(text) = name.drop_meta() {
		return Node::int(named_code(text));
	}
	let choices: String =
		KEY_NAMES.iter().map(|(number, name)| format!("if NAME == \"{name}\" then {number} else ")).collect();
	super::nodes::filled(&format!("{choices}if NAME.length == 1 then ord(NAME) else -1"), [("NAME", name)])
}

fn named_code(name: &str) -> i64 {
	let mut letters = name.chars();
	match (KEY_NAMES.iter().find(|(_, key)| *key == name), letters.next(), letters.next()) {
		(Some((code, _)), _, _) => *code,
		(None, Some(letter), None) => i64::from(u32::from(letter)),
		_ => -1,
	}
}

#[cfg(feature = "native")]
fn clicks_reader() -> Node {
	crate::warp_parser::parse(CLICKS_TEMPLATE)
}

/// The browser compiler's programs are for a page, which raises key and click itself
#[cfg(not(feature = "native"))]
fn with_window_events(program: Node) -> Node {
	program
}

fn lower_timers(program: Node) -> Node {
	let (statements, bracket, separator) = crate::variable_signals::main_statements(&program);
	if !statements.iter().any(|statement| {
		one_shot(statement).is_some()
			|| timer(statement).is_some()
			|| file_watch(statement).is_some()
			|| message_listener(statement).is_some()
			|| event_listener(statement, FRAME_WORD).is_some()
	}) {
		return program;
	}
	let main_variables = main_level_variables(&statements);
	let mut lowered = vec![];
	let (mut count, mut watches) = (0, 0);
	for statement in statements {
		if let Some((path, body)) = file_watch(&statement) {
			let handler = format!("{}{watches}", crate::host::FILE_HANDLER_PREFIX);
			lowered.push(function_with_globals(&handler, false, &[body], &main_variables));
			let start = [symbol(crate::host::SIGNAL_WATCH), Node::int(watches as i64), path];
			lowered.push(parens(start.to_vec()));
			watches += 1;
			continue;
		}
		let id = Node::int(count as i64);
		let mut channel_listen = None;
		let (body, kind, argument) = if let Some((channel, body)) = message_listener(&statement) {
			channel_listen = Some(call(crate::host::CHANNEL_LISTEN, vec![id.clone(), channel]));
			let messages = super::nodes::filled(MESSAGES_TEMPLATE, [("ID", id.clone()), ("BODY", body)]);
			(messages.drop_meta().clone(), LISTEN_EVERY, CHANNEL_CHECK_MILLISECONDS)
		} else if let Some(body) = event_listener(&statement, FRAME_WORD) {
			(body, LISTEN_FRAME, FRAME_MILLISECONDS)
		} else if let Some((time, body)) = one_shot(&statement) {
			let Some(minute_of_day) = time else {
				return no_time_of_day(&statement, AT_WORD);
			};
			(body, LISTEN_CLOCK, minute_of_day)
		} else if let Some((duration, body)) = timer(&statement) {
			match timer_start(&statement, &duration) {
				Ok((kind, argument)) => (body, kind, argument),
				Err(error) => return error,
			}
		} else {
			lowered.push(statement);
			continue;
		};
		let handler = format!("{}{count}", crate::host::TIMER_HANDLER_PREFIX);
		lowered.push(function_with_globals(&handler, false, &[body], &main_variables));
		lowered.extend(channel_listen);
		lowered.push(call(crate::host::SIGNAL_LISTEN, vec![Node::int(kind), id, Node::int(argument)]));
		count += 1;
	}
	Node::List(lowered, bracket, separator)
}

/// The kind and argument of signal_listen that start `on every <duration> {…}`: a time of day on some days, or a
/// constant duration
fn timer_start(statement: &Node, duration: &Node) -> Result<(i64, i64), Node> {
	match clock_schedule(duration) {
		Some(Ok((minute_of_day, weekdays))) => Ok((LISTEN_CLOCK, weekdays * CLOCK_WEEKDAYS + minute_of_day)),
		Some(Err(ClockError::Days(days))) => Err(Diagnostic::at(
			statement,
			format!("on every … at needs day, a weekday (monday … sunday), weekday or weekend, got {days}"),
		)
		.into_error()),
		Some(Err(ClockError::Time)) => Err(no_time_of_day(statement, "on every day at")),
		None => crate::numbers::units::milliseconds(duration)
			.map(|milliseconds| (LISTEN_EVERY, milliseconds))
			.ok_or_else(|| {
				Diagnostic::at(
					statement,
					format!(
						"on every needs a constant duration like `on every 5 seconds {{…}}`, got {}",
						duration.serialize().trim()
					),
				)
				.into_error()
			}),
	}
}

/// `on message {body}`, `on message from "chat" {body}`, and named events from other programs:
/// `on stop the machine from "chat" {body}`, `on "file system full" {body}`: the channel (a text) and the body
fn message_listener(statement: &Node) -> Option<(Node, Node)> {
	let listener = listeners::read(statement, &[ON_WORD])?;
	let words = listener.words();
	let (name, channel) = match words.as_slice() {
		[name @ .., from, channel] if word(from) == FROM_WORD && !name.is_empty() => (name, text(channel)?),
		name => (name, DEFAULT_CHANNEL.to_string()),
	};
	let from_written = name.len() < words.len();
	let channel = match name {
		[message] if word(message) == MESSAGE_WORD => channel,
		[event] if text(event).is_some() => event_channel(&channel, &text(event)?),
		words if from_written => event_channel(&channel, &event_name(words.iter())?),
		_ => return None,
	};
	Some((Node::Text(channel), listener.body))
}

fn text(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Text(text) => Some(text.clone()),
		_ => None,
	}
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
		Node::Key(name, Op::Colon, data) if matches!(data.drop_meta(), Node::List(_, Bracket::Curly, _)) => {
			(name.as_ref().clone(), Some(data.as_ref().clone()))
		}
		_ => (last.clone(), None),
	};
	if words.is_empty() && data.is_none() {
		return None;
	}
	let name = event_name(words.iter().chain([&last_word]))?;
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
			(BROADCAST_WORD, [value @ .., on, channel]) if word(on) == ON_WORD && !value.is_empty() => {
				(value.to_vec(), Some(text(channel)?))
			}
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
			None => (channel, listeners::row(value)),
		};
		Some(call(crate::host::CHANNEL_SEND, vec![Node::Text(channel), broadcasts(value, verbs)]))
	};
	match node {
		Node::List(items, _, _) if sent(&items).is_some() => sent(&items).expect("checked"),
		other => other.map_children(|child| broadcasts(child, verbs)),
	}
}

/// `on frame {body}`, `on frame: body` (the event `name`): the body
fn event_listener(statement: &Node, name: &str) -> Option<Node> {
	let listener = listeners::read(statement, &[ON_WORD])?;
	listener.trigger.is_symbol(name).then_some(listener.body)
}

/// `on file "notes.txt" change {body}`: the path (a text) and the body
fn file_watch(statement: &Node) -> Option<(Node, Node)> {
	let listener = listeners::read(statement, &[ON_WORD])?;
	match listener.words().as_slice() {
		[file, path, change]
			if word(file) == FILE_WORD && text(path).is_some() && CHANGE_WORDS.contains(&word(change).as_str()) =>
		{
			Some((path.clone(), listener.body))
		}
		_ => None,
	}
}

/// `on every 50 ms {body}`, `on every 5 seconds: body`: the duration and the body
fn timer(statement: &Node) -> Option<(Node, Node)> {
	let listener = listeners::read(statement, &[ON_WORD])?;
	match listener.words().as_slice() {
		[every, duration @ ..] if word(every) == EVERY_WORD && !duration.is_empty() => {
			Some((listeners::row(duration.to_vec()), listener.body))
		}
		_ => None,
	}
}

/// `at 9:00 {body}`, `at 9pm: body`: the minute of the day (None for a time that is no time of day) and the body;
/// None for any other use of `at`
fn one_shot(statement: &Node) -> Option<(Option<i64>, Node)> {
	let listener = listeners::read(statement, &[AT_WORD])?;
	Some((minute_of_day(&listener.words())?, listener.body))
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
	let Node::List(words, _, _) = duration.drop_meta() else {
		return None;
	};
	let [days, at, time @ ..] = words.as_slice() else {
		return None;
	};
	if word(at) != AT_WORD {
		return None;
	}
	let Some(weekdays) = weekdays(&word(days)) else {
		return Some(Err(ClockError::Days(days.serialize().trim().to_string())));
	};
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
			int(minute)
				.map(|minute| (hour, minute, None))
				.or_else(|| with_half(minute).map(|(minute, half)| (hour, minute, Some(half))))
		}
		_ => None,
	};
	let (hour, minute, half) = match words {
		[time] => hour_minute(time).or_else(|| with_half(time).map(|(hour, half)| (hour, 0, Some(half))))?,
		[time, spoken_half] => {
			let half = Some(half(spoken_half)?);
			hour_minute(time)
				.filter(|(_, _, written)| written.is_none())
				.map(|(hour, minute, _)| (hour, minute, half))
				.or_else(|| Some((int(time)?, 0, half)))?
		}
		_ => return None,
	};
	let hour = match half {
		None => Some(hour).filter(|hour| (0..24).contains(hour)),
		Some(half) => Some(hour).filter(|hour| (1..=12).contains(hour)).map(|hour| hour % 12 + 12 * half as i64),
	};
	Some(hour.filter(|_| (0..60).contains(&minute)).map(|hour| hour * 60 + minute))
}

/// `exit` or `exit()` as a statement: `exit(0)`; braces holding `exit 3` are one call, not two statements
fn bare_exits(node: Node) -> Node {
	let is_exit = |item: &Node| match item.drop_meta() {
		Node::Symbol(name) => name == crate::host::EXIT,
		Node::List(items, Bracket::Round, _) => items.len() == 1 && word(&items[0]) == crate::host::EXIT,
		_ => false,
	};
	let exit_zero = || call(crate::host::EXIT, vec![Node::int(0)]);
	match node {
		Node::List(items, bracket, separator)
			if separator.separates_statements() || (bracket == Bracket::Curly && items.len() == 1) =>
		{
			Node::List(
				items.into_iter().map(|item| if is_exit(&item) { exit_zero() } else { bare_exits(item) }).collect(),
				bracket,
				separator,
			)
		}
		other => other.map_children(bare_exits),
	}
}

/// Does the program define or assign the name itself (`exit := …`, `def exit(…)`, `exit = …`)
fn defines(program: &Node, name: &str) -> bool {
	let mut defined = false;
	program.visit(&mut |part| {
		if let Node::Key(target, op, _) = part {
			let head = match target.drop_meta() {
				Node::List(items, _, _) => items.first().map(word).unwrap_or_default(),
				other => word(other),
			};
			defined |= head == name && matches!(op, crate::operators::Op::Assign | crate::operators::Op::Define);
		}
	});
	defined
}
