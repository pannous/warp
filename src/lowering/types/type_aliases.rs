//! Type aliases (card types-alias, samples/types/types.warp): `type Name = string` names the type string wherever a type is
//! written, `name: Name` in a class, `age: Age?`, `[Name]`, `f(n: Name)`, `a: Age = 3`; an alias of an alias is resolved
//! to the end, and `x is Age` tests the aliased type; `type Code = list of int` names a type phrase. Aliases of other
//! shapes (`type Predicate = int -> bool`) stay declarations only.

use super::nodes::key;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::OF_WORD;
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

/// `type Name = string`: (Name, string); `type Code = list of int`: (Code, the type word `list of int`)
fn alias(node: &Node) -> Option<(String, String)> {
	let Node::Key(declared, Op::Assign, target) = node else {
		return None;
	};
	let Node::Type { name, body } = declared.drop_meta() else {
		return None;
	};
	let target = type_word(target)?;
	body.is_empty_node().then(|| (name.drop_meta().name(), target))
}

/// `type Name`, without a body
fn alias_declares(declared: &Node) -> bool {
	matches!(declared.drop_meta(), Node::Type { body, .. } if body.is_empty_node())
}

/// A type written as a word or as a phrase of words, `list of int`
fn type_word(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::List(words, Bracket::None, Separator::Space) => {
			let words: Option<Vec<&str>> = words.iter().map(|word| word.drop_meta().symbol_name()).collect();
			Some(words?.join(" "))
		}
		other => other.symbol_name().map(str::to_string),
	}
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

/// A type word or phrase with each alias in it resolved: `list of Name` is `list of text`
fn phrase_resolved(written: &str, aliases: &HashMap<String, String>) -> String {
	let words = written.split(' ').map(|word| target(word, aliases).unwrap_or_else(|| word.to_string()));
	words.collect::<Vec<_>>().join(" ")
}

fn resolved(node: Node, aliases: &HashMap<String, String>) -> Node {
	match node {
		// the phrase as one word: `list of int` is no call of list
		Node::Key(declared, Op::Assign, target) if type_word(&target).is_some() && alias_declares(&declared) => {
			let word = type_word(&target).expect("guarded");
			Node::Key(declared, Op::Assign, Box::new(Node::Symbol(word).with_meta_of(&target)))
		}
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
		// `f(xs: list of Name)`, parsed as the items `xs: list`, `of`, `Name`: the word after `of` is a type
		Node::List(items, bracket, separator) if items.iter().any(|item| item.is_symbol(OF_WORD)) => {
			let mut after_of = false;
			let items = items.into_iter().map(|item| {
				let typed = std::mem::replace(&mut after_of, item.is_symbol(OF_WORD));
				if typed {
					annotated(item, aliases)
				} else {
					resolved(item, aliases)
				}
			});
			Node::List(items.collect(), bracket, separator)
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
		Node::Symbol(written) => Node::Symbol(phrase_resolved(&written, aliases)),
		Node::List(..) | Node::Meta { .. } => annotation.map_children(|child| annotated(child, aliases)),
		other => resolved(other, aliases),
	}
}
