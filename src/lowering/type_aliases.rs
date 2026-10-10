//! Type aliases (card types-alias, samples/types.warp): `type Name = string` names the type string wherever a type is
//! written, `name: Name` in a class, `age: Age?`, `[Name]`, `f(n: Name)`, `a: Age = 3`; an alias of an alias is resolved
//! to the end. Aliases of other shapes (`type Predicate = int -> bool`) stay declarations only.

use super::nodes::key;
use crate::node::Node;
use crate::operators::Op;
use std::collections::HashMap;

/// `Age?`: the optional form of a type name
const OPTIONAL_MARK: char = '?';

pub fn lower(node: Node) -> Node {
	let mut aliases = HashMap::new();
	node.visit(&mut |part| aliases.extend(alias(part)));
	if aliases.is_empty() {
		return node;
	}
	resolved(node, &aliases)
}

/// `type Name = string`: (Name, string)
fn alias(node: &Node) -> Option<(String, String)> {
	let Node::Key(declared, Op::Assign, target) = node else { return None };
	let Node::Type { name, body } = declared.drop_meta() else { return None };
	let Node::Symbol(target) = target.drop_meta() else { return None };
	matches!(body.drop_meta(), Node::Empty).then(|| (name.drop_meta().name(), target.clone()))
}

/// The type an alias names in the end, `Age?` of `Years?` when `type Years = Age; type Age = int` gives `int?`
fn target(written: &str, aliases: &HashMap<String, String>) -> Option<String> {
	let (name, optional) = match written.strip_suffix(OPTIONAL_MARK) {
		Some(name) => (name, OPTIONAL_MARK.to_string()),
		None => (written, String::new()),
	};
	let mut resolved = aliases.get(name)?;
	for _ in 0..aliases.len() {
		match aliases.get(resolved) {
			Some(next) => resolved = next,
			None => break,
		}
	}
	Some(format!("{resolved}{optional}"))
}

fn resolved(node: Node, aliases: &HashMap<String, String>) -> Node {
	match node {
		declaration @ Node::Key(..) if alias(&declaration).is_some() => declaration,
		Node::Type { name, body } if matches!(body.drop_meta(), Node::Empty) => match target(&name.drop_meta().name(), aliases) {
			Some(target) => Node::Type { name: Box::new(Node::Symbol(target)), body },
			None => Node::Type { name, body },
		},
		Node::Type { name, body } => Node::Type { name, body: Box::new(resolved(*body, aliases)) },
		Node::Key(name, Op::Colon, annotation) => key(resolved(*name, aliases), Op::Colon, annotated(*annotation, aliases)),
		other => other.map_children(|child| resolved(child, aliases)),
	}
}

/// The type after a colon, `Name`, `[Name]`, `Age?`, with its alias names resolved; any other value as it is
fn annotated(annotation: Node, aliases: &HashMap<String, String>) -> Node {
	match annotation {
		Node::Symbol(written) => Node::Symbol(target(&written, aliases).unwrap_or(written)),
		Node::List(..) | Node::Meta { .. } => annotation.map_children(|child| annotated(child, aliases)),
		other => resolved(other, aliases),
	}
}
