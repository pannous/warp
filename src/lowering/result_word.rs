//! `result` is the value of the statement before it (wiki/result.md: "only used immediately before the keyword result
//! occurs"): that statement becomes `result = statement`, an assignment `x = v` is followed by `result = x`. A program
//! that assigns `result` itself keeps its own variable.

use super::nodes::key;
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;

const RESULT_WORD: &str = "result";

pub fn lower(node: Node) -> Node {
	if !mentions(&node) || assigns_result(&node) {
		return node;
	}
	with_results(node)
}

fn mentions(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= matches!(part, Node::Symbol(name) if name == RESULT_WORD));
	found
}

fn assigns_result(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= matches!(part, Node::Key(target, Op::Assign | Op::Define, _) if matches!(target.drop_meta(), Node::Symbol(name) if name == RESULT_WORD)));
	found
}

fn is_statement_list(bracket: &Bracket, separator: &Separator) -> bool {
	matches!(bracket, Bracket::None | Bracket::Curly) && matches!(separator, Separator::Semicolon | Separator::Newline)
}

fn result_of(value: Node) -> Node {
	key(symbol(RESULT_WORD), Op::Assign, value)
}

fn with_results(node: Node) -> Node {
	match node {
		Node::List(items, bracket, separator) if is_statement_list(&bracket, &separator) => {
			let items: Vec<Node> = items.into_iter().map(with_results).collect();
			let mut out: Vec<Node> = Vec::with_capacity(items.len());
			for item in items {
				if mentions(&item) {
					if let Some(previous) = out.pop() {
						out.extend(remembered(previous));
					}
				}
				out.push(item);
			}
			Node::List(out, bracket, separator)
		}
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(with_results).collect(), bracket, separator),
		Node::Key(left, op, right) => key(with_results(*left), op, with_results(*right)),
		Node::Meta { node, data } => Node::Meta { node: Box::new(with_results(*node)), data },
		other => other,
	}
}

/// The statement before a use of `result`, remembering its value
fn remembered(statement: Node) -> Vec<Node> {
	match statement.drop_meta() {
		// a definition has no value to remember
		Node::Key(head, Op::Define | Op::Assign, _) if matches!(head.drop_meta(), Node::List(..)) => vec![statement],
		Node::Key(target, Op::Assign | Op::Define, _) if matches!(target.drop_meta(), Node::Symbol(_)) => {
			let target = target.as_ref().clone();
			vec![statement, result_of(target)]
		}
		_ => vec![result_of(statement)],
	}
}
