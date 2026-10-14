//! `xs.insert 4` is `xs.insert(4)` (card xs-insert): a phrase whose head is a member is the call of that member, as
//! `f x` is `f(x)`. It runs after the markup passes, which read `p.note "hi"` as an element with a class

use crate::node::{parens, Bracket, Node, Separator};
use crate::operators::Op;

pub fn lower(node: Node) -> Node {
	let node = node.map_children(lower);
	let Node::List(items, Bracket::None, Separator::Space) = node.drop_meta() else {
		return node;
	};
	let [head, arguments @ ..] = items.as_slice() else {
		return node;
	};
	let Node::Key(receiver, Op::Dot, member) = head.drop_meta() else {
		return node;
	};
	if arguments.is_empty() || member.drop_meta().symbol_name().is_none() {
		return node;
	}
	let call = parens([vec![(**member).clone()], arguments.to_vec()].concat());
	Node::Key(receiver.clone(), Op::Dot, Box::new(call))
}
