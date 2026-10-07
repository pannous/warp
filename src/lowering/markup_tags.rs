//! wiki/mark.md: inside a tag's block `div(class:"form-group")` is the tag `div{class:"form-group"}`, and
//! `label(for:pwd):"Password"` the tag `label{for:pwd "Password"}` (samples/html.wasp, card g-_alg). Only data takes
//! this reading: a name the program defines stays its call, and outside a tag block a call of an unbound name stays
//! the loud error (P92). A comprehension or method call among an element's children gives children (card web-keyed).

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashSet;

const FOR_WORD: &str = "for";
const IN_WORD: &str = "in";

pub fn lower(node: Node) -> Node {
	// `label(for:pwd):"Password"` itself would read as a definition: only `:=`, function keywords and assignments define
	let mut defined = crate::welcome_forms::defined_names(&node);
	crate::library_words::collect_assigned_names(&node, &mut defined);
	outside_tags(node, &defined)
}

fn outside_tags(node: Node, defined: &HashSet<String>) -> Node {
	let node = spaced_element(node, defined);
	match tag(&node, defined) {
		Some(_) => tag_with_attributes(node, defined),
		None => node.map_children(|child| outside_tags(child, defined)),
	}
}

/// `ul { … }` with a blank, as a statement: the element `ul{ … }` when the program does not define the name, so its
/// children read as the glued tag's do (`ul { for item in items { li: item } }`, samples/html_dsl.wasp)
fn spaced_element(node: Node, defined: &HashSet<String>) -> Node {
	if let Node::List(items, Bracket::None, Separator::Space) = node.drop_meta() {
		if let [name, body] = items.as_slice() {
			let is_element = matches!(name.drop_meta(), Node::Symbol(tag) if crate::html::is_element_tag(tag) && !defined.contains(tag));
			if is_element && matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) {
				return Node::Key(Box::new(name.drop_meta().clone()), Op::Colon, Box::new(body.drop_meta().clone()));
			}
		}
	}
	node
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
				let is_element = matches!(name.drop_meta(), Node::Symbol(tag) if crate::html::is_element_tag(tag));
				let items = items.into_iter().map(|item| tag_with_attributes(attributed_tag(item, defined), defined))
					.map(|item| if is_element { loop_as_comprehension(item) } else { item })
					.map(|item| if is_element && is_computed_children(&item) { Node::List(vec![item], Bracket::Square, Separator::None) } else { item })
					.collect();
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

/// `[li{t} for t in ts]` or `ts.map(…)` among an element's children: its items are children, so it stays a list
/// (`[…]` around it) when it is lowered to statements, which in a block would run instead of being an item
fn is_computed_children(item: &Node) -> bool {
	match item.drop_meta() {
		Node::List(items, Bracket::Square, _) => items.iter().any(|part| matches!(part.drop_meta(), Node::List(words, _, _) if matches!(words.first().map(Node::drop_meta), Some(Node::Symbol(word)) if word == FOR_WORD))),
		Node::Key(_, Op::Dot, _) => true,
		_ => false,
	}
}

/// `ul{ for t in todos { li{t} } }`: a loop among an element's children gives one child per item, as the comprehension
/// `[li{t} for t in todos]` (`[element, (for t in todos ø)]` as the parser groups it); a body of several items gives
/// the value of its last (card markup-ul)
fn loop_as_comprehension(item: Node) -> Node {
	let Node::List(words, Bracket::None, _) = item.drop_meta() else { return item };
	let [for_word, variable, in_word, sequence, body] = words.as_slice() else { return item };
	let is_word = |node: &Node, word: &str| matches!(node.drop_meta(), Node::Symbol(symbol) if symbol == word);
	if !is_word(for_word, FOR_WORD) || !is_word(in_word, IN_WORD) {
		return item;
	}
	let element = match body.drop_meta() {
		Node::List(items, Bracket::Curly, _) if items.len() == 1 => items[0].clone(),
		Node::List(items, Bracket::Curly, separator) => Node::List(items.clone(), Bracket::Round, separator.clone()),
		other => other.clone(),
	};
	let header = Node::List(vec![for_word.clone(), variable.clone(), in_word.clone(), sequence.clone(), Node::Empty], Bracket::None, Separator::Space);
	Node::List(vec![element, header], Bracket::Square, Separator::Space)
}

fn tag_node(name: String, items: Vec<Node>) -> Node {
	Node::Key(Box::new(Node::Symbol(name)), Op::Colon, Box::new(Node::List(items, Bracket::Curly, Separator::Space)))
}
