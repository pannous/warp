//! The small node forms many lowering passes build and test: a call of a word, a word

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

pub(crate) use crate::node::{int, symbol};

/// `function(arguments…)`
pub(crate) fn call(function: &str, arguments: Vec<Node>) -> Node {
	Node::List([vec![symbol(function)], arguments].concat(), Bracket::Round, Separator::None)
}

pub(crate) fn key(left: Node, op: Op, right: Node) -> Node {
	Node::Key(Box::new(left), op, Box::new(right))
}

pub(crate) fn assign(target: Node, value: Node) -> Node {
	key(target, Op::Assign, value)
}

/// Statements run in order: `a; b`, `{a; b}`
pub(crate) fn statement_list(statements: Vec<Node>, bracket: Bracket) -> Node {
	Node::List(statements, bracket, Separator::Semicolon)
}

/// `int`, `float`, `text`, …: a word naming a type
pub(crate) fn is_type_word(node: &Node) -> bool {
	node.symbol_name().is_some_and(|word| crate::analyzer::type_word_kind(word).is_some())
}

/// The name a parameter declares: `x`, `x:int`, `x=1`
pub(crate) fn parameter_name(parameter: &Node) -> Option<String> {
	match parameter.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		Node::Key(name, _, _) => parameter_name(name),
		_ => None,
	}
}

/// `f(…)`: a definition's head naming a function
pub(crate) fn is_call_head(head: &Node) -> bool {
	matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if items.first().and_then(Node::symbol_name).is_some())
}

/// `name words… last = body`, parsed as the items `name`, `words…`, `last = body`: the name, the words through `last`
/// and the body
pub(crate) fn spaced_statement(items: &[Node]) -> Option<(&Node, Vec<&str>, &Node)> {
	let (name, rest) = items.split_first()?;
	let (last, middle) = rest.split_last()?;
	let Node::Key(last_word, Op::Assign | Op::Define, body) = last.drop_meta() else { return None };
	let words = middle.iter().chain(std::iter::once(last_word.as_ref())).map(Node::symbol_name).collect::<Option<_>>()?;
	Some((name, words, body))
}

/// The node with `rewrite` applied to its direct children: a list's items, a key's left then right, the node under its
/// metadata; any other node as it is. Unlike Node::map_children it leaves a class body (Node::Type) alone
pub(crate) fn children_rewritten(node: Node, mut rewrite: impl FnMut(Node) -> Node) -> Node {
	match node {
		Node::Key(left, op, right) => {
			let left = rewrite(*left);
			key(left, op, rewrite(*right))
		}
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(rewrite).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(rewrite(*node)), data },
		other => other,
	}
}
