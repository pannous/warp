//! An element of a field changes like an element of a variable, by value: `k.counts#i += 1` is
//! `k·counts·elements = k.counts; k·counts·elements#i += 1; k.counts = k·counts·elements`, the changed list written back
//! to its field. In a method the field is read from self (class_methods), so `counts#i += 1` changes the object too.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

/// The variable holding a field's list while one element changes: `k·counts·elements`
const ELEMENTS_SUFFIX: &str = "·elements";

pub fn lower(node: Node) -> Node {
	match node {
		Node::Key(target, op, value) if changes_element(&op) => match field_element(&target) {
			Some((field, index, name)) => written_back(field, index, op, lower(*value), name),
			None => Node::Key(target, op, Box::new(lower(*value))),
		},
		other => other.map_children(lower),
	}
}

fn changes_element(op: &Op) -> bool {
	*op == Op::Assign || op.is_compound_assign() || matches!(op, Op::Inc | Op::Dec)
}

/// `k.counts#i`: the field `k.counts`, the index i and the name of the variable for its list
fn field_element(target: &Node) -> Option<(Node, Node, String)> {
	let Node::Key(field, Op::Hash, index) = target.drop_meta() else { return None };
	let Node::Key(object, Op::Dot, member) = field.drop_meta() else { return None };
	let (Node::Symbol(object), Node::Symbol(member)) = (object.drop_meta(), member.drop_meta()) else { return None };
	Some((field.as_ref().clone(), index.as_ref().clone(), format!("{object}·{member}{ELEMENTS_SUFFIX}")))
}

fn written_back(field: Node, index: Node, op: Op, value: Node, name: String) -> Node {
	let elements = Node::Symbol(name);
	let element = Node::Key(Box::new(elements.clone()), Op::Hash, Box::new(index));
	let statements = vec![
		Node::Key(Box::new(elements.clone()), Op::Assign, Box::new(field.clone())),
		Node::Key(Box::new(element), op, Box::new(value)),
		Node::Key(Box::new(field), Op::Assign, Box::new(elements)),
	];
	Node::List(statements, Bracket::None, Separator::Semicolon)
}
