//! Type aliases (card types-alias, samples/types/types.warp): `type Name = string` names the type string wherever a type is
//! written, `name: Name` in a class, `age: Age?`, `[Name]`, `f(n: Name)`, `a: Age = 3`; an alias of an alias is resolved
//! to the end, and `x is Age` tests the aliased type. Aliases of other shapes (`type Predicate = int -> bool`) stay declarations only.

use super::nodes::key;
use crate::node::Node;
use crate::operators::Op;
use std::collections::HashMap;

/// `Age?`: the optional form of a type name
const OPTIONAL_MARK: char = '?';

pub fn lower(node: Node) -> Node {
	let aliases = declared_aliases(&node);
	if aliases.is_empty() {
		return node;
	}
	resolved(node, &aliases)
}

/// The program's `type Name = target` declarations by name
fn declared_aliases(node: &Node) -> HashMap<String, String> {
	let mut aliases = HashMap::new();
	node.visit(&mut |part| aliases.extend(alias(part)));
	aliases
}

/// Each alias with the type it names in the end (`type Paint = Color`: Paint, Color)
pub(crate) fn resolved_aliases(node: &Node) -> HashMap<String, String> {
	let aliases = declared_aliases(node);
	aliases.keys().filter_map(|name| Some((name.clone(), target(name, &aliases)?))).collect()
}

/// `type Name = string`: (Name, string)
fn alias(node: &Node) -> Option<(String, String)> {
	let Node::Key(declared, Op::Assign, target) = node else {
		return None;
	};
	let Node::Type { name, body } = declared.drop_meta() else {
		return None;
	};
	let target = target.symbol_name()?;
	body.is_empty_node().then(|| (name.drop_meta().name(), target.to_string()))
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
		Node::Type { name, body } if body.is_empty_node() => match target(&name.drop_meta().name(), aliases) {
			Some(target) => Node::Type { name: Box::new(Node::Symbol(target)), body },
			None => Node::Type { name, body },
		},
		Node::Type { name, body } => Node::Type { name, body: Box::new(resolved(*body, aliases)) },
		// `x is Age` tests the type Age names; `x == Age` keeps the comparison
		Node::Key(subject, Op::Eq, right) if is_test(&subject) => {
			key(resolved(*subject, aliases), Op::Eq, annotated(*right, aliases))
		}
		// `x is an Age`: the items `x is an` and `Age`, the words after the test continue its type phrase
		Node::List(items, bracket, separator)
			if items.first().is_some_and(
				|first| matches!(first.drop_meta(), Node::Key(subject, Op::Eq, _) if is_test(subject)),
			) =>
		{
			let mut items = items.into_iter();
			let first = resolved(items.next().expect("guarded"), aliases);
			let phrase = std::iter::once(first).chain(items.map(|word| annotated(word, aliases)));
			Node::List(phrase.collect(), bracket, separator)
		}
		Node::Key(name, Op::Colon, annotation) => {
			key(resolved(*name, aliases), Op::Colon, annotated(*annotation, aliases))
		}
		other => other.map_children(|child| resolved(child, aliases)),
	}
}

/// `x is …`, as the parser marks the subject of `is`
fn is_test(subject: &Node) -> bool {
	crate::type_tests::compared_text(subject).is_some()
}

/// The type after a colon, `Name`, `[Name]`, `Age?`, with its alias names resolved; any other value as it is
fn annotated(annotation: Node, aliases: &HashMap<String, String>) -> Node {
	match annotation {
		Node::Symbol(written) => Node::Symbol(target(&written, aliases).unwrap_or(written)),
		Node::List(..) | Node::Meta { .. } => annotation.map_children(|child| annotated(child, aliases)),
		other => resolved(other, aliases),
	}
}
