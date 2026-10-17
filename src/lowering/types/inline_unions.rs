//! Inline unions: `x: int | text` is `x:"int or text"`

use crate::node::Node;
use crate::operators::Op;

/// `x: int | text` is `x:"int or text"` (card inline-union), before any pass reads `int | text` as a value; so is a
/// union of declared classes, `s: Circle | Square` (card union-types)
pub fn lower_inline_unions(node: Node) -> Node {
	let classes = crate::analyzer::declared_classes(&node);
	lower_unions_of(node, &|word: &str| classes.contains(word))
}

fn lower_unions_of(node: Node, is_class: &dyn Fn(&str) -> bool) -> Node {
	match node {
		Node::Key(name, Op::Colon, type_node)
			if let Some(union) = crate::analyzer::union_of_types(&type_node, is_class) =>
		{
			Node::Key(name, Op::Colon, Box::new(Node::Symbol(union)))
		}
		// map_children leaves a class's field declarations as they are: `content: Circle | Square` is one of them
		Node::Type { name, body } => {
			let body = match *body {
				list @ Node::List(..) => list.map_children(|item| lower_unions_of(item, is_class)),
				single => lower_unions_of(single, is_class),
			};
			Node::Type { name, body: Box::new(body) }
		}
		other => other.map_children(|child| lower_unions_of(child, is_class)),
	}
}
