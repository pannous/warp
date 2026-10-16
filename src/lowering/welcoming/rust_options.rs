//! Rust's Option read as warp (card rust-option, decision rust-forms-welcome): `Some(v)` is the value v and `None` ø
//! (the parser's constant), so `if let Some(v) = o { A } else { B }` and `match o { Some(v) => A, None => B }` are
//! `if o != ø { v = o; A } else { B }`; `o.unwrap_or(d)` is `o ?? d`, `o.expect(m)` `o ?? raise(m)`,
//! `o.is_some()` `o != ø`. Runs before lower_match_arms reads the `=>` arms.

use super::nodes::{call, key};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::{LET_WORD, MATCH_WORD, RAISE_WORD};

const SOME_WORD: &str = "Some";
const WILDCARD: &str = "_";
const UNWRAP_OR: &str = "unwrap_or";
const EXPECT: &str = "expect";
const IS_SOME: &str = "is_some";
const IS_NONE: &str = "is_none";

pub fn lower(program: Node) -> Node {
	// a program's own Some keeps its meaning
	if !mentions_option(&program) || crate::library_words::defined_names(&program).contains(SOME_WORD) {
		return program;
	}
	rewritten(program)
}

fn mentions_option(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |node| {
		found |= some_value(node).is_some()
			|| matches!(node, Node::Key(_, Op::Dot, method) if option_method(method).is_some())
	});
	found
}

fn rewritten(node: Node) -> Node {
	if let Some(lowered) = if_let_some(&node).or_else(|| option_match(&node)) {
		return rewritten(lowered);
	}
	let node = node.map_children(rewritten);
	match node.drop_meta() {
		Node::Key(receiver, Op::Dot, method) => match option_method(method) {
			Some((IS_SOME, _)) => key(receiver.as_ref().clone(), Op::Ne, Node::Empty),
			Some((IS_NONE, _)) => key(receiver.as_ref().clone(), Op::Eq, Node::Empty),
			Some((UNWRAP_OR, Some(fallback))) => key(receiver.as_ref().clone(), Op::Coalesce, fallback),
			Some((EXPECT, Some(message))) => {
				key(receiver.as_ref().clone(), Op::Coalesce, call(RAISE_WORD, vec![message]))
			}
			_ => node,
		},
		_ => some_value(&node).unwrap_or(node),
	}
}

/// `Some(v)`: v
fn some_value(node: &Node) -> Option<Node> {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 2 && items[0].is_symbol(SOME_WORD) => {
			Some(items[1].clone())
		}
		_ => None,
	}
}

/// `o.unwrap_or(d)`, `o.is_some()`: the method's name and its one argument
fn option_method(method: &Node) -> Option<(&'static str, Option<Node>)> {
	let Node::List(items, Bracket::Round, _) = method.drop_meta() else {
		return None;
	};
	let name = items.first()?.name();
	let argument = items.get(1).cloned();
	[(UNWRAP_OR, 2), (EXPECT, 2), (IS_SOME, 1), (IS_NONE, 1)]
		.into_iter()
		.find(|(word, arity)| *word == name && items.len() == *arity)
		.map(|(word, _)| (word, argument))
}

/// `if let Some(v) = o { A } else { B }`, parsed as `if (let Some(v)) == o then {A} else {B}`
fn if_let_some(node: &Node) -> Option<Node> {
	let (head, otherwise) = match node.drop_meta() {
		Node::Key(head, Op::Else, otherwise) => (head.as_ref(), Some(otherwise.as_ref().clone())),
		head => (head, None),
	};
	let Node::Key(condition, Op::Then, body) = head.drop_meta() else {
		return None;
	};
	let Node::Key(nothing, Op::If, comparison) = condition.drop_meta() else {
		return None;
	};
	let Node::Key(pattern, Op::Eq, option) = comparison.drop_meta() else {
		return None;
	};
	let Node::List(words, _, _) = pattern.drop_meta() else {
		return None;
	};
	let [word, some] = words.as_slice() else {
		return None;
	};
	if !nothing.drop_meta().is_empty_node() || !word.is_symbol(LET_WORD) {
		return None;
	}
	let name = some_value(some)?;
	Some(tested(option, &name, body, otherwise))
}

/// `match o { Some(v) => A, None => B }`: arms of Some and None (or `_`) only
fn option_match(node: &Node) -> Option<Node> {
	let Node::List(items, _, _) = node.drop_meta() else {
		return None;
	};
	let [word, option, arms] = items.as_slice() else {
		return None;
	};
	let Node::List(arms, Bracket::Curly, _) = arms.drop_meta() else {
		return None;
	};
	if !word.is_symbol(MATCH_WORD) {
		return None;
	}
	let (mut some_arm, mut none_arm) = (None, None);
	for arm in arms {
		let Node::Key(pattern, Op::Arrow | Op::FatArrow, value) = arm.drop_meta() else {
			return None;
		};
		match some_value(pattern) {
			Some(name) => some_arm = Some((name, value.as_ref().clone())),
			None if pattern.drop_meta().is_empty_node() || pattern.is_symbol(WILDCARD) => {
				none_arm = Some(value.as_ref().clone())
			}
			None => return None,
		}
	}
	let (name, value) = some_arm?;
	Some(tested(option, &name, &value, Some(none_arm.unwrap_or(Node::Empty))))
}

/// `if o != ø { v = o; body } else { otherwise }`
fn tested(option: &Node, name: &Node, body: &Node, otherwise: Option<Node>) -> Node {
	let condition = key(Node::Empty, Op::If, key(option.clone(), Op::Ne, Node::Empty));
	let binding = key(name.clone(), Op::Assign, option.clone());
	let statements = match body.drop_meta() {
		Node::List(items, Bracket::Curly, _) => [vec![binding], items.clone()].concat(),
		other => vec![binding, other.clone()],
	};
	let then = key(condition, Op::Then, Node::List(statements, Bracket::Curly, Separator::Semicolon));
	match otherwise {
		Some(otherwise) => key(then, Op::Else, otherwise),
		None => then,
	}
}
