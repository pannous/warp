//! wiki/reference.md: inside a data literal `$a` names the enclosing node `a`, `$1` the enclosing node that declares
//! `@id:1` (`a[id=1]{…}`), so a literal describes a cyclic graph. The reference stays a name, so printing never loops.
//! A path read of a variable that holds such a literal follows the references at compile time: with
//! `x = a{ b:2 c{ parent=$a } }`, `x.c.parent.b` is `x.b`.

use crate::node::{Bracket, Node, ATTRIBUTE_MARK};
use crate::operators::Op;
use std::collections::HashMap;

const REFERENCE_MARK: char = '$';
const ID_KEY: &str = "id";

pub fn lower(node: Node) -> Node {
	let node = named_references(node, &mut Vec::new());
	let literals = referencing_literals(&node);
	if literals.is_empty() {
		return node;
	}
	followed_paths(node, &literals)
}

/// `name{…}`: the node's name and body items
fn named_body(node: &Node) -> Option<(String, &[Node])> {
	match node.drop_meta() {
		Node::Key(name, Op::Colon, body) if matches!(name.drop_meta(), Node::Symbol(_)) => match body.drop_meta() {
			Node::List(items, Bracket::Curly, _) => Some((name.name(), items)),
			_ => None,
		},
		_ => None,
	}
}

/// The id a body declares with `@id:n`
fn declared_id(items: &[Node]) -> Option<String> {
	items.iter().find_map(|item| match item.drop_meta() {
		Node::Key(key, Op::Colon, value) if key.name() == format!("{ATTRIBUTE_MARK}{ID_KEY}") => Some(value.serialize()),
		_ => None,
	})
}

/// `$1` of an enclosing node declaring `@id:1` becomes the name reference `$a`, so no pass takes it for Swift's
/// parameter `$1`; a `$n` no enclosing node declares stays the parameter
fn named_references(node: Node, enclosing: &mut Vec<(String, Option<String>)>) -> Node {
	if let Node::Symbol(word) = &node {
		let id = word.strip_prefix(REFERENCE_MARK).unwrap_or_default();
		if let Some((name, _)) = enclosing.iter().rev().find(|(_, declared)| !id.is_empty() && declared.as_deref() == Some(id)) {
			return Node::Symbol(format!("{REFERENCE_MARK}{name}"));
		}
		return node;
	}
	let Some((name, items)) = named_body(&node) else {
		return node.map_children(|child| named_references(child, enclosing));
	};
	enclosing.push((name, declared_id(items)));
	let node = node.map_children(|child| named_references(child, enclosing));
	enclosing.pop();
	node
}

/// Variables assigned once to a literal that refers to one of its enclosing nodes
fn referencing_literals(node: &Node) -> HashMap<String, Node> {
	let mut assignments: HashMap<String, Vec<&Node>> = HashMap::new();
	node.visit(&mut |part| {
		if let Node::Key(target, Op::Assign | Op::Define, value) = part {
			if let Node::Symbol(variable) = target.drop_meta() {
				assignments.entry(variable.clone()).or_default().push(value);
			}
		}
	});
	assignments.into_iter().filter_map(|(variable, values)| match values.as_slice() {
		[literal] if refers_back(literal, &mut Vec::new()) => Some((variable, literal.drop_meta().clone())),
		_ => None,
	}).collect()
}

fn refers_back(node: &Node, enclosing: &mut Vec<String>) -> bool {
	if let Node::Symbol(word) = node.drop_meta() {
		return word.strip_prefix(REFERENCE_MARK).is_some_and(|name| enclosing.iter().any(|enclosing_name| enclosing_name == name));
	}
	let named = named_body(node).map(|(name, _)| name);
	let pushed = named.is_some();
	enclosing.extend(named);
	let found = match node.drop_meta() {
		Node::Key(left, _, right) => refers_back(left, enclosing) || refers_back(right, enclosing),
		Node::List(items, _, _) => items.iter().any(|item| refers_back(item, enclosing)),
		_ => false,
	};
	if pushed {
		enclosing.pop();
	}
	found
}

/// `x.c.parent.b` of such a variable: the path with the references followed, `x.b`
fn followed_paths(node: Node, literals: &HashMap<String, Node>) -> Node {
	if let Some((variable, fields)) = dot_chain(&node) {
		if let Some(path) = literals.get(&variable).and_then(|literal| followed(literal, &fields)) {
			return path.into_iter().fold(Node::Symbol(variable), |reader, field| Node::Key(Box::new(reader), Op::Dot, Box::new(Node::Symbol(field))));
		}
	}
	node.map_children(|child| followed_paths(child, literals))
}

/// `x.c.parent`: the variable and its field names
fn dot_chain(node: &Node) -> Option<(String, Vec<String>)> {
	match node.drop_meta() {
		Node::Key(reader, Op::Dot, field) => {
			let Node::Symbol(field) = field.drop_meta() else { return None };
			let (variable, mut fields) = match reader.drop_meta() {
				Node::Symbol(variable) => (variable.clone(), Vec::new()),
				other => dot_chain(other)?,
			};
			fields.push(field.clone());
			Some((variable, fields))
		}
		_ => None,
	}
}

/// The fields from the literal's root to the node `fields` reaches, references followed; `None` when a field is
/// missing or the path follows no reference (it stays as written)
fn followed(literal: &Node, fields: &[String]) -> Option<Vec<String>> {
	let mut nodes: Vec<&Node> = vec![literal]; // the named node at each depth of `path`
	let mut path: Vec<String> = Vec::new();
	let mut followed_any = false;
	for field in fields {
		let (_, items) = named_body(nodes.last()?)?;
		let entry = items.iter().find(|item| matches!(item.drop_meta(), Node::Key(key, _, _) if key.name() == *field))?;
		let Node::Key(_, _, value) = entry.drop_meta() else { unreachable!("found a key") };
		match value.drop_meta() {
			Node::Symbol(word) if word.starts_with(REFERENCE_MARK) => {
				let name = &word[REFERENCE_MARK.len_utf8()..];
				let depth = nodes.iter().rposition(|node| named_body(node).is_some_and(|(node_name, _)| node_name == name))?;
				nodes.truncate(depth + 1);
				path.truncate(depth);
				followed_any = true;
			}
			_ => {
				nodes.push(entry);
				path.push(field.clone());
			}
		}
	}
	followed_any.then_some(path)
}
