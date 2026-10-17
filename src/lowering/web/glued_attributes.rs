//! An attribute's value ends before a text, a list or an element written after it on its line: those are the element's
//! content (card words-glue): `label{ for: name "Name" }` has for="name" and the text Name, not for="name Name";
//! `select{ bind: fruit ["apple" "pear"] }` binds fruit and lists the options.

use super::element_events::{element_items, with_element_items};
use super::nodes::key;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use crate::words::STYLE_WORD;

pub fn lower(program: Node) -> Node {
	let program = program.map_children(lower);
	let is_style = matches!(&program, Node::Key(tag, _, _) if tag.is_symbol(STYLE_WORD));
	if is_style || element_items(&program).is_none() {
		return program;
	}
	with_element_items(program, |items| items.into_iter().flat_map(unglued).collect())
}

/// `for: name "Name"` as the attribute `for: name` and the content "Name"; any other item as it is
fn unglued(item: Node) -> Vec<Node> {
	if let Node::Key(name, Op::Colon, value) = item.drop_meta() {
		if let Node::List(parts, Bracket::None, _) = value.drop_meta() {
			if let Some(content_start) = parts.iter().skip(1).position(is_content).map(|index| index + 1) {
				let value = match &parts[..content_start] {
					[single] => single.clone(),
					words => Node::List(words.to_vec(), Bracket::None, crate::node::Separator::Space),
				};
				let attribute = key(name.as_ref().clone(), Op::Colon, value).with_meta_of(&item);
				return [vec![attribute], parts[content_start..].to_vec()].concat();
			}
		}
	}
	vec![item]
}

/// A text (`"L"` is a Char), a list or an element: what an element holds, no part of an attribute's value
fn is_content(part: &Node) -> bool {
	match part.drop_meta() {
		Node::Text(_) | Node::Char(_) | Node::List(_, Bracket::Square, _) => true,
		element => element_items(element).is_some(),
	}
}
