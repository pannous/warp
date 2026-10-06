//! Timers (notes/system_signals.md): `on every 5 seconds {body}` at the main level is the handler function
//! `on·every·0() := {global …; body}` and the call `signal_every(0, 5000)` where it is written, which starts the timer;
//! the runtime runs the handler at the program's check points (crates/warp-runtime/src/system_signals.rs) and, in
//! `warp run`, after main while the timer lives. The duration is constant (`50 ms`, `5 seconds`, `1 min`).
//! `on file "notes.txt" change {body}` likewise is `on·file·0() := {…}` and `signal_watch(0, "notes.txt")`.
//! `exit` and `exit()` as statements are `exit(0)` (P121: the host word ends the run with that code).
//! Channels (src/channels.rs): `on message from "chat" {body}` (`on message {body}` for the channel "warp") is the timer
//! handler `on·every·0() := {global …; while channel_pending(0) > 0 { event = channel_next(0); body }}`, started by
//! `channel_listen(0, "chat")` and `signal_every(0, 20)`, so a program listening stays like one with a timer;
//! `broadcast value on "chat"` (`broadcast value`) is `channel_send("chat", value)`.

use crate::declarations::word;
use crate::diagnostic::Diagnostic;
use crate::event_signals::{function_with_globals, main_level_variables};
use crate::node::{Bracket, Node, Separator};

const ON_WORD: &str = "on";
const EVERY_WORD: &str = "every";
const FILE_WORD: &str = "file";
const CHANGE_WORDS: [&str; 2] = ["change", "changes"];
const MESSAGE_WORD: &str = "message";
const FROM_WORD: &str = "from";
const BROADCAST_WORD: &str = "broadcast";
/// The channel of `on message {…}` and `broadcast value` without a channel name
const DEFAULT_CHANNEL: &str = "warp";
/// How often a listening program looks for messages
const CHANNEL_CHECK_MILLISECONDS: i64 = 20;
/// The template of a message handler's body
const MESSAGES_TEMPLATE: &str = "while channel_pending(ID) > 0 { event = channel_next(ID); BODY }";

pub fn lower(program: Node) -> Node {
	let program = if defines(&program, crate::host::EXIT) { program } else { bare_exits(program) };
	let program = if defines(&program, BROADCAST_WORD) { program } else { broadcasts(program) };
	lower_timers(program)
}

fn lower_timers(program: Node) -> Node {
	let Node::List(statements, bracket, separator) = program.drop_meta().clone() else { return program };
	if !statements.iter().any(|statement| timer(statement).is_some() || file_watch(statement).is_some() || message_listener(statement).is_some()) {
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
		let Some((duration, body)) = timer(&statement) else {
			lowered.push(statement);
			continue;
		};
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

/// `on message {body}`, `on message from "chat" {body}`: the channel (a text) and the body
fn message_listener(statement: &Node) -> Option<(Node, Node)> {
	let Node::List(items, _, _) = statement.drop_meta() else { return None };
	let (channel, body) = match items.as_slice() {
		[on, message, body] if word(on) == ON_WORD && word(message) == MESSAGE_WORD => (Node::Text(DEFAULT_CHANNEL.to_string()), body),
		[on, message, from, channel, body] if word(on) == ON_WORD && word(message) == MESSAGE_WORD && word(from) == FROM_WORD => (channel.drop_meta().clone(), body),
		_ => return None,
	};
	(matches!(channel, Node::Text(_)) && matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _))).then(|| (channel, body.clone()))
}

/// `broadcast value on "chat"`, `broadcast value`: `channel_send("chat", value)`
fn broadcasts(node: Node) -> Node {
	let sent = |items: &[Node]| -> Option<Node> {
		let (first, rest) = items.split_first()?;
		if word(first) != BROADCAST_WORD || rest.is_empty() {
			return None;
		}
		let (value, channel) = match rest {
			[value @ .., on, channel] if word(on) == ON_WORD && matches!(channel.drop_meta(), Node::Text(_)) && !value.is_empty() => (value, channel.drop_meta().clone()),
			value => (value, Node::Text(DEFAULT_CHANNEL.to_string())),
		};
		let value = match value {
			[single] => single.clone(),
			several => Node::List(several.to_vec(), Bracket::None, Separator::Space),
		};
		Some(call(crate::host::CHANNEL_SEND, vec![channel, broadcasts(value)]))
	};
	match node {
		Node::List(items, _, _) if sent(&items).is_some() => sent(&items).expect("checked"),
		other => other.map_children(broadcasts),
	}
}

fn call(function: &str, arguments: Vec<Node>) -> Node {
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
	let (last, duration_words) = rest.split_last()?;
	let (duration_end, body) = match last.drop_meta() {
		Node::List(_, Bracket::Curly, _) => (None, last.clone()),
		_ => {
			let (end, body) = crate::declarations::handler_parts(last)?;
			(Some(end), body)
		}
	};
	let words: Vec<Node> = duration_words.iter().cloned().chain(duration_end).collect();
	let duration = match words.as_slice() {
		[] => return None,
		[single] => single.clone(),
		several => Node::List(several.to_vec(), Bracket::None, Separator::Space),
	};
	Some((duration, body))
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
