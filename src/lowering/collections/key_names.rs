//! Key names in a key handler (card key-if): inside `on key {…}` a bare `key` is the pressed key, `event.key`, and the
//! key words name keys as the page's KeyboardEvent.key does (natively lowering/signals/system_signals.rs KEY_NAMES), so
//! `on key { if key == ESC { exit } }` reads as written. A program's own variable (`key`, `UP`) stays its own. A
//! comparison with a word that names nothing (`key == ESCAPEE`, `key == q`) never holds, which a warning says.
//! The key words compare with `held_key` anywhere too (card held-key): `held_key == LEFT` is `held_key == "ArrowLeft"`.
//! Compared with `key` or `held_key` itself, a key word may be lowercase (card if-key): `if key == space`,
//! `held_key == left`; never a name the program binds anywhere (a variable, loop variable, parameter, field). So does a
//! compared text in any case: `key == "escape"` is `key == "Escape"`; one character keeps its case ("a" is not "A").
//!
//! A comparison with `held_key` asks whether that key is held, among others (card allow-diagonals): with LEFT and UP
//! held, `held_key == LEFT` and `held_key == UP` both hold, so a program steers diagonally; `held_key` alone names the
//! last to go down. `held_key == ""`: none is held.

use super::nodes::{call, key};
use crate::errors::diagnostic::Diagnostic;
use crate::event_signals::{main_level_variables, EVENT_WORD};
use crate::node::{symbol, Node};
use crate::operators::Op;
use std::cell::Cell;
use std::collections::HashSet;
use warp_runtime::host_words::{HELD_KEY, KEY_HELD};

const HANDLER_WORDS: [&str; 2] = ["on", "once"];
const KEY_EVENT: &str = "key";
/// Each key word and the KeyboardEvent.key it names; F1 … F12 name themselves
const KEY_WORDS: [(&str, &str); 34] = [
	("Escape", "Escape"),
	("ESC", "Escape"),
	("ESCAPE", "Escape"),
	("Enter", "Enter"),
	("ENTER", "Enter"),
	("RETURN", "Enter"),
	("Tab", "Tab"),
	("TAB", "Tab"),
	("Backspace", "Backspace"),
	("BACKSPACE", "Backspace"),
	("Space", " "),
	("SPACE", " "),
	("ArrowUp", "ArrowUp"),
	("UP", "ArrowUp"),
	("ArrowDown", "ArrowDown"),
	("DOWN", "ArrowDown"),
	("ArrowLeft", "ArrowLeft"),
	("LEFT", "ArrowLeft"),
	("ArrowRight", "ArrowRight"),
	("RIGHT", "ArrowRight"),
	("Delete", "Delete"),
	("DELETE", "Delete"),
	("Shift", "Shift"),
	("SHIFT", "Shift"),
	("Control", "Control"),
	("CONTROL", "Control"),
	("CTRL", "Control"),
	("Alt", "Alt"),
	("ALT", "Alt"),
	("OPTION", "Alt"),
	("Meta", "Meta"),
	("META", "Meta"),
	("COMMAND", "Meta"),
	("CMD", "Meta"),
];
/// The key a program keeps once it compares with it (card escape-window)
const ESCAPE_NAME: &str = "Escape";
const FUNCTION_KEYS: std::ops::RangeInclusive<u8> = 1..=12;

pub fn lower(program: Node) -> Node {
	let (statements, _, _) = crate::variable_signals::main_statements(&program);
	let mut warnings = vec![];
	let keys = KeyScope::of(&program, &statements);
	let named = with_key_names(program, &keys, &mut warnings);
	keys.reported(named, &warnings)
}

/// `held_key == LEFT`, `ESC != held_key`: before system_values reads `held_key`
pub fn lower_held_comparisons(program: Node) -> Node {
	let (statements, _, _) = crate::variable_signals::main_statements(&program);
	let mut warnings = vec![];
	let keys = KeyScope::of(&program, &statements);
	let named = held_comparisons(program, &keys, &mut warnings);
	keys.reported(named, &warnings)
}

/// The program's main-level variables, which stay its own, and every name it binds, never a lowercase key word
struct KeyScope {
	own: HashSet<String>,
	bound: HashSet<String>,
	/// A comparison named Escape: the program keeps that key (card escape-window)
	escape: Cell<bool>,
}

