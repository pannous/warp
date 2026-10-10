//! `x as T?` keeps ø (card text-generally): ø stays ø, any other value is cast to T, and the result auto-unwraps as an
//! optional does (P179): used as a T it is one, ø there is the loud error of ø as a T. The value is computed once:
//! `f() as text?` becomes `(optional·0 = f(); if optional·0 == ø then ø else optional·0 as text)`.

use super::nodes::key;
use crate::lowering::variable_signals::if_then_else;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::cell::Cell;

const OPTIONAL_PREFIX: &str = "optional·";
const OPTIONAL_MARK: char = '?';

pub fn lower(node: Node) -> Node {
	Optionals { counter: Cell::new(0) }.rewrite(node)
}

struct Optionals {
	counter: Cell<usize>,
}

impl Optionals {
	fn rewrite(&self, node: Node) -> Node {
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
			if_then_else(is_empty, Node::Empty, key(held, Op::As, Node::Symbol(warp_type.to_string())))
		};
		if let Node::Symbol(_) = value.drop_meta() {
			return cast(value);
		}
		let held = Node::Symbol(format!("{OPTIONAL_PREFIX}{}", self.counter.replace(self.counter.get() + 1)));
		let assignment = key(held.clone(), Op::Assign, value);
		Node::List(vec![assignment, cast(held)], Bracket::Round, Separator::Semicolon)
	}
}
