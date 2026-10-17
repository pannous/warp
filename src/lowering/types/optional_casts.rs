//! `x as T?` keeps ø (card text-generally): ø stays ø, any other value is cast to T, and the result auto-unwraps as an
//! optional does (P179): used as a T it is one, ø there is the loud error of ø as a T. The value is computed once:
//! `f() as text?` becomes `(optional·0 = f(); if optional·0 == ø then ø else optional·0 as text)`.

use super::nodes::{assign, if_then_else, key, Counter};
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;

const OPTIONAL_PREFIX: &str = "optional·";
const OPTIONAL_MARK: char = '?';

pub fn lower(node: Node) -> Node {
	Optionals { counter: Counter::default() }.rewrite(node)
}

struct Optionals {
	counter: Counter,
}

impl Optionals {
	fn rewrite(&self, node: Node) -> Node {
		// `def f(x) as int? := …`, `x as int? = …`: the head declares an optional result, no cast
		if let Node::Key(head, op @ (Op::Define | Op::Assign), body) = node.drop_meta() {
			if matches!(head.drop_meta(), Node::Key(_, Op::As, _)) {
				return key(head.as_ref().clone(), *op, self.rewrite(body.as_ref().clone())).with_meta_of(&node);
			}
		}
		if let Node::Key(value, Op::As, target) = node.drop_meta() {
			if let Node::Symbol(target) = target.drop_meta() {
				if let Some(warp_type) = target.strip_suffix(OPTIONAL_MARK).filter(|name| !name.is_empty()) {
					return self.keeping_empty(self.rewrite(value.as_ref().clone()), warp_type);
				}
			}
		}
		node.map_children(|child| self.rewrite(child))
	}

	fn keeping_empty(&self, value: Node, warp_type: &str) -> Node {
		let cast = |held: Node| {
			let is_empty = key(held.clone(), Op::Eq, Node::Empty);
			if_then_else(is_empty, Node::Empty, key(held, Op::As, symbol(warp_type)))
		};
		if let Node::Symbol(_) = value.drop_meta() {
			return cast(value);
		}
		let held = Node::Symbol(format!("{OPTIONAL_PREFIX}{}", self.counter.next_number()));
		let assignment = assign(held.clone(), value);
		Node::List(vec![assignment, cast(held)], Bracket::Round, Separator::Semicolon)
	}
}
