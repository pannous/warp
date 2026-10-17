//! System values (P136, notes/system_signals.md): `battery` (percent), `charging`, `online` and `dark mode` are the
//! machine's state, read by the host word `system_value("battery")` (a yes/no value is `system_value("online") != 0`).
//! A program that binds the name itself (`online = 3`, `f(online) := …`, a field `{battery: 5}`) keeps its own.
//! `name` (before signal_values::poll_shared) marks each read as the symbol `system·battery`, so poll_shared turns a
//! main-level listener on one (`whenever battery < 20% {…}`, `on change online {…}`) into a check of `on·shared`, like
//! a listener on a shared value, and adds a timer `on every 1000ms {}` so a `warp run` stays and looks once a second.
//! `read` (after poll_shared) makes each marked symbol the host call.
//! `whenever battery < 20% {…}` arrives as `battery < (20 % {…})`: a percent compared with the battery is that number.
//! The clipboard: `on clipboard change {…}` is `on change` of its change count (`clipboard count`), which reads no
//! content; `clipboard` is its text, the host call `clipboard_text()` made where the program reads it, never polled.
//! `now` is the clock's instant where the program reads it, `instant_at(clock())` (time/mod.rs), never polled.
//! Browser names (P188): `clipboard.read()` is `clipboard`, `clipboard.write(text)` the std word
//! `std_io("clipboard", "write", [text])` (std_adapters.rs; host-files.js in a page).

use super::nodes::{arguments_rewritten, call, key};
use crate::declarations::word;
use crate::lowering::signals::system_signals::CHANGE_WORDS;
use crate::lowering::web::listeners;
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::time::NOW_WORD;
use crate::words::{FLOAT_WORD, ON_WORD};
use std::collections::HashSet;
use warp_runtime::host_words::{
	BATTERY, CLIPBOARD, CLIPBOARD_COUNT, CLIPBOARD_TEXT, HELD_KEY, KEY_CODE_HELD, MOUSE_X, MOUSE_Y, SYSTEM_VALUE,
	SYSTEM_VALUES,
};

/// `held_key` reads the held key's code once and names it (card held-key)
const HELD_CODE: &str = "held·code";
/// The mark of a system value read between the two passes
pub const SYSTEM_PREFIX: &str = "system·";
/// Host readings that are measurements, not counts: float sources (decision exact-decimals), so the exact decimals
/// they meet (`col + 0.5 - mouse_x`) convert once at that use instead of making every later step exact
const FLOAT_VALUES: [&str; 2] = [MOUSE_X, MOUSE_Y];
const LISTENER_WORDS: [&str; 3] = ["whenever", "once", "on"];
/// The timer that keeps a listening program and wakes it for the checks
const KEEP_LISTENING: &str = "on every 1000ms {}";
const CLIPBOARD_READ: &str = "read";
const CLIPBOARD_WRITE: &str = "write";
/// stands for the text in the template of a clipboard write
const TEXT_PLACEHOLDER: &str = "clipboard_text_written";

pub fn name(program: Node) -> Node {
	let bound = assigned_symbols(&program);
	let names: Vec<(&str, bool)> = SYSTEM_VALUES
		.into_iter()
		.chain([(CLIPBOARD, false), (NOW_WORD, false), (HELD_KEY, false)])
		.filter(|(name, _)| !bound.contains(*name))
		.collect();
	if names.is_empty() {
		return program;
	}
	let program = if bound.contains(CLIPBOARD) { program } else { clipboard_listeners(program) };
	let marked = mark(program, &names);
	if marked_names(&marked).is_empty() {
		return marked;
	}
	let (statements, bracket, separator) = crate::variable_signals::main_statements(&marked);
	// a listener on a system value: one in its trigger (`whenever battery < 20%`); a read in the body of `on frame`
	// listens to nothing, so a headless run still ends after its frame (card full-screen)
	let listens = |statement: &Node| {
		let trigger = listeners::read(statement, &LISTENER_WORDS).map(|listener| listener.trigger);
		LISTENER_WORDS.contains(&first_word(statement).as_str())
			&& !marked_names(trigger.as_ref().unwrap_or(statement)).is_empty()
	};
	let Some(first_listener) = statements.iter().position(listens) else {
		return marked;
	};
	// right after the first listener: the program's last statement stays its value
	let mut statements: Vec<Node> = statements.into_iter().map(with_percent_body).collect();
	statements.insert(first_listener + 1, crate::warp_parser::parse_generated(KEEP_LISTENING));
	Node::List(statements, bracket, separator)
}

pub fn read(program: Node) -> Node {
	if marked_names(&program).is_empty() {
		return program;
	}
	reading(program)
}

