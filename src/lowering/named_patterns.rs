//! Named patterns (card patterns-values, tree-patterns step 4, user decision: the keyword): `pattern Pair = [_, _]`
//! names a pattern. The definition leaves the program and the name stands for its pattern where a pattern goes:
//! `x is Pair`, an arm `Pair => …` or `Pair: …`, and `x is Pair and Short`, which tests x against both. A named pattern
//! binds its names like the written one (`pattern Point = [x, y]; match p { Point => x + y }`). A plain assignment is
//! always a value; one whose right side can only be a pattern (`Pair = [_, _]`) is the error whose fix is the keyword.

use super::nodes::{children_rewritten, key};
use super::switch::{is_switch_head, rest_pattern, split_switch, WILDCARD};
use crate::diagnostic::Diagnostic;
use crate::node::Node;
use crate::operators::Op;
use std::collections::HashMap;

const PATTERN_WORD: &str = "pattern";

type Patterns = HashMap<String, Node>;

pub fn lower(program: Node) -> Node {
	let Node::List(statements, bracket, separator) = program else {
		return program;
	};
	if let Some(error) = statements.iter().find_map(pattern_without_keyword) {
		return error;
	}
	let (definitions, statements): (Vec<Node>, Vec<Node>) = statements
		.into_iter()
		.partition(|statement| pattern_definition(statement).is_some());
	if definitions.is_empty() {
		return Node::List(statements, bracket, separator);
	}
	let patterns: Patterns = definitions.iter().filter_map(pattern_definition).collect();
	let statements = statements
		.into_iter()
		.map(|statement| substituted(statement, &patterns))
		.collect();
	Node::List(statements, bracket, separator)
}

/// `pattern Pair = [_, _]`: the name and its pattern
fn pattern_definition(statement: &Node) -> Option<(String, Node)> {
	let Node::List(parts, _, _) = statement.drop_meta() else {
		return None;
	};
	let [keyword, definition] = parts.as_slice() else {
		return None;
	};
	match definition.drop_meta() {
		Node::Key(name, Op::Assign, pattern) if keyword.is_symbol(PATTERN_WORD) => {
			Some((name.symbol_name()?.to_string(), pattern.as_ref().clone()))
		}
		_ => None,
	}
}

/// `Pair = [_, _]` assigns what only a pattern can be: the error whose fix is `pattern Pair = [_, _]`
fn pattern_without_keyword(statement: &Node) -> Option<Node> {
	match statement.drop_meta() {
		Node::Key(name, Op::Assign, pattern) if name.symbol_name().is_some() && is_open(pattern) => {
			let name = name.name();
			Some(
				Diagnostic::at(
					statement,
					format!("{name} is assigned a pattern, which is no value: name it with the keyword pattern"),
				)
				.fix(format!("{PATTERN_WORD} {name} = …"))
				.into_error(),
			)
		}
		_ => None,
	}
}

/// A pattern no value could be: it holds `_`, an unnamed rest `..` or an item pattern
fn is_open(pattern: &Node) -> bool {
	// a named rest `...xs` may be a spread of the list xs
	if rest_pattern(pattern) == Some(None) {
		return true;
	}
	match pattern.drop_meta() {
		Node::Symbol(name) => name == WILDCARD,
		Node::List(items, _, _) => items.iter().any(is_open),
		Node::Key(left, Op::Range | Op::To, end) if matches!(end.drop_meta(), Node::Empty) => {
			matches!(left.drop_meta(), Node::List(..))
		}
		Node::Key(_, Op::Colon, value) => is_open(value),
		_ => false,
	}
}

fn substituted(node: Node, patterns: &Patterns) -> Node {
	let node = children_rewritten(node, |child| substituted(child, patterns));
	match node {
		Node::Key(subject, Op::Eq, pattern) => Node::Key(subject, Op::Eq, Box::new(resolved(*pattern, patterns))),
		Node::Key(arm, Op::FatArrow, body) => Node::Key(Box::new(resolved(*arm, patterns)), Op::FatArrow, body),
		// `x is Pair and Short`: Short tests the same subject
		Node::Key(tests, op @ (Op::And | Op::Or), pattern) => {
			match (tested_subject(&tests), pattern_named(&pattern, patterns)) {
				(Some(subject), Some(pattern)) => {
					let test = key(subject.clone(), Op::Eq, pattern.clone());
					Node::Key(tests, op, Box::new(test))
				}
				_ => Node::Key(tests, op, pattern),
			}
		}
		Node::List(items, bracket, separator) => match split_switch(&items) {
			Some((head, _, _)) if is_switch_head(head) => {
				let items = items.into_iter().map(|item| with_case_keys(item, patterns)).collect();
				Node::List(items, bracket, separator)
			}
			_ => Node::List(items, bracket, separator),
		},
		other => other,
	}
}

/// The subject of the first test of `x is A and B`
fn tested_subject(tests: &Node) -> Option<&Node> {
	match tests.drop_meta() {
		Node::Key(subject, Op::Eq, _) => Some(subject),
		Node::Key(first, Op::And | Op::Or, _) => tested_subject(first),
		_ => None,
	}
}

fn pattern_named<'a>(node: &Node, patterns: &'a Patterns) -> Option<&'a Node> {
	patterns.get(node.drop_meta().symbol_name()?)
}

fn resolved(node: Node, patterns: &Patterns) -> Node {
	pattern_named(&node, patterns).cloned().unwrap_or(node)
}

/// The colon arms `Pair: …` of a switch's case block, which as an item may also hold its subject
fn with_case_keys(item: Node, patterns: &Patterns) -> Node {
	match item {
		Node::Key(arm, Op::Colon, body) => Node::Key(Box::new(resolved(*arm, patterns)), Op::Colon, body),
		Node::List(items, bracket, separator) => Node::List(
			items.into_iter().map(|item| with_case_keys(item, patterns)).collect(),
			bracket,
			separator,
		),
		Node::Meta { node, data } => Node::Meta {
			node: Box::new(with_case_keys(*node, patterns)),
			data,
		},
		other => other,
	}
}
