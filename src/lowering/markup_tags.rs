//! wiki/mark.md: inside a tag's block `div(class:"form-group")` is the tag `div{class:"form-group"}`, and
//! `label(for:pwd):"Password"` the tag `label{for:pwd "Password"}` (samples/html.warp, card g-_alg). Only data takes
//! this reading: a name the program defines stays its call, and outside a tag block a call of an unbound name stays
//! the loud error (P92). A comprehension or method call among an element's children gives children (card web-keyed).

use super::nodes::key;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashSet;

const FOR_WORD: &str = "for";
const IN_WORD: &str = "in";
const ALL_WORD: &str = "all";
const TAG_ITEM: &str = "tag·item"; // the loop variable of `li all xs`
/// `form post "/todos" { … }`: a form written as the route it sends to (card g_mSEw)
const FORM_TAG: &str = "form";
const METHOD_ATTRIBUTE: &str = "method";
const ACTION_ATTRIBUTE: &str = "action";

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

/// `ul { … }` with a blank, as a statement or a child of an element: the element `ul{ … }` when the program does not define the name, so its
/// children read as the glued tag's do (`ul { for item in items { li: item } }`, samples/html_dsl.warp);
/// glued `html(lang: "en"){ … }` (a round list) the element `html{ lang: "en" … }`, its attributes before its children
/// (card markup-attribute)
fn spaced_element(node: Node, defined: &HashSet<String>) -> Node {
	if let Node::List(items, Bracket::None | Bracket::Round, _) = node.drop_meta() {
		if let [head, body] = items.as_slice() {
			if let (Some((tag, mut content)), Node::List(children, Bracket::Curly, separator)) = (element_head(head, defined), body.drop_meta()) {
				content.extend(children.iter().cloned());
				return key(Node::Symbol(tag), Op::Colon, Node::List(content, Bracket::Curly, separator.clone()));
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
				let items = if is_element { tags_over_values(spaced_children(items, &separator, defined), defined) } else { items };
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

/// An element's children with each spaced `ul { … }` the element; a lone one is the whole block, `{ ul { … } }`
fn spaced_children(items: Vec<Node>, separator: &Separator, defined: &HashSet<String>) -> Vec<Node> {
	match spaced_element(Node::List(items, Bracket::None, separator.clone()), defined) {
		Node::List(items, Bracket::None, _) => items.into_iter().map(|item| spaced_element(item, defined)).collect(),
		element => vec![element],
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

/// `form post "/todos" { body }` → `form{ method:"post" action:"/todos" body }`, unless the program defines form
pub fn lower_form_routes(node: Node) -> Node {
	if !node.mentions_any(&[FORM_TAG]) || crate::library_words::defined_names(&node).contains(FORM_TAG) {
		return node;
	}
	with_form_routes(node)
}

fn with_form_routes(node: Node) -> Node {
	let Node::List(items, bracket, separator) = node else { return node.map_children(with_form_routes) };
	let (mut written, mut rewritten): (Vec<Node>, bool) = (vec![], false);
	for item in items.into_iter().map(with_form_routes) {
		written.push(item);
		if let [.., form, method, action, block] = written.as_slice() {
			let method = method.drop_meta().name();
			if let (FORM_TAG, true, Node::List(body, Bracket::Curly, body_separator)) = (form.drop_meta().name().as_str(), crate::serve::METHODS.contains(&method.as_str()), block.drop_meta()) {
				let (action, body, body_separator) = (action.clone(), body.clone(), body_separator.clone());
				written.pop();
				written.truncate(written.len() - 3);
				let attribute = |name: &str, value: Node| key(Node::Symbol(name.into()), Op::Colon, value);
				let fields = [attribute(METHOD_ATTRIBUTE, Node::Text(method)), attribute(ACTION_ATTRIBUTE, action)].into_iter().chain(body);
				written.push(key(Node::Symbol(FORM_TAG.into()), Op::Colon, Node::List(fields.collect(), Bracket::Curly, body_separator)));
				rewritten = true;
			}
		}
	}
	match written.len() {
		1 if rewritten && bracket == Bracket::None => written.pop().expect("one"),
		_ => Node::List(written, bracket, separator),
	}
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

/// Among an element's children a tag word applied to values, as broadcasting applies a function (card ul-li):
/// `li all fruits` and `li ["apple" "pear"]` give one tag per item (`[li{item} for item in …]`), `li "apple"` the
/// one tag `li{"apple"}`; a bare word before anything else stays a child
fn tags_over_values(children: Vec<Node>, defined: &HashSet<String>) -> Vec<Node> {
	let tag_word = |node: &Node| match node.drop_meta() {
		Node::Symbol(word) if crate::markup::is_element_tag(word) && !defined.contains(word) => Some(word.clone()),
		_ => None,
	};
	let is_word = |node: &Node, word: &str| matches!(node.drop_meta(), Node::Symbol(symbol) if symbol == word);
	let mut result = Vec::with_capacity(children.len());
	let mut rest = children.into_iter().peekable();
	while let Some(child) = rest.next() {
		let Some(tag) = tag_word(&child) else {
			result.push(child);
			continue;
		};
		let next = rest.peek().map(Node::drop_meta);
		if next.is_some_and(|node| is_word(node, ALL_WORD)) {
			rest.next();
			match rest.next() {
				Some(values) => result.push(tag_per_item(tag, values)),
				None => result.extend([child, Node::Symbol(ALL_WORD.into())]),
			}
		} else if matches!(next, Some(Node::List(_, Bracket::Square, _))) {
			result.push(tag_per_item(tag, rest.next().unwrap()));
		} else if matches!(next, Some(Node::Text(_) | Node::Number(_))) {
			result.push(tag_node(tag, vec![rest.next().unwrap()]));
		} else {
			result.push(child);
		}
	}
	result
}

/// `[tag{item} for item in values]`, the comprehension markup reads as children
fn tag_per_item(tag: String, values: Node) -> Node {
	let item = Node::Symbol(TAG_ITEM.into());
	let header = Node::List(vec![Node::Symbol(FOR_WORD.into()), item.clone(), Node::Symbol(IN_WORD.into()), values, Node::Empty], Bracket::None, Separator::Space);
	Node::List(vec![tag_node(tag, vec![item]), header], Bracket::Square, Separator::Space)
}

fn tag_node(name: String, items: Vec<Node>) -> Node {
	key(Node::Symbol(name), Op::Colon, Node::List(items, Bracket::Curly, Separator::Space))
}