/// The system values the node reads, marked
pub fn marked_names(node: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	node.visit(&mut |part| {
		if let Node::Symbol(name) = part {
			if name.starts_with(SYSTEM_PREFIX) {
				names.insert(name.clone());
			}
		}
	});
	names
}

fn first_word(statement: &Node) -> String {
	match statement.drop_meta() {
		Node::List(items, _, _) => items.first().map(word).unwrap_or_default(),
		_ => String::new(),
	}
}

/// The names the program binds: assigned, defined, parameters of functions and lambdas
pub(crate) fn assigned_symbols(program: &Node) -> HashSet<String> {
	let mut bound = HashSet::new();
	program.visit(&mut |part| {
		if let Node::Key(target, Op::Assign | Op::Define | Op::Arrow, _) = part {
			bound.extend(crate::variable_signals::symbols(target));
		}
	});
	bound
}

/// `held_key`: the name of the key held now, as `on key` names a press (system_signals.rs key_named)
fn held_key() -> Node {
	let read = key(symbol(HELD_CODE), Op::Assign, call(SYSTEM_VALUE, vec![Node::Text(KEY_CODE_HELD.to_string())]));
	let named = crate::system_signals::key_named(symbol(HELD_CODE));
	Node::List(vec![read, named], Bracket::Curly, Separator::Semicolon)
}

/// Each read of a system value as its mark; a field name (`{battery: 5}`, `p.battery`) is no read
fn mark(node: Node, names: &[(&str, bool)]) -> Node {
	let marked = |name: &str| match names.iter().any(|(system, _)| *system == name) {
		false => None,
		true if name == CLIPBOARD => Some(call(CLIPBOARD_TEXT, vec![])),
		true if name == NOW_WORD => Some(crate::time::now_call()),
		true if name == HELD_KEY => Some(held_key()),
		true => Some(Node::Symbol(format!("{SYSTEM_PREFIX}{name}"))),
	};
	match node {
		Node::Symbol(name) => marked(&name).unwrap_or(Node::Symbol(name)),
		Node::Key(left, Op::Dot, right)
			if word(&left) == CLIPBOARD
				&& names.iter().any(|(name, _)| *name == CLIPBOARD)
				&& clipboard_method(&right).is_some() =>
		{
			let (method, arguments) = clipboard_method(&right).expect("a clipboard method");
			match (method.as_str(), arguments.as_slice()) {
				(CLIPBOARD_WRITE, [text]) => clipboard_write(mark(text.clone(), names)),
				_ => marked(CLIPBOARD).expect("the clipboard's read"),
			}
		}
		Node::Key(left, op @ (Op::Colon | Op::Dot), right) => {
			let right = if op == Op::Dot {
				method_arguments_marked(*right, names)
			} else if is_bare_word(&right) {
				// data stays data: `{created: now}` keeps the word, `now!` reads the clock (decision data-stays-data)
				*right
			} else {
				mark(*right, names)
			};
			// the target of a dot is read (`now.hour`), a key's name is not
			let left = if op == Op::Dot { Box::new(mark(*left, names)) } else { left };
			Node::Key(left, op, Box::new(right))
		}
		Node::List(items, bracket, separator) => {
			// `dark mode`: a phrase of two words
			let (mut joined, mut phrases) = (vec![], 0);
			let mut items = items.into_iter().peekable();
			while let Some(item) = items.next() {
				// `not dark mode` and `x = not dark mode` arrive as `(not dark) mode`, `(x = not dark) mode`: the phrase's
				// first word is the rightmost of the item before
				let phrase = items
					.peek()
					.and_then(|next| with_phrase(&item, &|first| marked(&format!("{} {}", word(first), word(next)))));
				match phrase {
					Some(phrase) => {
						items.next();
						phrases += 1;
						joined.push(mark(phrase, names));
					}
					None => joined.push(mark(item, names)),
				}
			}
			match (joined.len(), &bracket) {
				(1, Bracket::None) if phrases > 0 => joined.pop().expect("one item"),
				_ => Node::List(joined, bracket, separator),
			}
		}
		other => other.map_children(|child| mark(child, names)),
	}
}

/// After a dot: the method name stays, its arguments are reads (`seen.push(online)` is `(. seen (push online))`)
fn method_arguments_marked(member: Node, names: &[(&str, bool)]) -> Node {
	match member {
		Node::List(items, bracket, separator) if items.len() > 1 => {
			arguments_rewritten(items, bracket, separator, |argument| mark(argument, names))
		}
		meta @ Node::Meta { .. } => meta.under_meta(|node| method_arguments_marked(node, names)),
		field => field,
	}
}

