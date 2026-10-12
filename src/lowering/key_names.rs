//! Key names in a key handler (card key-if): inside `on key {…}` a bare `key` is the pressed key, `event.key`, and the
//! key words name keys as the page's KeyboardEvent.key does (natively lowering/system_signals.rs KEY_NAMES), so
//! `on key { if key == ESC { exit } }` reads as written. A program's own variable `key` stays its own.

use super::nodes::key;
use crate::event_signals::{main_level_variables, EVENT_WORD};
use crate::node::{symbol, Node};
use crate::operators::Op;

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
	let own_key = main_level_variables(&statements).contains(KEY_EVENT);
	with_key_names(program, own_key)
}

fn with_key_names(node: Node, own_key: bool) -> Node {
	match key_handler_body(&node) {
		Some(index) => {
			let Node::List(mut items, bracket, separator) = node.drop_meta().clone() else {
				unreachable!("a handler is a list")
			};
			items[index] = named(items[index].clone(), own_key);
			Node::List(items, bracket, separator)
		}
		None => node.map_children(|child| with_key_names(child, own_key)),
	}
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

/// The body with `key` as event.key (unless the program has its own) and the key words as the keys' names
fn named(body: Node, own_key: bool) -> Node {
	match body.drop_meta() {
		Node::Symbol(word) if word == KEY_EVENT && !own_key => key(symbol(EVENT_WORD), Op::Dot, symbol(KEY_EVENT)),
		Node::Symbol(word) => key_name(word).map(Node::Text).unwrap_or(body),
		// `event.key`: the field stays a field
		Node::Key(target, Op::Dot, field) => {
			key(named(target.as_ref().clone(), own_key), Op::Dot, field.as_ref().clone())
		}
		_ => body.map_children(|child| named(child, own_key)),
	}
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
