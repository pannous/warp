//! `m#2#1 = 9` (`m[1][0] = 9`) sets the item inside the inner list (card nested-index): an index assignment stores the
//! changed list back into its target, and `m#2` is no variable to store into. So the inner list is taken out, changed
//! and put back: `nested·0 = m#2; nested·0#1 = 9; m#2 = nested·0`, also for `+=` and deeper chains. An index that is a
//! computation is evaluated once.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const INNER: &str = "nested·";
const INDEX: &str = "nested·index·";

pub fn lower(node: Node) -> Node {
	let mut counter = 0;
	lowered(node, &mut counter)
}

fn lowered(node: Node, counter: &mut usize) -> Node {
	match nested_assignment(&node) {
		Some((base, index, inner_index, op, value)) => {
			let number = *counter;
			*counter += 1;
			let (index_binding, index) = match index.drop_meta() {
				Node::Number(_) | Node::Symbol(_) => (None, index),
				_ => {
					let name = Node::Symbol(format!("{INDEX}{number}"));
					(Some(assign(name.clone(), index)), name)
				}
			};
			let inner_list = Node::Key(Box::new(base), Op::Hash, Box::new(index));
			let inner = Node::Symbol(format!("{INNER}{number}"));
			let changed = Node::Key(Box::new(Node::Key(Box::new(inner.clone()), Op::Hash, Box::new(inner_index))), op, Box::new(lowered(value, counter)));
			let statements = index_binding.into_iter()
				.chain([assign(inner.clone(), inner_list.clone()), lowered(changed, counter), lowered(assign(inner_list, inner), counter)])
				.collect();
			Node::List(statements, Bracket::None, Separator::Semicolon)
		}
		None => node.map_children(|child| lowered(child, counter)),
	}
}

/// `base#index#inner_index <op> value` of an assignment or a compound one (`+=`)
fn nested_assignment(node: &Node) -> Option<(Node, Node, Node, Op, Node)> {
	let Node::Key(target, op, value) = node.drop_meta() else { return None };
	if !(*op == Op::Assign || op.is_compound_assign()) {
		return None;
	}
	let Node::Key(inner, Op::Hash, inner_index) = target.drop_meta() else { return None };
	let Node::Key(base, Op::Hash, index) = inner.drop_meta() else { return None };
	Some((base.as_ref().clone(), index.as_ref().clone(), inner_index.as_ref().clone(), *op, value.as_ref().clone()))
}

fn assign(target: Node, value: Node) -> Node {
	Node::Key(Box::new(target), Op::Assign, Box::new(value))
}
