//! `(p).a` is `p.a` (card dump-text): parentheses around a variable before a dot only group it, so the dot reads the
//! field or method as it does without them. Before, `(p).a` stayed the data `a:1.a` and `(t).dump` of a text failed
//! at run time as an item print could not join. `(function f).dump` keeps its group: the function itself, not a call

use crate::node::{Bracket, Node};
use crate::operators::Op;

pub fn lower(node: Node) -> Node {
	match node {
		Node::Key(receiver, Op::Dot, member) => {
			Node::Key(Box::new(ungrouped(lower(*receiver))), Op::Dot, Box::new(lower(*member)))
		}
		other => other.map_children(lower),
	}
}

fn ungrouped(receiver: Node) -> Node {
	match receiver.drop_meta() {
		Node::List(items, Bracket::Round, _)
			if items.len() == 1
				&& matches!(items[0].drop_meta(), Node::Symbol(_))
				&& crate::closures::referenced_function(&items[0]).is_none() =>
		{
			items[0].clone()
		}
		_ => receiver,
	}
}
