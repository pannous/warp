//! A bare class word is a class reference (card bare-class, decision type-print): `class int` without a body is the
//! int type, as `function f` is the function, so a printed `class time` pasted back is time. A bodyless `class Shape`
//! the program uses elsewhere (Kotlin's `sealed class Shape` and its `: Shape()` subclasses) still declares it; one
//! naming nothing and used nowhere is an unknown-type error naming the nearest type. C#'s `class Foo;` stays the
//! file-scoped declaration (file_declarations.rs), but as a value (`x = class int; …`) it is the reference.

use super::nodes::children_rewritten;
use crate::extensions::strings::near_miss;
use crate::node::Node;
use crate::warp_parser::{EXTENDS_KEYWORD, IMPLEMENTS_WORD};
use std::collections::HashSet;

pub fn lower(node: Node) -> Node {
	if !has_bodyless_class(&node) {
		return node;
	}
	let declared = declared_classes(&node);
	let used = used_names(&node);
	Program { declared: &declared, used: &used }.rewritten(node, true)
}

struct Program<'a> {
	/// The classes declared with a body
	declared: &'a HashSet<String>,
	/// Every name the program mentions, a declaration's own name among them
	used: &'a [String],
}

impl Program<'_> {
	/// As a statement `class Foo` may declare (a file-scoped `class Foo;`, a marker class used elsewhere); as a value
	/// it is the reference
	fn rewritten(&self, node: Node, is_statement: bool) -> Node {
		if let Some(name) = bodyless_class(&node) {
			let declares = is_statement && (is_file_class(&node) || self.declares(&name));
			return if declares { unmarked(node) } else { self.reference(&name) };
		}
		match node {
			Node::List(items, bracket, separator) if separator.separates_statements() || is_statement && are_classes(&items) => {
				Node::List(items.into_iter().map(|item| self.rewritten(item, true)).collect(), bracket, separator)
			}
			other => children_rewritten(other, |child| self.rewritten(child, false)),
		}
	}

	/// An unknown class the program mentions again
	fn declares(&self, name: &str) -> bool {
		!self.is_known(name) && self.used.iter().filter(|used| *used == name).count() > 1
	}

	fn is_known(&self, name: &str) -> bool {
		self.declared.contains(name) || builtin(name).is_some()
	}

	fn reference(&self, name: &str) -> Node {
		if self.declared.contains(name) {
			return Node::Symbol(name.to_string());
		}
		if let Some(builtin) = builtin(name) {
			return Node::Symbol(builtin.to_string());
		}
		// `class Strng`: the built-in types are lowercase, so the name is compared lowercased with them
		let builtins = crate::type_tests::builtin_type_words().map(str::to_string);
		let near = near_miss(name, self.declared.iter().cloned()).or_else(|| near_miss(&name.to_lowercase(), builtins));
		let near = near.map(|near| format!("; did you mean {near}?"));
		crate::node::error(&format!("unknown type {name}: no class or type of that name{}", near.unwrap_or_default()))
	}
}

/// `class int`, Kotlin's `class Int`: the built-in type of that name, any case
fn builtin(name: &str) -> Option<&'static str> {
	crate::type_tests::builtin_type_name(name).or_else(|| crate::type_tests::builtin_type_name(&name.to_lowercase()))
}

/// `class Foo` without fields: its name
fn bodyless_class(node: &Node) -> Option<String> {
	let Node::Type { name, body } = node.drop_meta() else { return None };
	let empty = matches!(body.drop_meta(), Node::List(items, _, _) if items.is_empty()) || body.is_empty_node();
	let bare = name.attribute(crate::words::BARE_CLASS_MARK).is_some();
	let plain = name.attribute(EXTENDS_KEYWORD).is_none() && name.attribute(IMPLEMENTS_WORD).is_none();
	(empty && bare && plain).then(|| name.drop_meta().name())
}

/// `sealed class Shape` and the class after it, parsed as one statement
fn are_classes(items: &[Node]) -> bool {
	items.iter().all(|item| matches!(item.drop_meta(), Node::Type { .. }))
}

/// A bodyless declaration as the parser gives one otherwise, without BARE_CLASS_MARK
fn unmarked(node: Node) -> Node {
	match node {
		Node::Meta { node, data } => Node::Meta { node: Box::new(unmarked(*node)), data },
		Node::Type { name, body } => Node::Type { name: Box::new(without_bare_mark(*name)), body },
		other => other,
	}
}

fn without_bare_mark(name: Node) -> Node {
	match name {
		Node::Meta { node, data } if matches!(data.as_ref(), Node::Key(key, _, _) if key.name().ends_with(crate::words::BARE_CLASS_MARK)) => *node,
		Node::Meta { node, data } => Node::Meta { node: Box::new(without_bare_mark(*node)), data },
		other => other,
	}
}

fn is_file_class(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Type { name, .. } if name.attribute(crate::words::FILE_CLASS_MARK).is_some())
}

fn has_bodyless_class(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= bodyless_class(part).is_some());
	found
}

fn declared_classes(node: &Node) -> HashSet<String> {
	let mut declared = HashSet::new();
	node.visit(&mut |part| {
		if let Node::Type { name, .. } = part {
			if bodyless_class(part).is_none() {
				declared.insert(name.drop_meta().name());
			}
		}
	});
	declared
}

/// The names mentioned, also in attributes (`: Shape()`, `extends Shape`), which visit skips
fn used_names(node: &Node) -> Vec<String> {
	let mut used = vec![];
	collect_names(node, &mut used);
	used
}

fn collect_names(node: &Node, used: &mut Vec<String>) {
	for (_, value) in node.attributes() {
		collect_names(value, used);
	}
	match node.drop_meta() {
		Node::Symbol(name) => used.push(name.clone()),
		Node::Type { name, body } => {
			collect_names(name, used);
			collect_names(body, used);
		}
		other => other.parts().into_iter().for_each(|part| collect_names(part, used)),
	}
}