impl KeyScope {
	fn of(program: &Node, statements: &[Node]) -> Self {
		KeyScope {
			own: main_level_variables(statements),
			bound: crate::program_names::Bindings::of(program).bound(),
			escape: Cell::new(false),
		}
	}

	/// The key a comparison names, Escape noted
	fn noted(&self, name: Option<String>) -> Option<String> {
		self.escape.set(self.escape.get() || name.as_deref() == Some(ESCAPE_NAME));
		name
	}

	/// The program with its key names, or the warnings' error; one comparing a key with Escape keeps that key (else
	/// Escape over the paint window ends the run)
	fn reported(&self, named: Node, warnings: &[Diagnostic]) -> Node {
		match crate::errors::diagnostic::report(warnings) {
			Ok(()) if self.escape.get() => crate::lowering::system_signals::keeping_escape(named),
			Ok(()) => named,
			Err(error) => error,
		}
	}
}

fn held_comparisons(node: Node, keys: &KeyScope, warnings: &mut Vec<Diagnostic>) -> Node {
	match node.drop_meta() {
		Node::Key(left, op @ (Op::Eq | Op::Ne), right) if left.is_symbol(HELD_KEY) || right.is_symbol(HELD_KEY) => {
			let [left, right] = [left, right].map(|side| held_compared(side, keys, warnings));
			let compared = if left.is_symbol(HELD_KEY) { &right } else { &left };
			if compared.is_symbol(HELD_KEY) {
				return key(left, *op, right);
			}
			let held = call(KEY_HELD, vec![crate::system_signals::key_code(compared.clone())]);
			key(held, if *op == Op::Eq { Op::Ne } else { Op::Eq }, Node::int(0))
		}
		_ => node.map_children(|child| held_comparisons(child, keys, warnings)),
	}
}

/// A side of a comparison with `held_key`: a key word becomes its name; a capitalized word naming nothing (`LEFTT`) warns,
/// a lowercase one stays for the undefined-variable error
fn held_compared(side: &Node, keys: &KeyScope, warnings: &mut Vec<Diagnostic>) -> Node {
	match side.symbol_name() {
		Some(word) if word == HELD_KEY || keys.own.contains(word) => side.clone(),
		Some(word) => match keys.noted(key_name(word).or_else(|| lowercase_key_name(word, &keys.bound))) {
			Some(name) => Node::Text(name),
			None => {
				if word.chars().all(|letter| !letter.is_lowercase()) {
					warnings.push(names_nothing_warning(side, HELD_KEY, word));
				}
				side.clone()
			}
		},
		None => keys.noted(text_key_name(side)).map(Node::Text).unwrap_or_else(|| side.clone()),
	}
}

fn names_nothing_warning(side: &Node, compared: &str, word: &str) -> Diagnostic {
	Diagnostic::at(side, format!("{compared}: {word} names nothing, so the comparison never holds; write a key name (ESC, ENTER, UP, F1 …) or a text such as \"{word}\"")).severe()
}

fn with_key_names(node: Node, keys: &KeyScope, warnings: &mut Vec<Diagnostic>) -> Node {
	match key_handler_body(&node) {
		Some(index) => {
			let Node::List(mut items, bracket, separator) = node.drop_meta().clone() else {
				unreachable!("a handler is a list")
			};
			let body = &items[index];
			let mut known = keys.own.clone();
			known.extend(body.written_names::<Vec<String>>());
			unbound_comparisons(body, &known, &keys.bound, warnings);
			items[index] = named(body.clone(), keys);
			Node::List(items, bracket, separator)
		}
		None => node.map_children(|child| with_key_names(child, keys, warnings)),
	}
}

/// `key == ESCAPEE`: a side of a comparison that is a word naming nothing, neither a key nor a variable
fn unbound_comparisons(body: &Node, known: &HashSet<String>, bound: &HashSet<String>, warnings: &mut Vec<Diagnostic>) {
	let body = body.drop_meta();
	if let Node::Key(left, Op::Eq | Op::Ne, right) = body {
		for (side, other) in [(left, right), (right, left)] {
			let names_key = |word: &str| other.is_symbol(KEY_EVENT) && lowercase_key_name(word, bound).is_some();
			let Some(word) = side.symbol_name().filter(|word| names_nothing(word, known) && !names_key(word)) else {
				continue;
			};
			let shadowed_key = other.is_symbol(KEY_EVENT).then(|| key_name(&word.to_uppercase())).flatten();
			warnings.push(match shadowed_key {
				// `f(left) := left` elsewhere: until scopes are tracked (card path-parameter-scope), the bound name wins
				Some(_) => Diagnostic::at(
					side,
					format!(
						"on key: {word} is also a name the program binds, so it names no key here; write {}",
						word.to_uppercase()
					),
				)
				.severe(),
				None => names_nothing_warning(side, "on key", word),
			});
		}
	}
	for part in body.parts() {
		unbound_comparisons(part, known, bound, warnings);
	}
}

