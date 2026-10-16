//! Data stays clean (decision error-optional): every item but a literal written into a list or field is marked, and the
//! emitter fails where it holds an Error (wasm_emitter data_item); results, locals and arguments carry an Error on.
//! The writes marked: the items of a list literal `[e 2]`, `xs#1 = e`, `xs.add(e)`, `q.k = e`, the values of a map
//! literal `q = {k: e}`; the emitter marks the fields of an instance (declared_values with_declared_field_types).
//! A place declared to admit an Error (`xs: [int or error]`, `[int?]`, a field `x: int or error`) stays unmarked.
//! Only lists as written are marked: the lists later passes build (`guarded_call`'s arguments) hold what they are given

use crate::lowering::nodes::key;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use std::collections::HashSet;

/// Meta key marking an item written into a list or field; its value names the place
pub const DATA_ITEM: &str = "data item";

/// The place an item is written into, named in the runtime error
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Place {
	ListItem,
	Field,
}

impl Place {
	fn word(self) -> &'static str {
		match self {
			Place::ListItem => "list item",
			Place::Field => "field",
		}
	}
}

pub fn lower(node: Node) -> Node {
	let mut admitting = HashSet::new();
	collect_names_admitting_errors(&node, &mut admitting);
	Marker { names_admitting_errors: admitting }.lower(node)
}

/// Does a declared type admit an Error: `int or error`, `[int or error]`, the optional `int?` (decision error-optional)
pub fn type_admits_errors(type_name: &str) -> bool {
	type_name
		.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '?'))
		.any(|word| word == "error" || word.ends_with('?'))
}

fn node_admits_errors(declared: &Node) -> bool {
	type_admits_errors(&declared.serialize())
}

/// The variables and fields declared to admit an Error, anywhere: `xs: [int or error]`, `class p{x: int?}`
fn collect_names_admitting_errors(node: &Node, names: &mut HashSet<String>) {
	match node {
		Node::Key(name, Op::Colon, declared) => {
			if let Some(name) = name.symbol_name().filter(|_| node_admits_errors(declared)) {
				names.insert(name.to_string());
			}
			collect_names_admitting_errors(name, names);
			collect_names_admitting_errors(declared, names);
		}
		Node::Key(left, _, right) | Node::Type { name: left, body: right } => {
			collect_names_admitting_errors(left, names);
			collect_names_admitting_errors(right, names);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| collect_names_admitting_errors(item, names)),
		Node::Meta { node, .. } => collect_names_admitting_errors(node, names),
		_ => {}
	}
}

struct Marker {
	names_admitting_errors: HashSet<String>,
}

impl Marker {
	fn lower(&self, node: Node) -> Node {
		match crate::node::children_rewritten(node, |child| self.lower(child)) {
			Node::List(items, Bracket::Square, separator) => Node::List(
				items.into_iter().map(|item| marked(item, Place::ListItem)).collect(),
				Bracket::Square,
				separator,
			),
			Node::Key(target, Op::Assign, value) => {
				let value = self.written(&target, *value);
				key(with_declared_type_unmarked(*target), Op::Assign, value)
			}
			Node::Key(value, Op::As, declared) if node_admits_errors(&declared) => {
				Node::Key(Box::new(unmarked(*value)), Op::As, declared)
			}
			Node::Key(receiver, Op::Dot, call) if !self.admits_errors(&receiver) => {
				Node::Key(receiver, Op::Dot, Box::new(with_added_items_marked(*call)))
			}
			other => other,
		}
	}

	/// The value of `target = value`, marked as what the target's place holds
	fn written(&self, target: &Node, value: Node) -> Node {
		if self.admits_errors(target) {
			return unmarked(value);
		}
		match target.drop_meta() {
			Node::Key(declared_target, Op::Colon, declared)
				if node_admits_errors(declared) || self.admits_errors(declared_target) =>
			{
				unmarked(value)
			}
			Node::Key(_, Op::Hash, _) => marked(value, Place::ListItem),
			Node::Key(_, Op::Dot, _) => marked(value, Place::Field),
			Node::Symbol(_) => with_map_values_marked(value),
			_ => value,
		}
	}

