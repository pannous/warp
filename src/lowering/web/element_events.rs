//! Handlers on elements (card web-element, notes/web_framework.md step 2): `button{ on click { count += 1 } "Add" }`
//! becomes the main-level handler `on click·1 { count += 1 }` (a page event of its own, event_signals.rs) and the element
//! the attribute `data-warp-click: "1"`. The page calls handler 1 on a click inside that element and shows the
//! program's markup anew (playground.js, worker.js showHandled).
//! `input{ bind: name }` (card web-bind) is `input{ value: name on input { name = event.value } }`; a checkbox or radio
//! binds `checked` to `event.checked`. Binds are spelled out first (lower_binds), so a component sees the handler of a
//! bind among its state's handlers (component_state.rs, card web-bind-component).
//! A bound variable's declared type gives the field its checks (card web-validate): `age: int = 30` makes
//! `input{ bind: age }` a number field of step 1, which the browser refuses 2.5 in, `float` one of any step, `bool` a
//! checkbox; an attribute the element sets itself wins.

use super::nodes::{any_part, block, key};
use crate::event_signals::{is_element_event, ELEMENT_EVENT_JOINER};
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::ON_WORD;
use std::collections::HashMap;

const BIND_WORD: &str = "bind";
const INPUT_EVENT: &str = "input";
const TYPE_ATTRIBUTE: &str = "type";
/// input types whose binding is `checked`, not `value`
const CHECKED_TYPES: [&str; 2] = ["checkbox", "radio"];
const VALUE: &str = "value";
const CHECKED: &str = "checked";
const STEP_ATTRIBUTE: &str = "step";
const NUMBER_FIELD: &str = "number";
const CHECKBOX_FIELD: &str = "checkbox";
pub(crate) const ANY_STEP: &str = "any";
const WHOLE_STEP: &str = "1";
/// `data-warp-click`: the attribute naming an element's handler of an event
pub const HANDLER_ATTRIBUTE_PREFIX: &str = "data-warp-";

pub fn lower(program: Node) -> Node {
	let Node::List(statements, bracket, separator) = program else {
		return program;
	};
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
	any_part(node, |part| {
		element_items(part).is_some_and(|items| (0..items.len()).any(|index| handler_at(items, index).is_some()))
	})
}

/// Each `bind: name` of an element spelled out as its attribute and handler, with the checks of name's declared type
pub fn lower_binds(program: Node) -> Node {
	let declared = declared_variable_types(&program);
	with_binds(program, &declared)
}

fn with_binds(node: Node, declared: &HashMap<String, Option<String>>) -> Node {
	let node = node.map_children(|child| with_binds(child, declared));
	with_element_items(node, |items| {
		let items: Vec<Node> = items.into_iter().flat_map(line_pairs).collect();
		let checks: Vec<Node> = (0..items.len())
			.filter_map(|index| bound_at(&items, index))
			.flat_map(|variable| type_checks(&variable, declared, &items))
			.collect();
		let property = bound_property(&[checks.clone(), items.clone()].concat());
		let mut checks = Some(checks);
		items
			.iter()
			.enumerate()
			.flat_map(|(index, item)| match bound_at(&items, index) {
				Some(variable) => {
					checks.take().unwrap_or_default().into_iter().chain(bound(variable, property, declared)).collect()
				}
				None => vec![item.clone()],
			})
			.collect()
	})
}

/// `bind: fruit aria-label: "f"` on a line before more items: its pairs as items of their own, as markup renders them
fn line_pairs(item: Node) -> Vec<Node> {
	match item.drop_meta() {
		Node::List(pairs, Bracket::None, _)
			if pairs.iter().all(|pair| matches!(pair.drop_meta(), Node::Key(_, Op::Colon, _))) =>
		{
			pairs.clone()
		}
		_ => vec![item],
	}
}

/// `age: int = 30`: each variable's declared type; none for a name declared with two types
fn declared_variable_types(program: &Node) -> HashMap<String, Option<String>> {
	let mut declared: HashMap<String, Option<String>> = HashMap::new();
	program.visit(&mut |part| {
		let Node::Key(target, Op::Assign, _) = part.drop_meta() else {
			return;
		};
		let Node::Key(name, Op::Colon, kind) = target.drop_meta() else {
			return;
		};
		let (Node::Symbol(name), Node::Symbol(kind)) = (name.drop_meta(), kind.drop_meta()) else {
			return;
		};
		declared
			.entry(name.clone())
			.and_modify(|known| {
				if known.as_ref() != Some(kind) {
					*known = None
				}
			})
			.or_insert_with(|| Some(kind.clone()));
	});
	declared
}

/// The attributes a bound variable's declared type checks its field with, but those the element sets itself
fn type_checks(variable: &Node, declared: &HashMap<String, Option<String>>, items: &[Node]) -> Vec<Node> {
	let Some(Some(kind)) = declared.get(&variable.drop_meta().name()) else {
		return vec![];
	};
	let checks: &[(&str, Node)] = if crate::analyzer::is_bool_type(kind) {
		&[(TYPE_ATTRIBUTE, Node::Text(CHECKBOX_FIELD.to_string()))]
	} else if crate::analyzer::is_whole_type(kind) {
		&[(TYPE_ATTRIBUTE, Node::Text(NUMBER_FIELD.to_string())), (STEP_ATTRIBUTE, Node::Text(WHOLE_STEP.to_string()))]
	} else if crate::analyzer::builtin_type_kind(kind) == Some(crate::type_kinds::Kind::Float) {
		&[(TYPE_ATTRIBUTE, Node::Text(NUMBER_FIELD.to_string())), (STEP_ATTRIBUTE, Node::Text(ANY_STEP.to_string()))]
	} else {
		&[]
	};
	let sets = |name: &str| {
		items
			.iter()
			.any(|item| matches!(item.drop_meta(), Node::Key(set, Op::Colon, _) if set.drop_meta().name() == name))
	};
	checks
		.iter()
		.filter(|(name, _)| !sets(name))
		.map(|(name, value)| key(symbol(name), Op::Colon, value.clone()))
		.collect()
}

