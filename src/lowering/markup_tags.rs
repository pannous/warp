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
	outside_tags(spaced_elements(node, &defined), &defined)
}

/// Only whole statements of the main level: deeper `p { … }` is `match p { … }`, a parameter p and its block
fn spaced_elements(program: Node, defined: &HashSet<String>) -> Node {
	match program {
		Node::List(statements, bracket, separator) if crate::variable_signals::is_statement_list(&bracket, &separator) =>
			Node::List(statements.into_iter().map(|statement| spaced_element(statement, defined)).collect(), bracket, separator),
		single => spaced_element(single, defined),
	}
}

fn outside_tags(node: Node, defined: &HashSet<String>) -> Node {
	match tag(&node, defined) {
		Some(_) => tag_with_attributes(node, defined),
		None => node.map_children(|child| outside_tags(child, defined)),
	}
}

/// `ul { … }` with a blank, as a statement: the element `ul{ … }` when the program does not define the name, so its
/// children read as the glued tag's do (`ul { for item in items { li: item } }`, samples/html_dsl.wasp);
/// glued `html(lang: "en"){ … }` (a round list) the element `html{ lang: "en" … }`, its attributes before its children
/// (card markup-attribute)
fn spaced_element(node: Node, defined: &HashSet<String>) -> Node {
	if let Node::List(items, Bracket::None | Bracket::Round, _) = node.drop_meta() {
		if let [head, body] = items.as_slice() {
			if let (Some((tag, mut content)), Node::List(children, Bracket::Curly, separator)) = (element_head(head, defined), body.drop_meta()) {
				content.extend(children.iter().cloned());
				return Node::Key(Box::new(Node::Symbol(tag)), Op::Colon, Box::new(Node::List(content, Bracket::Curly, separator.clone())));
			}
		}
	}
	node
}

/// `ul` or `html(lang: "en")` naming an element the program does not define: its tag and attributes
fn element_head(head: &Node, defined: &HashSet<String>) -> Option<(String, Vec<Node>)> {
	let parts = match head.drop_meta() {
		Node::Symbol(tag) => Some((tag.clone(), vec![])),
		_ => attributes(head, defined),
	};
	parts.filter(|(tag, _)| crate::markup::is_element_tag(tag) && !defined.contains(tag))
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
				let is_element = matches!(name.drop_meta(), Node::Symbol(tag) if crate::markup::is_element_tag(tag));
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

/// `name(key:value…)` of a name the program does not define: the name and the key-value pairs, also when they come
/// grouped in their parentheses (a statement's `html(lang: "en")`)
fn attributes(node: &Node, defined: &HashSet<String>) -> Option<(String, Vec<Node>)> {
	let Node::List(items, _, _) = node.drop_meta() else { return None };
	let (name, pairs) = items.split_first()?;
	let pairs = match pairs {
		[group] => match group.drop_meta() {
			Node::List(grouped, Bracket::Round, _) => grouped.as_slice(),
			_ => pairs,
		},
		_ => pairs,
	};
	let Node::Symbol(name) = name.drop_meta() else { return None };
	let all_pairs = !pairs.is_empty() && pairs.iter().all(|pair| matches!(pair.drop_meta(), Node::Key(_, Op::Colon, _)));
	(all_pairs && !defined.contains(name)).then(|| (name.clone(), pairs.to_vec()))
}

/// HTML's own attribute form (P188): `input{type="text"}`, `label(for="pwd")` and `p {class="note"}` are
/// `input{type:"text"}`, `label(for:"pwd")` and `p {class:"note"}`, before soft_keywords reads `class = …` as a
/// definition; in an element any other `name = value` stays the assignment
pub fn lower_html_attributes(node: Node) -> Node {
	match node {
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower_html_attributes(*node)), data },
		Node::Key(tag, Op::Colon, body) if is_element_name(&tag) => Node::Key(tag, Op::Colon, Box::new(lower_html_attributes(attributes_of_group(*body)))),
		Node::List(items, bracket, separator) if items.first().is_some_and(is_element_name) && items.len() > 1 => {
			let mut items = items.into_iter();
			let tag = items.next().into_iter();
			let parts = items.map(|item| lower_html_attributes(attributes_of_group(html_attribute(item))));
			Node::List(tag.chain(parts).collect(), bracket, separator)
		}
		other => other.map_children(lower_html_attributes),
	}
}

fn is_element_name(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(tag) if crate::markup::is_element_tag(tag))
}

/// The items of an element's `{…}` or `(…)`, `name = value` of an attribute as `name: value`
fn attributes_of_group(group: Node) -> Node {
	match group {
		Node::Meta { node, data } => Node::Meta { node: Box::new(attributes_of_group(*node)), data },
		Node::List(items, bracket @ (Bracket::Curly | Bracket::Round), separator) => Node::List(items.into_iter().map(html_attribute).collect(), bracket, separator),
		other => other,
	}
}

fn html_attribute(item: Node) -> Node {
	match item {
		Node::Meta { node, data } => Node::Meta { node: Box::new(html_attribute(*node)), data },
		Node::Key(name, Op::Assign, value) if matches!(name.drop_meta(), Node::Symbol(word) if crate::markup::names_attribute(word)) => Node::Key(name, Op::Colon, value),
		other => other,
	}
}

/// `[li{t} for t in ts]`, `ts.map(…)` or `(if c then a else b)` among an element's children: its items are children, so it stays a list
/// (`[…]` around it) when it is lowered to statements, which in a block would run instead of being an item
fn is_computed_children(item: &Node) -> bool {
	match item.drop_meta() {
		Node::List(items, Bracket::Square, _) => items.iter().any(|part| matches!(part.drop_meta(), Node::List(words, _, _) if matches!(words.first().map(Node::drop_meta), Some(Node::Symbol(word)) if word == FOR_WORD))),
		Node::Key(_, Op::Dot, _) => true,
		// `(if shown then Fruit() else [])`: a condition, which in a block would run as a statement
		Node::List(items, Bracket::Round, _) => matches!(items.as_slice(), [single] if matches!(single.drop_meta(), Node::Key(_, Op::Then | Op::Else, _))),
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
