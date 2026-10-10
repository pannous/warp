//! `result` is the value of the statement before it (wiki/result.md: "only used immediately before the keyword result
//! occurs"): that statement becomes `result = statement`, an assignment `x = v` is followed by `result = x`. A program
//! that assigns `result` itself keeps its own variable. A print gives ø, so a statement ending in a print remembers
//! the printed value: `if c: print "hi" else: print "ho"; result` is "hi" (card result-keyword).

use super::nodes::{children_rewritten, key};
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::warp_parser::{print_arguments_of, print_call, starts_print};

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
	node.visit(&mut |part| found |= matches!(part, Node::Key(target, Op::Assign | Op::Define, _) if target.is_symbol(RESULT_WORD)));
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
		other => children_rewritten(other, with_results),
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
		_ if prints_last(&statement) => vec![remembering(statement)],
		_ => vec![result_of(statement)],
	}
}

/// The one value a print statement shows
fn printed(node: &Node) -> Option<Node> {
	match node.drop_meta() {
		Node::List(items, bracket, _) if starts_print(node) => match print_arguments_of(items, bracket).as_slice() {
			[argument] => Some(argument.clone()),
			_ => None,
		},
		_ => None,
	}
}

/// A print last in the statement, in a block or a branch of an `if`
fn prints_last(node: &Node) -> bool {
	match node.drop_meta() {
		Node::List(items, Bracket::Curly, _) => items.last().is_some_and(prints_last),
		Node::Key(_, Op::Then, branch) => prints_last(branch),
		Node::Key(taken, Op::Else, otherwise) => prints_last(taken) || prints_last(otherwise),
		other => printed(other).is_some(),
	}
}

/// The statement assigning the value it ends with to `result`, a print the value it shows: `{result = x; print result}`
fn remembering(node: Node) -> Node {
	if let Some(value) = printed(&node) {
		return Node::List(vec![result_of(value), print_call([symbol(RESULT_WORD)])], Bracket::Curly, Separator::Semicolon);
	}
	match node.drop_meta().clone() {
		Node::List(mut items, Bracket::Curly, separator) if !items.is_empty() => {
			let last = items.pop().expect("not empty");
			items.push(remembering(last));
			Node::List(items, Bracket::Curly, separator)
		}
		Node::Key(condition, Op::Then, branch) => key(*condition, Op::Then, remembering(*branch)),
		Node::Key(taken, Op::Else, otherwise) => key(remembering(*taken), Op::Else, remembering(*otherwise)),
		_ => result_of(node),
	}
}
