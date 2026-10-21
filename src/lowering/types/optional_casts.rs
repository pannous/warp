//! `x as T?` keeps ø (card text-generally): ø stays ø, any other value is cast to T, and the result auto-unwraps as an
//! optional does (P179): used as a T it is one, ø there is the loud error of ø as a T. The value is computed once:
//! `f() as text?` becomes `(optional·0 = f(); if optional·0 == ø then ø else optional·0 as text)`. A cast to a list type
//! casts each item: `["1", "2"] as ints` is [1, 2].

use super::nodes::{assign, if_then_else, key, Counter};
use crate::lowering::words::library_words::BYTE_LIST;
use crate::node::{call, symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::{LIST_WORD, MAP_WORD};

const OPTIONAL_PREFIX: &str = "optional·";
const OPTIONAL_MARK: char = '?';
/// The item of a list cast item by item
const CAST_ITEM: &str = "cast·item";

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
			// `text as bytes` is the text's UTF-8 bytes, no cast of each item
			let element = crate::analyzer::declared_element_type(target).filter(|_| !target.is_symbol(BYTE_LIST));
			if let Some(element) = element {
				return items_cast(self.rewrite(value.as_ref().clone()), element);
			}
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

/// `xs as texts`, `xs as [text]`: the list with each item cast to the element type, `(xs as list).map(item => item as text)`
fn items_cast(value: Node, element: &str) -> Node {
	let item = symbol(CAST_ITEM);
	let lambda = key(item.clone(), Op::Arrow, key(item, Op::As, symbol(element)));
	key(key(value, Op::As, symbol(LIST_WORD)), Op::Dot, call(MAP_WORD, vec![lambda]))
}
