//! The small node forms many lowering passes build and test: a call of a word, a word

use super::words::{FOR_WORD, IN_WORD};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::cell::Cell;

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

/// `play 440` alone or as the one statement of a block `{play 440}`: the words of a call (card statement-block)
pub(crate) fn is_spaced_call(bracket: &Bracket, separator: &Separator) -> bool {
	matches!(bracket, Bracket::None | Bracket::Curly) && *separator == Separator::Space
}

/// `f x`, `(f x)`, `{f x}`: words that call their head
pub(crate) fn is_words_call(bracket: &Bracket, separator: &Separator) -> bool {
	is_spaced_call(bracket, separator) || *bracket == Bracket::Round && *separator == Separator::Space
}

/// The call resolved from the words of `{play 440}` stays the block of its one statement, `{play(440)}`
pub(crate) fn in_block_as_written(bracket: &Bracket, call: Node) -> Node {
	match bracket {
		Bracket::Curly => Node::List(vec![call], Bracket::Curly, Separator::None),
		_ => call,
	}
}

/// `x = {a b}` assigns the data `{a b}`, no block of the statement `a b`: `x = {abs -3}` is that list
pub(crate) fn is_assigned_data(target: &Node, value: &Node) -> bool {
	matches!(target.drop_meta(), Node::Symbol(_)) && matches!(value.drop_meta(), Node::List(_, Bracket::Curly, _))
}

/// The parts of a node rewritten, the node itself kept as written
pub(crate) fn with_parts_rewritten(node: Node, rewrite: &mut dyn FnMut(Node) -> Node) -> Node {
	match node {
		Node::Meta { node, data } => Node::Meta { node: Box::new(with_parts_rewritten(*node, rewrite)), data },
		other => children_rewritten(other, rewrite),
	}
}

/// Statements run in order: `a; b`, `{a; b}`
pub(crate) fn statement_list(statements: Vec<Node>, bracket: Bracket) -> Node {
	Node::List(statements, bracket, Separator::Semicolon)
}

/// `{a; b}`
pub(crate) fn block(statements: Vec<Node>) -> Node {
	statement_list(statements, Bracket::Curly)
}

pub(crate) fn is_block(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(_, Bracket::Curly, _))
}

/// The words `for i in …` of a loop: i
pub(crate) fn for_in_variable(items: &[Node]) -> Option<String> {
	match items {
		[keyword, variable, within, ..] if keyword.is_symbol(FOR_WORD) && within.is_symbol(IN_WORD) => variable.symbol_name().map(String::from),
		_ => None,
	}
}

/// `if condition then body`
pub(crate) fn if_then(condition: Node, body: Node) -> Node {
	key(key(Node::Empty, Op::If, condition), Op::Then, body)
}

pub(crate) fn if_then_else(condition: Node, body: Node, otherwise: Node) -> Node {
	key(if_then(condition, body), Op::Else, otherwise)
}

/// `a: 1`
pub(crate) fn is_colon_pair(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(_, Op::Colon, _))
}

/// `=`, `:=` and the compound assignments `+=` …: the operators binding a name
pub(crate) fn is_binding(op: &Op) -> bool {
	matches!(op, Op::Assign | Op::Define) || op.is_compound_assign()
}

/// `def`, `fun`, `fn` …
pub(crate) fn is_function_keyword(node: &Node) -> bool {
	node.symbol_name().is_some_and(crate::operators::is_function_keyword)
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

/// A parameter as the parameters it stands for: a group `(a, b)` its items, ø none
pub(crate) fn grouped_parameters(parameter: &Node) -> Vec<Node> {
	match parameter.drop_meta() {
		Node::List(group, Bracket::Round, _) => group.clone(),
		Node::Empty => vec![],
		_ => vec![parameter.clone()],
	}
}

/// Numbers a pass's temporaries apart: each `next_number` the next one, from 0 or from where it starts
#[derive(Default)]
pub(crate) struct Counter(Cell<usize>);

impl Counter {
	pub(crate) const fn starting_at(first: usize) -> Self {
		Counter(Cell::new(first))
	}

	pub(crate) fn next_number(&self) -> usize {
		self.0.replace(self.0.get() + 1)
	}
}

/// `x = value`: the name and the value
pub(crate) fn named_assignment(statement: &Node) -> Option<(String, Node)> {
	match statement.drop_meta() {
		Node::Key(name, Op::Assign, value) => Some((name.symbol_name()?.to_string(), value.as_ref().clone())),
		_ => None,
	}
}
