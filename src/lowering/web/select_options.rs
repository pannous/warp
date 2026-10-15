//! A list inside `select{}` gives its options (card select-shorthand, notes/decisions.md): `select{ bind: fruit;
//! ["apple" "pear"] }` is `select{ bind: fruit option{ "apple" } option{ "pear" } }`, a map item `"s": "small"` the
//! option `option{ value: "s" "small" }`, a list variable `select{ fruits }` one option per item it holds when the page
//! shows. A list written right after the bound variable (`bind: fruit ["apple" "pear"]`) is the options, not part of it
//! (glued_attributes.rs).

use super::element_events::{element_items, with_element_items};
use super::nodes::key;
use crate::event_signals::is_element_event;
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::ON_WORD;

const SELECT: &str = "select";
const OPTION: &str = "option";
const VALUE: &str = "value";
/// the loop variable of a list variable's options
const OPTION_ITEM: &str = "listed_option";

pub fn lower(program: Node) -> Node {
	let program = program.map_children(lower);
	if !is_select(&program) {
		return program;
	}
	with_element_items(program, |items| items.into_iter().flat_map(options_of).collect())
}

fn is_select(node: &Node) -> bool {
	element_items(node).is_some() && matches!(node, Node::Key(tag, _, _) if tag.is_symbol(SELECT))
}

/// A select's item with the options it lists spelled out; any other item as it is
fn options_of(item: Node) -> Vec<Node> {
	match item.drop_meta() {
		Node::List(listed, Bracket::Square, _) if listed.iter().all(is_listed_option) => {
			listed.iter().map(option).collect()
		}
		// not the words of a handler `on change {…}`
		Node::Symbol(name) if name != ON_WORD && !is_element_event(name) => options_of_variable(name),
		_ => vec![item],
	}
}

/// `"apple"`, `"s": "small"`: an option's value, or its value and label
fn is_listed_option(listed: &Node) -> bool {
	let is_value =
		|node: &Node| matches!(node.drop_meta(), Node::Text(_) | Node::Number(_) | Node::Char(_) | Node::Symbol(_));
	match listed.drop_meta() {
		Node::Key(value, Op::Colon, label) => is_value(value) && is_value(label),
		other => is_value(other),
	}
}

/// `"apple"` as `option{ "apple" }`, `"s": "small"` as `option{ value: "s" "small" }`
fn option(listed: &Node) -> Node {
	let content = match listed.drop_meta() {
		Node::Key(value, Op::Colon, label) => {
			vec![key(symbol(VALUE), Op::Colon, value.as_ref().clone()), label.as_ref().clone()]
		}
		other => vec![other.clone()],
	};
	element(content)
}

/// `for listed_option in fruits { option{ listed_option } }`, as written in a select
fn options_of_variable(name: &str) -> Vec<Node> {
	let written = crate::warp_parser::parse_generated(&format!(
		"{SELECT}{{ for {OPTION_ITEM} in {name} {{ {OPTION}{{ {OPTION_ITEM} }} }} }}"
	));
	let mut items = vec![];
	written.visit(&mut |part| {
		if is_select(part) {
			items = element_items(part).cloned().unwrap_or_default();
		}
	});
	items
}

fn element(content: Vec<Node>) -> Node {
	key(symbol(OPTION), Op::Colon, Node::List(content, Bracket::Curly, Separator::None))
}
