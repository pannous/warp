//! An object's fields may be separated by commas on one line and by newlines or semicolons between lines (JSON5 style,
//! "commas optional", samples/kitchensink.wasp): the parser groups `{ a: 1, b: 2; c: 3 }` as `{ (a: 1, b: 2); c: 3 }`,
//! and `.c` found no field; `{ "a": 1⏎ b: 2 }` (a quoted name, rows on lines) neither. A braced list whose items are all
//! fields, alone or in such groups, is one comma-separated row of them.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

pub fn lower(node: Node) -> Node {
	let node = node.map_children(lower);
	let Node::List(items, Bracket::Curly, separator) = node.drop_meta() else { return node };
	let regrouped = *separator != Separator::Colon || items.iter().any(is_field_group);
	if items.is_empty() || !regrouped || !items.iter().all(|item| is_field(item) || is_field_group(item)) {
		return node;
	}
	let flat = Node::List(items.iter().flat_map(fields).collect(), Bracket::Curly, Separator::Colon);
	match node {
		Node::Meta { data, .. } => Node::Meta { node: Box::new(flat), data },
		_ => flat,
	}
}

fn is_field(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(_, Op::Colon, _))
}

/// `a: 1, b: 2` within the braces, a `;` group of such rows too
fn is_field_group(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, Bracket::None, separator) if *separator != Separator::Space && items.iter().all(|item| is_field(item) || is_field_group(item)))
}

fn fields(node: &Node) -> Vec<Node> {
	match node.drop_meta() {
		Node::List(items, Bracket::None, _) if is_field_group(node) => items.iter().flat_map(fields).collect(),
		_ => vec![node.clone()],
	}
}
