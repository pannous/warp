//! wiki/mark.md: inside a tag's block `div(class:"form-group")` is the tag `div{class:"form-group"}`, and
//! `label(for:pwd):"Password"` the tag `label{for:pwd "Password"}` (samples/html.wasp, card g-_alg). Only data takes
//! this reading: a name the program defines stays its call, and outside a tag block a call of an unbound name stays
//! the loud error (P92).

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashSet;

pub fn lower(node: Node) -> Node {
	// `label(for:pwd):"Password"` itself would read as a definition: only `:=`, function keywords and assignments define
	let mut defined = crate::welcome_forms::defined_names(&node);
	crate::library_words::collect_assigned_names(&node, &mut defined);
	outside_tags(node, &defined)
}

fn outside_tags(node: Node, defined: &HashSet<String>) -> Node {
	match tag(&node, defined) {
		Some(_) => tag_with_attributes(node, defined),
		None => node.map_children(|child| outside_tags(child, defined)),
	}
}

/// `name{…}` of a name the program does not define: its body items
fn tag<'a>(node: &'a Node, defined: &HashSet<String>) -> Option<&'a [Node]> {
	let Node::Key(name, Op::Colon, body) = node.drop_meta() else { return None };
	let Node::Symbol(name) = name.drop_meta() else { return None };
	match body.drop_meta() {
		Node::List(items, Bracket::Curly, _) if !defined.contains(name) => Some(items),
		_ => None,
	}
}

/// A tag whose items, and their items in turn, read `name(key:value…)` as the tag `name{key:value…}`
fn tag_with_attributes(node: Node, defined: &HashSet<String>) -> Node {
	match node {
		Node::Meta { node, data } => Node::Meta { node: Box::new(tag_with_attributes(*node, defined)), data },
		Node::Key(name, Op::Colon, body) => match *body {
			Node::List(items, Bracket::Curly, separator) => {
				let items = items.into_iter().map(|item| tag_with_attributes(attributed_tag(item, defined), defined)).collect();
				Node::Key(name, Op::Colon, Box::new(Node::List(items, Bracket::Curly, separator)))
			}
			body => Node::Key(name, Op::Colon, Box::new(body)),
		},
		other => other,
	}
}

/// `label(for:pwd)` → `label{for:pwd}`, `label(for:pwd):"Password"` → `label{for:pwd "Password"}`; anything else as is
fn attributed_tag(item: Node, defined: &HashSet<String>) -> Node {
	if let Some((name, attributes)) = attributes(&item, defined) {
		return tag_node(name, attributes);
	}
	if let Node::Key(head, Op::Colon, content) = item.drop_meta() {
		if let Some((name, mut attributes)) = attributes(head, defined) {
			attributes.push(content.as_ref().clone());
			return tag_node(name, attributes);
		}
	}
	item
}

/// `name(key:value…)` of a name the program does not define: the name and the key-value pairs
fn attributes(node: &Node, defined: &HashSet<String>) -> Option<(String, Vec<Node>)> {
	let Node::List(items, _, _) = node.drop_meta() else { return None };
	let (name, pairs) = items.split_first()?;
	let Node::Symbol(name) = name.drop_meta() else { return None };
	let all_pairs = !pairs.is_empty() && pairs.iter().all(|pair| matches!(pair.drop_meta(), Node::Key(_, Op::Colon, _)));
	(all_pairs && !defined.contains(name)).then(|| (name.clone(), pairs.to_vec()))
}

fn tag_node(name: String, items: Vec<Node>) -> Node {
	Node::Key(Box::new(Node::Symbol(name)), Op::Colon, Box::new(Node::List(items, Bracket::Curly, Separator::Space)))
}
