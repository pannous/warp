//! A number subscript on a variable that starts as the empty map `{}` keys it, like a text subscript (user decision
//! P34): `d={}; d[1]="a"` is {1:"a"} and `d[1]` reads that entry. On any other value `d[1]` stays a position.

use crate::node::Node;
use crate::operators::Op;
use crate::warp_parser::WrittenIndex;
use std::collections::HashSet;

pub fn lower(node: Node) -> Node {
	let maps = empty_map_variables(&node);
	if maps.is_empty() {
		return node;
	}
	keyed(node, &maps)
}

/// The variables assigned the empty map `{}`
fn empty_map_variables(node: &Node) -> HashSet<String> {
	let mut maps = HashSet::new();
	node.visit(&mut |part| if let Node::Key(target, Op::Assign, value) = part {
		if let (Node::Symbol(name), Node::List(items, crate::node::Bracket::Curly, _)) = (target.drop_meta(), value.drop_meta()) {
			if items.is_empty() {
				maps.insert(name.clone());
			}
		}
	});
	maps
}

fn keyed(node: Node, maps: &HashSet<String>) -> Node {
	match node {
		Node::Key(target, Op::Hash, index) if matches!(target.drop_meta(), Node::Symbol(name) if maps.contains(name)) => {
			match written_index(&index) {
				Some(number) => {
					let key = Node::Key(Box::new(Node::Text(number.to_string())), Op::Add, Box::new(crate::node::int(1)));
					Node::Key(target, Op::Hash, Box::new(key))
				}
				None => Node::Key(target, Op::Hash, Box::new(keyed(*index, maps))),
			}
		}
		Node::Key(left, op, right) => Node::Key(Box::new(keyed(*left, maps)), op, Box::new(keyed(*right, maps))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| keyed(item, maps)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(keyed(*node, maps)), data },
		other => other,
	}
}

fn written_index(index: &Node) -> Option<crate::extensions::numbers::Number> {
	match index {
		Node::Meta { data, node } => match data.as_ref() {
			Node::Data(dada) => dada.downcast_ref::<WrittenIndex>().map(|WrittenIndex(number)| *number).or_else(|| written_index(node)),
			_ => written_index(node),
		},
		_ => None,
	}
}
