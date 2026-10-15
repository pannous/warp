//! Key names in a key handler (card key-if): inside `on key {…}` a bare `key` is the pressed key, `event.key`, and the
//! key words name keys as the page's KeyboardEvent.key does (natively lowering/signals/system_signals.rs KEY_NAMES), so
//! `on key { if key == ESC { exit } }` reads as written. A program's own variable (`key`, `UP`) stays its own. A
//! comparison with a word that names nothing (`key == ESCAPEE`, `key == q`) never holds, which a warning says.
//! The key words compare with `held_key` anywhere too (card held-key): `held_key == LEFT` is `held_key == "ArrowLeft"`.
//! Beside `key` or `held_key` a key word may be written in lowercase (card if-key): `if key == space`, `held_key == left`;
//! elsewhere a lowercase word keeps its meaning (`return`, a local `up`).

use super::nodes::key;
use crate::errors::diagnostic::Diagnostic;
use crate::event_signals::{main_level_variables, EVENT_WORD};
use crate::node::{symbol, Node};
use crate::operators::Op;
use std::collections::HashSet;
use warp_runtime::host_words::HELD_KEY;

const HANDLER_WORDS: [&str; 2] = ["on", "once"];
const KEY_EVENT: &str = "key";
/// Each key word and the KeyboardEvent.key it names; F1 … F12 name themselves
const KEY_WORDS: [(&str, &str); 22] = [
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
];
const FUNCTION_KEYS: std::ops::RangeInclusive<u8> = 1..=12;

pub fn lower(program: Node) -> Node {
	let (statements, _, _) = crate::variable_signals::main_statements(&program);
	let mut warnings = vec![];
	let named = with_key_names(program, &main_level_variables(&statements), &mut warnings);
	match crate::errors::diagnostic::report(&warnings) {
		Ok(()) => named,
		Err(error) => error,
	}
}

/// `held_key == LEFT`, `ESC != held_key`: before system_values reads `held_key`
pub fn lower_held_comparisons(program: Node) -> Node {
	let (statements, _, _) = crate::variable_signals::main_statements(&program);
	let mut warnings = vec![];
	let named = held_comparisons(program, &main_level_variables(&statements), &mut warnings);
	match crate::errors::diagnostic::report(&warnings) {
		Ok(()) => named,
		Err(error) => error,
	}
}

fn held_comparisons(node: Node, own: &HashSet<String>, warnings: &mut Vec<Diagnostic>) -> Node {
	match node.drop_meta() {
		Node::Key(left, op @ (Op::Eq | Op::Ne), right) if left.is_symbol(HELD_KEY) || right.is_symbol(HELD_KEY) => {
			let [left, right] = [left, right].map(|side| held_compared(side, own, warnings));
			key(left, *op, right)
		}
		_ => node.map_children(|child| held_comparisons(child, own, warnings)),
	}
}

/// A side of a comparison with `held_key`: a key word becomes its name; a capitalized word naming nothing (`LEFTT`) warns,
/// a lowercase one stays for the undefined-variable error
fn held_compared(side: &Node, own: &HashSet<String>, warnings: &mut Vec<Diagnostic>) -> Node {
	match side.symbol_name() {
		Some(word) if word == HELD_KEY || own.contains(word) => side.clone(),
		Some(word) => match compared_key_name(word) {
			Some(name) => Node::Text(name),
			None => {
				if word.chars().all(|letter| !letter.is_lowercase()) {
					warnings.push(names_nothing_warning(side, HELD_KEY, word));
				}
				side.clone()
			}
		},
		None => side.clone(),
	}
}

fn names_nothing_warning(side: &Node, compared: &str, word: &str) -> Diagnostic {
	Diagnostic::at(side, format!("{compared}: {word} names nothing, so the comparison never holds; write a key name (ESC, ENTER, UP, F1 …) or a text such as \"{word}\"")).severe()
}

fn with_key_names(node: Node, own: &HashSet<String>, warnings: &mut Vec<Diagnostic>) -> Node {
	match key_handler_body(&node) {
		Some(index) => {
			let Node::List(mut items, bracket, separator) = node.drop_meta().clone() else {
				unreachable!("a handler is a list")
			};
			let body = &items[index];
			let mut known = own.clone();
			known.extend(body.assigned_names::<Vec<String>>(|op| matches!(op, Op::Assign | Op::Define)));
			unbound_comparisons(body, &known, warnings);
			items[index] = named(body.clone(), own);
			Node::List(items, bracket, separator)
		}
		None => node.map_children(|child| with_key_names(child, own, warnings)),
	}
}

/// `key == ESCAPEE`: a side of a comparison that is a word naming nothing, neither a key nor a variable
fn unbound_comparisons(body: &Node, known: &HashSet<String>, warnings: &mut Vec<Diagnostic>) {
	let body = body.drop_meta();
	if let Node::Key(left, Op::Eq | Op::Ne, right) = body {
		for side in [left, right] {
			if let Some(word) = side.symbol_name().filter(|word| names_nothing(word, known)) {
				warnings.push(names_nothing_warning(side, "on key", word));
			}
		}
	}
	for part in body.parts() {
		unbound_comparisons(part, known, warnings);
	}
}

fn names_nothing(word: &str, known: &HashSet<String>) -> bool {
	word != KEY_EVENT && word != EVENT_WORD && !known.contains(word) && compared_key_name(word).is_none()
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
fn named(body: Node, own: &HashSet<String>) -> Node {
	match body.drop_meta() {
		Node::Symbol(word) if own.contains(word) => body,
		Node::Symbol(word) if word == KEY_EVENT => key(symbol(EVENT_WORD), Op::Dot, symbol(KEY_EVENT)),
		Node::Symbol(word) => key_name(word).map(Node::Text).unwrap_or(body),
		// `key == space`: a side of a comparison also names a key in lowercase
		Node::Key(left, op @ (Op::Eq | Op::Ne), right) => {
			let [left, right] = [left, right].map(|side| named_compared(side.as_ref().clone(), own));
			key(left, *op, right)
		}
		// `event.key`: the field stays a field
		Node::Key(target, Op::Dot, field) => key(named(target.as_ref().clone(), own), Op::Dot, field.as_ref().clone()),
		_ => body.map_children(|child| named(child, own)),
	}
}

fn named_compared(side: Node, own: &HashSet<String>) -> Node {
	match side.symbol_name() {
		Some(word) if !own.contains(word) && word != KEY_EVENT => {
			compared_key_name(word).map(Node::Text).unwrap_or_else(|| named(side, own))
		}
		_ => named(side, own),
	}
}

/// A key word as written, or in lowercase (`space`, `esc`, `f1`): the lowercase form only where it is compared
fn compared_key_name(word: &str) -> Option<String> {
	let lowercase = word.chars().all(|letter| !letter.is_uppercase());
	key_name(word).or_else(|| lowercase.then(|| key_name(&word.to_uppercase())).flatten())
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