/// The item with its rightmost word (along the right operands of keys) replaced by the phrase it starts, if it starts one
fn with_phrase(item: &Node, phrase: &dyn Fn(&Node) -> Option<Node>) -> Option<Node> {
	match item.drop_meta() {
		Node::Key(left, op, right) if !matches!(op, Op::Colon | Op::Dot) => {
			with_phrase(right, phrase).map(|right| Node::Key(left.clone(), *op, Box::new(right)))
		}
		Node::Symbol(_) => phrase(item),
		_ => None,
	}
}

/// `on clipboard change {…}`: `on change` of the clipboard's change count
fn clipboard_listeners(node: Node) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let listens = matches!(items.as_slice(), [on, clipboard, change, ..] if word(on) == ON_WORD && word(clipboard) == CLIPBOARD && CHANGE_WORDS.contains(&word(change).as_str()));
			let items: Vec<Node> = items.into_iter().map(clipboard_listeners).collect();
			if !listens {
				return Node::List(items, bracket, separator);
			}
			let listener = [symbol(ON_WORD), symbol(CHANGE_WORDS[0]), symbol(CLIPBOARD_COUNT)];
			Node::List(listener.into_iter().chain(items.into_iter().skip(3)).collect(), bracket, separator)
		}
		meta @ Node::Meta { .. } => meta.under_meta(clipboard_listeners),
		other => other,
	}
}

/// `whenever battery < 20% {…}`, which arrives as `whenever battery < (20 % {…})`: `whenever battery < 20 {…}`
fn with_percent_body(statement: Node) -> Node {
	let Node::List(items, bracket, separator) = statement.drop_meta().clone() else {
		return statement;
	};
	let [listener, condition] = items.as_slice() else {
		return statement;
	};
	if !marked_names(condition).contains(&format!("{SYSTEM_PREFIX}{BATTERY}")) {
		return statement;
	}
	match percent_body(condition) {
		Some((condition, body)) => Node::List(vec![listener.clone(), condition, body], bracket, separator),
		None => statement,
	}
}

fn percent_body(condition: &Node) -> Option<(Node, Node)> {
	let Node::Key(left, op, right) = condition.drop_meta() else {
		return None;
	};
	match right.drop_meta() {
		Node::Key(percent, Op::Mod, body) if matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) => {
			Some((Node::Key(left.clone(), *op, percent.clone()), body.as_ref().clone()))
		}
		_ => percent_body(right).map(|(right, body)| (Node::Key(left.clone(), *op, Box::new(right)), body)),
	}
}

/// Each mark as the host call
fn reading(node: Node) -> Node {
	match node {
		Node::Symbol(name) if name.starts_with(SYSTEM_PREFIX) => {
			let name = name[SYSTEM_PREFIX.len()..].to_string();
			let yes_no = SYSTEM_VALUES.iter().any(|(system, yes_no)| *system == name && *yes_no);
			let measured = FLOAT_VALUES.contains(&name.as_str());
			let call = call(SYSTEM_VALUE, vec![Node::Text(name)]);
			if yes_no {
				key(call, Op::Ne, Node::int(0))
			} else if measured {
				key(call, Op::As, symbol(FLOAT_WORD))
			} else {
				call
			}
		}
		other => other.map_children(reading),
	}
}

/// `read()` or `write(text)` after `clipboard.`: the method and its arguments
fn clipboard_method(call: &Node) -> Option<(String, Vec<Node>)> {
	let Node::List(items, _, _) = call.drop_meta() else {
		return None;
	};
	let (method, arguments) = items.split_first()?;
	let method = word(method);
	let arguments: Vec<Node> = match arguments {
		[Node::List(grouped, Bracket::Round, _)] => grouped.clone(),
		other => other.to_vec(),
	};
	match (method.as_str(), arguments.len()) {
		(CLIPBOARD_READ, 0) | (CLIPBOARD_WRITE, 1) => Some((method, arguments)),
		_ => None,
	}
}

/// `std_io("clipboard", "write", [text])`
fn clipboard_write(text: Node) -> Node {
	let call = crate::warp_parser::parse_generated(&format!(
		"std_io(\"{CLIPBOARD}\", \"{CLIPBOARD_WRITE}\", [{TEXT_PLACEHOLDER}])"
	));
	crate::law::substitute(&call, &[(TEXT_PLACEHOLDER.to_string(), text)].into_iter().collect())
}

/// A word alone as a field's value, not forced with `!`
fn is_bare_word(value: &Node) -> bool {
	matches!(value.drop_meta(), Node::Symbol(_)) && crate::mutation::bang_target(value).is_none()
}
