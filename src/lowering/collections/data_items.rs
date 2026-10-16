//! Data stays clean (decision error-optional): every item but a literal written into a list or field is marked, and the
//! emitter fails where it holds an Error (wasm_emitter data_item); results, locals and arguments carry an Error on.
//! The writes marked: the items of a list literal `[e 2]`, `xs#1 = e`, `xs.add(e)`, `xs.insert(1, e)`, `q.k = e`, `m["k"] = e`, the values of a map
//! literal `q = {k: e}`, the items of a tuple literal `t = (e, 2)`; the emitter marks the fields of an instance (declared_values with_declared_field_types).
//! A place declared to admit an Error stays unmarked: a variable of the scope (`xs: [int or error] = …`, a parameter
//! `f(xs: [int?])`), a field `x: int or error` of the instance's class.
//! Only lists as written are marked: the lists later passes build (`guarded_call`'s arguments) hold what they are given

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

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
	let class_fields = crate::class_methods::class_fields(&node);
	let classes: Vec<String> = class_fields.keys().cloned().collect();
	let instances = crate::class_methods::instance_classes(&node, &classes);
	let mut variables = HashSet::new();
	collect_variables_admitting_errors(&node, &mut variables);
	let maps = map_variables(&node);
	Marker { variables_admitting_errors: variables, class_fields: &class_fields, instances: &instances, maps: &maps }
		.lower(node)
}

/// The variables given a map literal: `m = {a: 1}`, whose `m[k] = e` writes an entry
fn map_variables(node: &Node) -> HashSet<String> {
	let mut maps = HashSet::new();
	node.visit(&mut |part| {
		if let Node::Key(target, Op::Assign, value) = part {
			if let (Some(name), Node::List(_, Bracket::Curly, _)) = (target.symbol_name(), value.drop_meta()) {
				maps.insert(name.to_string());
			}
		}
	});
	maps
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

/// The variables of one scope declared to admit an Error (`xs: [int or error] = …`, a parameter `f(xs: [int?])`); a
/// function inside has its own scope
fn collect_variables_admitting_errors(node: &Node, names: &mut HashSet<String>) {
	match node {
		Node::Key(target, Op::Assign, value) => {
			if let Node::Key(name, Op::Colon, declared) = target.drop_meta() {
				if let Some(name) = name.symbol_name().filter(|_| node_admits_errors(declared)) {
					names.insert(name.to_string());
				}
			}
			collect_variables_admitting_errors(value, names);
		}
		Node::Key(_, Op::Define, _) | Node::Type { .. } => {}
		Node::Key(left, _, right) => {
			collect_variables_admitting_errors(left, names);
			collect_variables_admitting_errors(right, names);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| collect_variables_admitting_errors(item, names)),
		Node::Meta { node, .. } => collect_variables_admitting_errors(node, names),
		_ => {}
	}
}

/// The parameters of a definition head `f(xs: [int or error])` declared to admit an Error
fn parameters_admitting_errors(head: &Node, names: &mut HashSet<String>) {
	head.visit(&mut |part| {
		if let Node::Key(name, Op::Colon, declared) = part {
			if let Some(name) = name.symbol_name().filter(|_| node_admits_errors(declared)) {
				names.insert(name.to_string());
			}
		}
	});
}

struct Marker<'a> {
	variables_admitting_errors: HashSet<String>,
	/// each class's fields and their declared type names
	class_fields: &'a HashMap<String, Vec<(String, String)>>,
	/// the class of each variable holding an instance, whose fields' declared types decide
	instances: &'a HashMap<String, String>,
	maps: &'a HashSet<String>,
}

