//! Key names in a key handler (card key-if): inside `on key {…}` a bare `key` is the pressed key, `event.key`, and the
//! key words name keys as the page's KeyboardEvent.key does (natively lowering/signals/system_signals.rs KEY_NAMES), so
//! `on key { if key == ESC { exit } }` reads as written. A program's own variable (`key`, `UP`) stays its own. A
//! comparison with a word that names nothing (`key == ESCAPEE`, `key == q`) never holds, which a warning says.
//! The key words compare with `held_key` anywhere too (card held-key): `held_key == LEFT` is `held_key == "ArrowLeft"`.
//! Key words and compared texts name keys in any case: ESC, Esc, escape, "escape" and "Escape" are Escape; a single
//! character stays itself, "a" and "A" being different presses.

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
/// The key a program keeps once it names it (card escape-window)
const ESCAPE_NAME: &str = "Escape";
const FUNCTION_KEYS: std::ops::RangeInclusive<u8> = 1..=12;

pub fn lower(program: Node) -> Node {
	let (statements, _, _) = crate::variable_signals::main_statements(&program);
	let mut warnings = vec![];
	let mut escape = false;
	let named = with_key_names(program, &main_level_variables(&statements), &mut warnings, &mut escape);
	reported(named, &warnings, escape)
}

/// `held_key == LEFT`, `ESC != held_key`: before system_values reads `held_key`
pub fn lower_held_comparisons(program: Node) -> Node {
	let (statements, _, _) = crate::variable_signals::main_statements(&program);
	let mut warnings = vec![];
	let mut escape = false;
	let named = held_comparisons(program, &main_level_variables(&statements), &mut warnings, &mut escape);
	reported(named, &warnings, escape)
}

/// The program with its key names, or the warnings' error; one comparing a key with Escape keeps that key (card
/// escape-window: else Escape over the paint window ends the run)
fn reported(named: Node, warnings: &[Diagnostic], escape: bool) -> Node {
	match crate::errors::diagnostic::report(warnings) {
		Ok(()) if escape => crate::lowering::system_signals::keeping_escape(named),
		Ok(()) => named,
		Err(error) => error,
	}
}

fn held_comparisons(node: Node, own: &HashSet<String>, warnings: &mut Vec<Diagnostic>, escape: &mut bool) -> Node {
	match node.drop_meta() {
		Node::Key(left, op @ (Op::Eq | Op::Ne), right) if left.is_symbol(HELD_KEY) || right.is_symbol(HELD_KEY) => {
			let [left, right] = [left, right].map(|side| held_compared(side, own, warnings, escape));
			key(left, *op, right)
		}
		_ => node.map_children(|child| held_comparisons(child, own, warnings, escape)),
	}
}

/// A side of a comparison with `held_key`: a key word becomes its name; a capitalized word naming nothing (`LEFTT`) warns,
/// a lowercase one stays for the undefined-variable error
fn held_compared(side: &Node, own: &HashSet<String>, warnings: &mut Vec<Diagnostic>, escape: &mut bool) -> Node {
	match side.symbol_name() {
		Some(word) if word == HELD_KEY || own.contains(word) => side.clone(),
		Some(word) => match key_name(word, escape) {
			Some(name) => Node::Text(name),
			None => {
				if word.chars().all(|letter| !letter.is_lowercase()) {
					warnings.push(names_nothing_warning(side, HELD_KEY, word));
				}
				side.clone()
			}
		},
		None => text_key_name(side, escape).unwrap_or_else(|| side.clone()),
	}
}

/// `key == "escape"`: a compared text naming a key in any case is that key's name, as a word naming it is; one
/// character stays itself ("a" and "A" are different presses)
fn text_key_name(side: &Node, escape: &mut bool) -> Option<Node> {
	match side.drop_meta() {
		Node::Text(text) if text.chars().count() > 1 => key_name(text, escape).map(Node::Text),
		_ => None,
	}
}

fn names_nothing_warning(side: &Node, compared: &str, word: &str) -> Diagnostic {
	Diagnostic::at(side, format!("{compared}: {word} names nothing, so the comparison never holds; write a key name (ESC, ENTER, UP, F1 …) or a text such as \"{word}\"")).severe()
}

fn with_key_names(node: Node, own: &HashSet<String>, warnings: &mut Vec<Diagnostic>, escape: &mut bool) -> Node {
	match key_handler_body(&node) {
		Some(index) => {
			let Node::List(mut items, bracket, separator) = node.drop_meta().clone() else {
				unreachable!("a handler is a list")
			};
			let body = &items[index];
			let mut known = own.clone();
			known.extend(body.assigned_names::<Vec<String>>(|op| matches!(op, Op::Assign | Op::Define)));
			unbound_comparisons(body, &known, warnings);
			items[index] = named(body.clone(), own, escape);
			Node::List(items, bracket, separator)
		}
		None => node.map_children(|child| with_key_names(child, own, warnings, escape)),
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
	word != KEY_EVENT && word != EVENT_WORD && !known.contains(word) && known_key_name(word).is_none()
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
fn named(body: Node, own: &HashSet<String>, escape: &mut bool) -> Node {
	match body.drop_meta() {
		Node::Symbol(word) if own.contains(word) => body,
		Node::Symbol(word) if word == KEY_EVENT => key(symbol(EVENT_WORD), Op::Dot, symbol(KEY_EVENT)),
		Node::Symbol(word) => key_name(word, escape).map(Node::Text).unwrap_or(body),
		Node::Key(left, op @ (Op::Eq | Op::Ne), right) => {
			let [left, right] = [left, right]
				.map(|side| text_key_name(side, escape).unwrap_or_else(|| named(side.as_ref().clone(), own, escape)));
			key(left, *op, right)
		}
		// `event.key`: the field stays a field
		Node::Key(target, Op::Dot, field) => {
			key(named(target.as_ref().clone(), own, escape), Op::Dot, field.as_ref().clone())
		}
		_ => body.map_children(|child| named(child, own, escape)),
	}
}

/// The key a key word names; `escape` notes Escape among them
fn key_name(word: &str, escape: &mut bool) -> Option<String> {
	let name = known_key_name(word);
	*escape |= name.as_deref() == Some(ESCAPE_NAME);
	name
}

/// The key a word or text names, in any case: ESC, Esc, escape and "Escape" alike, f1 as F1
fn known_key_name(word: &str) -> Option<String> {
	let function_key = word
		.strip_prefix(['F', 'f'])
		.and_then(|number| number.parse::<u8>().ok())
		.filter(|number| FUNCTION_KEYS.contains(number));
	KEY_WORDS
		.iter()
		.find(|(name, _)| name.eq_ignore_ascii_case(word))
		.map(|(_, key)| key.to_string())
		.or_else(|| function_key.map(|number| format!("F{number}")))
}
