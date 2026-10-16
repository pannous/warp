//! Data stays clean (decision error-optional): every item but a literal written in a list literal is marked, and the
//! emitter fails where it holds an Error (wasm_emitter data_item); results, locals and arguments carry an Error on.
//! Only lists as written are marked: the lists later passes build (`guarded_call`'s arguments) hold what they are given

use crate::node::{Bracket, Node};

/// Meta key marking an item written in a list literal
pub const DATA_ITEM: &str = "data item";

/// Every item but a literal written in a `[…]` list, marked
pub fn lower(node: Node) -> Node {
	match crate::node::children_rewritten(node, lower) {
		Node::List(items, Bracket::Square, separator) => {
			Node::List(items.into_iter().map(marked).collect(), Bracket::Square, separator)
		}
		other => other,
	}
}

fn marked(item: Node) -> Node {
	if may_hold_an_error(item.drop_meta()) {
		Node::meta(item, Node::key(DATA_ITEM, Node::True))
	} else {
		item
	}
}

/// Any item but a literal and a nested list (whose items are marked themselves): `e`, `f(x)`, `e+1`, `c ? e : 1`
fn may_hold_an_error(item: &Node) -> bool {
	!matches!(
		item,
		Node::Number(_)
			| Node::Text(_)
			| Node::Char(_)
			| Node::True
			| Node::False
			| Node::Empty
			| Node::List(_, Bracket::Square, _)
	)
}

/// The item under the mark, through any other metadata around it
pub fn marked_item(node: &Node) -> Option<&Node> {
	let Node::Meta { node: inner, data } = node else {
		return None;
	};
	match data.as_ref() {
		Node::Key(key, _, _) if key.name() == DATA_ITEM => Some(inner),
		_ => marked_item(inner),
	}
}