/// `bind: name` at `index` of an element's items: the bound variable
fn bound_at(items: &[Node], index: usize) -> Option<Node> {
	match items.get(index)?.drop_meta() {
		Node::Key(word, Op::Colon, variable) if word.is_symbol(BIND_WORD) => Some(variable.as_ref().clone()),
		_ => None,
	}
}

/// `value`, or `checked` for a checkbox or radio (`type: "checkbox"`)
fn bound_property(items: &[Node]) -> &'static str {
	let checks = items.iter().any(|item| {
		matches!(item.drop_meta(), Node::Key(name, Op::Colon, kind)
		if name.drop_meta().name() == TYPE_ATTRIBUTE && CHECKED_TYPES.contains(&kind.drop_meta().name().as_str()))
	});
	if checks {
		CHECKED
	} else {
		VALUE
	}
}

/// `bind: name` as the attribute `value: name` and the handler `on input { name = event.value }`; a whole-number
/// variable takes only whole numbers (card guest-type): `if event.value is int then name = event.value`
fn bound(variable: Node, property: &str, declared: &HashMap<String, Option<String>>) -> [Node; 4] {
	let attribute = key(symbol(property), Op::Colon, variable.clone());
	let read = key(symbol(crate::event_signals::EVENT_WORD), Op::Dot, symbol(property));
	let assign = key(variable.clone(), Op::Assign, read);
	let assign = match declared.get(&variable.drop_meta().name()) {
		Some(Some(kind)) if crate::analyzer::is_whole_type(kind) => {
			let check = crate::warp_parser::parse_generated(&format!(
				"{} is {kind}",
				crate::event_signals::EVENT_WORD.to_string() + "." + property
			));
			super::nodes::if_then(check, assign)
		}
		_ => assign,
	};
	let update = block(vec![assign]);
	[attribute, symbol(ON_WORD), symbol(INPUT_EVENT), update]
}

/// The items of an element's block: `button{ … }` (a block of anything else is code, its `on click` page-wide)
pub(crate) fn element_items(node: &Node) -> Option<&Vec<Node>> {
	let Node::Key(tag, Op::Colon | Op::None, content) = node else {
		return None;
	};
	let tag = tag.symbol_name()?;
	match content.drop_meta() {
		Node::List(items, Bracket::Curly, _) if crate::web::markup::is_element_tag(tag) => Some(items),
		_ => None,
	}
}

/// An element with its items rewritten; any other node as it is
pub(crate) fn with_element_items(node: Node, rewrite: impl FnOnce(Vec<Node>) -> Vec<Node>) -> Node {
	if element_items(&node).is_none() {
		return node;
	}
	let Node::Key(tag, op, content) = node else { unreachable!("an element") };
	let Node::List(items, bracket, separator) = content.drop_meta().clone() else { unreachable!("an element's block") };
	Node::Key(tag, op, Box::new(Node::List(rewrite(items), bracket, separator)))
}

/// `on click {…}` at `index` of a tag's items, as a block reads it (`on`, then `click: {…}`) or as a statement does
pub(crate) fn handler_at(items: &[Node], index: usize) -> Option<(String, Node, usize)> {
	// `form{⏎ on submit { … }⏎ button{…} }`: a handler on a line of its own is that line's group
	if let Node::List(line, Bracket::None, _) = items.get(index)?.drop_meta() {
		return handler_at(line, 0)
			.filter(|(_, _, length)| *length == line.len())
			.map(|(event, body, _)| (event, body, 1));
	}
	if !items.get(index)?.is_symbol(ON_WORD) {
		return None;
	}
	let event_name = |node: &Node| match node.drop_meta() {
		Node::Symbol(event) if is_element_event(event) => Some(event.clone()),
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
		with_element_items(node, |items| self.marked_items(items))
	}

	/// An element's items with its handlers taken out and the element marked with their numbers
	fn marked_items(&mut self, items: Vec<Node>) -> Vec<Node> {
		let mut kept = vec![];
		let mut index = 0;
		while index < items.len() {
			match handler_at(&items, index) {
				Some((event, body, length)) => {
					self.numbered += 1;
					kept.push(key(
						Node::Symbol(format!("{HANDLER_ATTRIBUTE_PREFIX}{event}")),
						Op::Colon,
						Node::Text(self.numbered.to_string()),
					));
					let handled = Node::Symbol(format!("{event}{ELEMENT_EVENT_JOINER}{}", self.numbered));
					self.handlers.push(Node::List(
						vec![symbol(ON_WORD), handled, body],
						Bracket::None,
						Separator::Space,
					));
					index += length;
				}
				None => {
					kept.push(self.marked(items[index].clone()));
					index += 1;
				}
			}
		}
		kept
	}
}
