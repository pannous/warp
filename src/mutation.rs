//! `fn!` mutates in place (user decision D2 2026-10-03, "by position", wiki/mutable.md): a `!` after a function or method
//! assigns its result back to the variable it was called on. `x.upper!`, `x.upper()!` and `upper(x)!` become
//! `x = x.upper` in the parser; in `upper x!` the parser only sees `x!`, so it marks x and `lower` turns the call into
//! `x = upper x`. A lone `x!`, `f!` or `{…}!` keeps evaluating.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

/// Meta key on a name written `x!`
const MUTATED_MARK: &str = "mutated";

/// The variable a call written before `!` changes: x in `x.upper`, `x.upper()` and `upper(x)`
pub fn mutated_variable(call: &Node) -> Option<&Node> {
	match call.drop_meta() {
		Node::Key(receiver, Op::Dot, method) if is_name(receiver) && is_method(method) => Some(receiver),
		Node::List(items, Bracket::Round, _) => match items.as_slice() {
			[function, argument] if is_name(function) && is_name(argument) => Some(argument),
			_ => None,
		},
		_ => None,
	}
}

/// `x!`: evaluated alone, assigned the result of the call it is the argument of
pub fn marked(name: Node) -> Node {
	Node::Meta { node: Box::new(name), data: Box::new(Node::key(MUTATED_MARK, Node::True)) }
}

/// `upper x!` → `x = upper x`; every other marked name is just the name
pub fn lower(node: Node) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = items.into_iter().map(lower_inside).collect();
			let call = Node::List(items, bracket, separator);
			match &call {
				Node::List(items, _, separator) if items.len() == 2 && is_name(&items[0]) && is_marked(&items[1])
					&& !matches!(separator, Separator::Semicolon | Separator::Newline) => {
					let variable = unmarked(items[1].clone());
					Node::Key(Box::new(variable), Op::Assign, Box::new(strip_marks(call)))
				}
				_ => strip_marks(call),
			}
		}
		// `x.upper!`: the parser marked the method it was parsing when it met the `!`
		Node::Key(receiver, Op::Dot, method) if is_name(&receiver) && is_marked(&method) => {
			let call = Node::Key(receiver.clone(), Op::Dot, Box::new(unmarked(*method)));
			Node::Key(receiver, Op::Assign, Box::new(call))
		}
		Node::Key(left, op, right) => Node::Key(Box::new(lower(*left)), op, Box::new(lower(*right))),
		Node::Meta { .. } if is_marked(&node) => unmarked(node),
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower(*node)), data },
		other => other,
	}
}

/// Items keep their marks until their own list decides; deeper lists are lowered on their own
fn lower_inside(item: Node) -> Node {
	if is_marked(&item) { item } else { lower(item) }
}

fn strip_marks(node: Node) -> Node {
	match node {
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| if is_marked(&item) { unmarked(item) } else { item }).collect(), bracket, separator),
		other => other,
	}
}

/// The mark may sit under other meta information, such as the position the parser records
fn is_marked(node: &Node) -> bool {
	match node {
		Node::Meta { node, data } => is_mark(data) || is_marked(node),
		_ => false,
	}
}

fn is_mark(data: &Node) -> bool {
	matches!(data, Node::Key(key, _, _) if key.name() == MUTATED_MARK)
}

fn unmarked(node: Node) -> Node {
	match node {
		Node::Meta { node, data } if is_mark(&data) => *node,
		Node::Meta { node, data } => Node::Meta { node: Box::new(unmarked(*node)), data },
		other => other,
	}
}

fn is_name(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(_))
}

fn is_method(method: &Node) -> bool {
	match method.drop_meta() {
		Node::Symbol(_) => true,
		Node::List(items, _, _) => items.first().is_some_and(is_name),
		_ => false,
	}
}