fn names_nothing(word: &str, known: &HashSet<String>) -> bool {
	word != KEY_EVENT && word != EVENT_WORD && !known.contains(word) && key_name(word).is_none()
}

/// `on key {body}`, `once key {body}`, `on key: body`: the index of the item holding the body
fn key_handler_body(node: &Node) -> Option<usize> {
	let Node::List(items, _, _) = node.drop_meta() else {
		return None;
	};
	let is_handler_word = HANDLER_WORDS.contains(&items.first()?.symbol_name()?);
	match items.get(1)?.drop_meta() {
		Node::Symbol(event) if is_handler_word && event == KEY_EVENT && items.len() == 3 => Some(2),
		Node::Key(event, Op::Colon, _) if is_handler_word && event.is_symbol(KEY_EVENT) && items.len() == 2 => Some(1),
		_ => None,
	}
}

/// The body with `key` as event.key and the key words as the keys' names, but for the program's own variables
fn named(body: Node, keys: &KeyScope) -> Node {
	let own = &keys.own;
	match body.drop_meta() {
		Node::Symbol(word) if own.contains(word) => body,
		Node::Symbol(word) if word == KEY_EVENT => key(symbol(EVENT_WORD), Op::Dot, symbol(KEY_EVENT)),
		Node::Symbol(word) => keys.noted(key_name(word)).map(Node::Text).unwrap_or(body),
		// `key == space`, `up != key`, `key == "escape"`: the word or text compared with the pressed key itself names a
		// key in lowercase too
		Node::Key(left, op @ (Op::Eq | Op::Ne), right) if is_pressed_key(left, own) || is_pressed_key(right, own) => {
			let [left, right] = [left, right].map(|side| {
				let lowercase = side.symbol_name().and_then(|word| lowercase_key_name(word, &keys.bound));
				let compared = keys.noted(lowercase.or_else(|| text_key_name(side)));
				compared.map(Node::Text).unwrap_or_else(|| named(side.as_ref().clone(), keys))
			});
			key(left, *op, right)
		}
		// `event.key`: the field stays a field
		Node::Key(target, Op::Dot, field) => key(named(target.as_ref().clone(), keys), Op::Dot, field.as_ref().clone()),
		_ => body.map_children(|child| named(child, keys)),
	}
}

/// The bare `key` of a key handler, unless the program has a variable of that name
fn is_pressed_key(side: &Node, own: &HashSet<String>) -> bool {
	side.is_symbol(KEY_EVENT) && !own.contains(KEY_EVENT)
}

/// `space`, `esc`, `f1`: a key word in lowercase, unless the program binds that name anywhere
fn lowercase_key_name(word: &str, bound: &HashSet<String>) -> Option<String> {
	let lowercase = word.chars().all(|letter| !letter.is_uppercase());
	(lowercase && !bound.contains(word)).then(|| key_name(&word.to_uppercase())).flatten()
}

/// `"escape"`, `"ESCAPE"`, `"arrowup"`: a text compared with the pressed key names that key in any case; one character
/// stays itself, "a" and "A" being different presses
fn text_key_name(side: &Node) -> Option<String> {
	let Node::Text(text) = side.drop_meta() else { return None };
	let named = KEY_WORDS.iter().find(|(word, _)| word.eq_ignore_ascii_case(text)).map(|(_, key)| key.to_string());
	named.or_else(|| key_name(&text.to_uppercase())).filter(|_| text.chars().count() > 1)
}

fn key_name(word: &str) -> Option<String> {
	let function_key = word
		.strip_prefix('F')
		.and_then(|number| number.parse::<u8>().ok())
		.is_some_and(|number| FUNCTION_KEYS.contains(&number));
	KEY_WORDS
		.iter()
		.find(|(name, _)| *name == word)
		.map(|(_, key)| key.to_string())
		.or_else(|| function_key.then(|| word.to_string()))
}
