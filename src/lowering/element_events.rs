//! Handlers on elements (card web-element, notes/web_framework.md step 2): `button{ on click { count += 1 } "Add" }`
//! becomes the main-level handler `on click·1 { count += 1 }` (a page event of its own, event_signals.rs) and the element
//! the attribute `data-wasp-click: "1"`. The page calls handler 1 on a click inside that element and shows the
//! program's markup anew (playground.js, worker.js showHandled).

use crate::event_signals::{ELEMENT_EVENT_JOINER, PAGE_EVENTS};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const ON_WORD: &str = "on";
/// `data-wasp-click`: the attribute naming an element's handler of an event
pub const HANDLER_ATTRIBUTE_PREFIX: &str = "data-wasp-";

pub fn lower(program: Node) -> Node {
	let Node::List(statements, bracket, separator) = program else { return program };
	if !statements.iter().any(has_element_handler) {
		return Node::List(statements, bracket, separator);
	}
	let mut marker = Marker { handlers: vec![], numbered: 0 };
	let mut lowered = vec![];
	for statement in statements {
		let marked = marker.marked(statement);
		// the handlers come before the statement whose markup names them: the last line stays the program's value
		lowered.append(&mut marker.handlers);
		lowered.push(marked);
	}
	Node::List(lowered, bracket, separator)
}

pub(crate) fn has_element_handler(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= element_items(part).is_some_and(|items| (0..items.len()).any(|index| handler_at(items, index).is_some())));
	found
}

/// The items of an element's block: `button{ … }` (a block of anything else is code, its `on click` page-wide)
pub(crate) fn element_items(node: &Node) -> Option<&Vec<Node>> {
	let Node::Key(tag, Op::Colon | Op::None, content) = node else { return None };
	let Node::Symbol(tag) = tag.drop_meta() else { return None };
	match content.drop_meta() {
		Node::List(items, Bracket::Curly, _) if crate::html::is_element_tag(tag) => Some(items),
		_ => None,
	}
}

/// `on click {…}` at `index` of a tag's items, as a block reads it (`on`, then `click: {…}`) or as a statement does
pub(crate) fn handler_at(items: &[Node], index: usize) -> Option<(String, Node, usize)> {
	if !matches!(items.get(index)?.drop_meta(), Node::Symbol(word) if word == ON_WORD) {
		return None;
	}
	let event_name = |node: &Node| match node.drop_meta() {
		Node::Symbol(event) if PAGE_EVENTS.contains(&event.as_str()) => Some(event.clone()),
		_ => None,
	};
	match items.get(index + 1)?.drop_meta() {
		Node::Key(event, Op::Colon, body) => Some((event_name(event)?, body.as_ref().clone(), 2)),
		event => Some((event_name(event)?, items.get(index + 2)?.clone(), 3)),
	}
}

/// The handlers taken out of the elements so far, and how many there were in the program
struct Marker {
	handlers: Vec<Node>,
	numbered: usize,
}

impl Marker {
	/// The node with each element handler taken out into `handlers` and its element marked with the handler's number
	fn marked(&mut self, node: Node) -> Node {
		if element_items(&node).is_none() {
			return node.map_children(|child| self.marked(child));
		}
		let Node::Key(tag, op, content) = node else { unreachable!("an element") };
		let Node::List(items, bracket, separator) = content.drop_meta().clone() else { unreachable!("an element's block") };
		Node::Key(tag, op, Box::new(self.marked_items(items, bracket, separator)))
	}

	/// An element's items with its handlers taken out and the element marked with their numbers
	fn marked_items(&mut self, items: Vec<Node>, bracket: Bracket, separator: Separator) -> Node {
		let mut kept = vec![];
		let mut index = 0;
		while index < items.len() {
			match handler_at(&items, index) {
				Some((event, body, length)) => {
					self.numbered += 1;
					kept.push(Node::Key(Box::new(Node::Symbol(format!("{HANDLER_ATTRIBUTE_PREFIX}{event}"))), Op::Colon, Box::new(Node::Text(self.numbered.to_string()))));
					let handled = Node::Symbol(format!("{event}{ELEMENT_EVENT_JOINER}{}", self.numbered));
					self.handlers.push(Node::List(vec![Node::Symbol(ON_WORD.to_string()), handled, body], Bracket::None, Separator::Space));
					index += length;
				}
				None => {
					kept.push(self.marked(items[index].clone()));
					index += 1;
				}
			}
		}
		Node::List(kept, bracket, separator)
	}
}