	/// Is the written place, or its variable or field, declared to admit an Error
	fn admits_errors(&self, target: &Node) -> bool {
		match target.drop_meta() {
			Node::Symbol(name) => self.names_admitting_errors.contains(name),
			Node::Key(left, Op::Hash, _) => self.admits_errors(left),
			Node::Key(left, Op::Dot, field) => self.admits_errors(left) || self.admits_errors(field),
			_ => false,
		}
	}
}

/// `xs: [int] = …`: the type is no list literal
fn with_declared_type_unmarked(target: Node) -> Node {
	target.under_meta(|target| match target {
		Node::Key(name, Op::Colon, declared) => Node::Key(name, Op::Colon, Box::new(unmarked(*declared))),
		other => other,
	})
}

/// `{k: e}`: each value of a map literal marked as a field, nested maps through their own values
fn with_map_values_marked(value: Node) -> Node {
	value.under_meta(|value| match value {
		Node::List(entries, Bracket::Curly, separator)
			if entries.iter().all(|entry| matches!(entry.drop_meta(), Node::Key(_, Op::Colon, _))) =>
		{
			let entries = entries.into_iter().map(|entry| {
				entry.under_meta(|entry| match entry {
					Node::Key(key, Op::Colon, value) => Node::Key(key, Op::Colon, Box::new(field_marked(*value))),
					other => other,
				})
			});
			Node::List(entries.collect(), Bracket::Curly, separator)
		}
		other => other,
	})
}

fn field_marked(value: Node) -> Node {
	match value.drop_meta() {
		Node::List(_, Bracket::Curly, _) => with_map_values_marked(value),
		_ => marked(value, Place::Field),
	}
}

/// `add(e)` of `xs.add(e)`: its arguments marked as list items
fn with_added_items_marked(call: Node) -> Node {
	call.under_meta(|call| match call {
		Node::List(mut items, bracket, separator) if call_appends(&items) => {
			let arguments = items.split_off(1).into_iter().map(|item| marked(item, Place::ListItem));
			items.extend(arguments);
			Node::List(items, bracket, separator)
		}
		other => other,
	})
}

fn call_appends(items: &[Node]) -> bool {
	items.first().and_then(Node::symbol_name).is_some_and(|name| crate::list_mutations::appends(name, items.len() - 1))
}

/// The item marked as written into `place`, unless it is a literal or already marked
pub fn marked(item: Node, place: Place) -> Node {
	if may_hold_an_error(item.drop_meta()) && marked_item(&item).is_none() {
		Node::meta(item, Node::key(DATA_ITEM, Node::Symbol(place.word().to_string())))
	} else {
		item
	}
}

/// A list or map literal given to a place that admits an Error: its own items unmarked
fn unmarked(value: Node) -> Node {
	unmarked_item(value).under_meta(|value| match value {
		Node::List(items, bracket @ (Bracket::Square | Bracket::Curly), separator) => {
			Node::List(items.into_iter().map(unmarked_item).collect(), bracket, separator)
		}
		other => other,
	})
}

fn unmarked_item(item: Node) -> Node {
	match item {
		Node::Meta { node, data } if is_mark(&data) => *node,
		Node::Meta { node, data } => Node::Meta { node: Box::new(unmarked_item(*node)), data },
		Node::Key(key, Op::Colon, value) => Node::Key(key, Op::Colon, Box::new(unmarked_item(*value))),
		other => other,
	}
}

/// Any item but a literal, a nested list (whose items are marked themselves) and a function:
/// `e`, `f(x)`, `e+1`, `c ? e : 1`
fn may_hold_an_error(item: &Node) -> bool {
	!matches!(
		item,
		Node::Number(_)
			| Node::Text(_)
			| Node::Char(_)
			| Node::True
			| Node::False
			| Node::Empty
			| Node::List(_, Bracket::Square | Bracket::Curly, _)
			| Node::Key(_, Op::Arrow | Op::Define, _)
	)
}

fn is_mark(data: &Node) -> bool {
	matches!(data, Node::Key(key, _, _) if key.name() == DATA_ITEM)
}

/// The item under the mark, through any other metadata around it, and the place it is written into
pub fn marked_item(node: &Node) -> Option<(&Node, Place)> {
	let Node::Meta { node: inner, data } = node else {
		return None;
	};
	match data.as_ref() {
		Node::Key(key, _, place) if key.name() == DATA_ITEM => {
			Some((inner, if place.name() == Place::Field.word() { Place::Field } else { Place::ListItem }))
		}
		_ => marked_item(inner),
	}
}