impl Marker<'_> {
	fn lower(&self, node: Node) -> Node {
		if let Node::Key(head, Op::Define, body) = node {
			return Node::Key(head.clone(), Op::Define, Box::new(self.inside_definition(&head).lower(*body)));
		}
		match crate::node::children_rewritten(node, |child| self.lower(child)) {
			Node::List(items, Bracket::Square, separator) => Node::List(
				items.into_iter().map(|item| marked(item, Place::ListItem)).collect(),
				Bracket::Square,
				separator,
			),
			Node::Key(target, Op::Assign, value) => {
				let value = self.written(&target, *value);
				Node::Key(Box::new(with_declared_type_unmarked(*target)), Op::Assign, Box::new(value))
			}
			// `a, b = e, 2` parses as `a, (b = e), 2`: values bound to variables, no tuple stored
			Node::List(items, bracket, Separator::Comma) if items.iter().any(is_assignment) => {
				Node::List(items.into_iter().map(with_assigned_value_unmarked).collect(), bracket, Separator::Comma)
			}
			Node::Key(value, Op::As, declared) if node_admits_errors(&declared) => {
				Node::Key(Box::new(unmarked(*value)), Op::As, declared)
			}
			Node::Key(receiver, Op::Dot, call) if !self.admits_errors(&receiver) => {
				Node::Key(receiver, Op::Dot, Box::new(with_written_items_marked(*call)))
			}
			other => other,
		}
	}

	/// The scope of a definition's body: its parameters and own variables, and the variables around it
	fn inside_definition(&self, head: &Node) -> Self {
		let mut variables = self.variables_admitting_errors.clone();
		parameters_admitting_errors(head, &mut variables);
		Marker { variables_admitting_errors: variables, ..*self }
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
			Node::Key(receiver, Op::Hash, index) if names_a_key(index) || self.is_map(receiver) => {
				marked(value, Place::Field)
			}
			Node::Key(_, Op::Hash, _) => marked(value, Place::ListItem),
			Node::Key(_, Op::Dot, _) => marked(value, Place::Field),
			Node::Symbol(_) | Node::Key(_, Op::Colon, _) => with_literal_items_marked(value),
			_ => value,
		}
	}

	/// Is the written place declared to admit an Error: its variable, or the field of an instance of a class
	fn admits_errors(&self, target: &Node) -> bool {
		match target.drop_meta() {
			Node::Symbol(name) => self.variables_admitting_errors.contains(name),
			Node::Key(left, Op::Hash, _) => self.admits_errors(left),
			Node::Key(instance, Op::Dot, field) => self.field_admits_errors(instance, field),
			_ => false,
		}
	}

	fn is_map(&self, receiver: &Node) -> bool {
		receiver.symbol_name().is_some_and(|name| self.maps.contains(name))
	}

	fn field_admits_errors(&self, instance: &Node, field: &Node) -> bool {
		let Some(class) = instance.symbol_name().and_then(|name| self.instances.get(name)) else {
			return false;
		};
		let fields = self.class_fields.get(class).map(Vec::as_slice).unwrap_or_default();
		fields.iter().any(|(name, type_name)| *name == field.drop_meta().name() && type_admits_errors(type_name))
	}
}

fn is_assignment(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(_, Op::Assign, _))
}

fn with_assigned_value_unmarked(node: Node) -> Node {
	node.under_meta(|node| match node {
		Node::Key(target, Op::Assign, value) => Node::Key(target, Op::Assign, Box::new(unmarked(*value))),
		other => other,
	})
}

/// `xs: [int] = …`: the type is no list literal
fn with_declared_type_unmarked(target: Node) -> Node {
	target.under_meta(|target| match target {
		Node::Key(name, Op::Colon, declared) => Node::Key(name, Op::Colon, Box::new(unmarked(*declared))),
		other => other,
	})
}

/// `{k: e}`, `(e, 2)`: the items of a map or tuple literal given to a variable (a list literal marks its own)
fn with_literal_items_marked(value: Node) -> Node {
	value.under_meta(|value| match value {
		Node::List(items, Bracket::Round, Separator::Comma) if items.len() > 1 => {
			let items = items.into_iter().map(|item| marked(item, Place::ListItem));
			Node::List(items.collect(), Bracket::Round, Separator::Comma)
		}
		other => with_map_values_marked(other),
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

/// `m["k"] = e` (shifted to `m#("k" + 1)` like a 0-based index): an entry of a map, no list item (as `m[k] = e` of a
/// map variable)
fn names_a_key(index: &Node) -> bool {
	match index.drop_meta() {
		Node::Text(_) | Node::Char(_) => true,
		Node::Key(key, Op::Add, _) => names_a_key(key),
		_ => false,
	}
}

/// `add(e)` of `xs.add(e)`, `insert(1, e)`: the items it writes marked as list items, not the named `at: i`
fn with_written_items_marked(call: Node) -> Node {
	call.under_meta(|call| match call {
		Node::List(mut items, bracket, separator) if writes_items(&items) => {
			let arguments = items.split_off(1).into_iter().map(|item| match item.drop_meta() {
				Node::Key(_, Op::Colon, _) => item,
				_ => marked(item, Place::ListItem),
			});
			items.extend(arguments);
			Node::List(items, bracket, separator)
		}
		other => other,
	})
}

fn writes_items(items: &[Node]) -> bool {
	items.first().and_then(Node::symbol_name).is_some_and(|name| {
		crate::list_mutations::appends(name, items.len() - 1) || name == crate::list_mutations::INSERT_METHOD
	})
}

/// The item marked as written into `place`, unless it is a literal or already marked
pub fn marked(item: Node, place: Place) -> Node {
	if may_hold_an_error(item.drop_meta()) && marked_item(&item).is_none() {
		Node::meta(item, Node::key(DATA_ITEM, Node::Symbol(place.word().to_string())))
	} else {
		item
	}
}

/// A list, map or tuple literal given to a place that admits an Error: its own items unmarked
fn unmarked(value: Node) -> Node {
	unmarked_item(value).under_meta(|value| match value {
		Node::List(items, bracket @ (Bracket::Square | Bracket::Curly | Bracket::Round), separator) => {
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
