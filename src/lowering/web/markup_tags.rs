//! An HTML element's own children rules, beside those of any tag's items (data/tag_children.rs): `ul { … }` with a
//! blank is the element, `li all fruits` one li per fruit, and a loop, comprehension or method call among the
//! children gives children (card web-keyed)

use crate::lowering::nodes::key;
use crate::lowering::tag_children::{attributes, tag_node};
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::{ALL_WORD, FOR_WORD, IN_WORD};
use std::collections::HashSet;

const TAG_ITEM: &str = "tag·item"; // the loop variable of `li all xs`

/// An element's children: spaced elements, tag words applied to values
pub(crate) fn element_children(items: Vec<Node>, separator: &Separator, defined: &HashSet<String>) -> Vec<Node> {
	tags_over_values(spaced_children(items, separator, defined), defined)
}

/// A child giving children: a loop as its comprehension, a computed list kept a list
pub(crate) fn element_child(item: Node) -> Node {
	let item = loop_as_comprehension(item);
	match is_computed_children(&item) {
		true => Node::List(vec![item], Bracket::Square, Separator::None),
		false => item,
	}
}

/// Only whole statements of the main level: deeper `p { … }` is `match p { … }`, a parameter p and its block
pub(crate) fn spaced_elements(program: Node, defined: &HashSet<String>) -> Node {
	match program {
		Node::List(statements, bracket, separator)
			if crate::variable_signals::is_statement_list(&bracket, &separator) =>
		{
			Node::List(
				statements.into_iter().map(|statement| spaced_element(statement, defined)).collect(),
				bracket,
				separator,
			)
		}
		single => spaced_element(single, defined),
	}
}

/// `ul { … }` with a blank, as a statement or a child of an element: the element `ul{ … }` when the program does not define the name, so its
/// children read as the glued tag's do (`ul { for item in items { li: item } }`, samples/web/html_dsl.warp);
/// glued `html(lang: "en"){ … }` (a round list) the element `html{ lang: "en" … }`, its attributes before its children
/// (card markup-attribute)
fn spaced_element(node: Node, defined: &HashSet<String>) -> Node {
	if let Node::List(items, Bracket::None | Bracket::Round, _) = node.drop_meta() {
		if let [head, body] = items.as_slice() {
			if let (Some((tag, mut content)), Node::List(children, Bracket::Curly, separator)) =
				(element_head(head, defined), body.drop_meta())
			{
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
	parts.filter(|(tag, _)| crate::web::markup::is_element_tag(tag) && !defined.contains(tag))
}

/// An element's children with each spaced `ul { … }` the element; a lone one is the whole block, `{ ul { … } }`
fn spaced_children(items: Vec<Node>, separator: &Separator, defined: &HashSet<String>) -> Vec<Node> {
	match spaced_element(Node::List(items, Bracket::None, separator.clone()), defined) {
		Node::List(items, Bracket::None, _) => items.into_iter().map(|item| spaced_element(item, defined)).collect(),
		element => vec![element],
	}
}

/// `[li{t} for t in ts]`, `ts.map(…)` or `(if c then a else b)` among an element's children: its items are children, so it stays a list
/// (`[…]` around it) when it is lowered to statements, which in a block would run instead of being an item
fn is_computed_children(item: &Node) -> bool {
	match item.drop_meta() {
		Node::List(items, Bracket::Square, _) => items.iter().any(|part| matches!(part.drop_meta(), Node::List(words, _, _) if words.first().and_then(Node::symbol_name).is_some_and(|word| word == FOR_WORD))),
		Node::Key(_, Op::Dot, _) => true,
		// `(if shown then Fruit() else [])`, also without parentheses: a condition, which in a block would run as a statement
		Node::List(items, Bracket::Round, _) => matches!(items.as_slice(), [single] if is_condition(single)),
		_ => is_condition(item),
	}
}

fn is_condition(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(_, Op::Then | Op::Else, _))
}

/// `ul{ for t in todos { li{t} } }`: a loop among an element's children gives one child per item, as the comprehension
/// `[li{t} for t in todos]` (`[element, (for t in todos ø)]` as the parser groups it); a body of several items gives
/// the value of its last (card markup-ul)
fn loop_as_comprehension(item: Node) -> Node {
	let Node::List(words, Bracket::None, _) = item.drop_meta() else {
		return item;
	};
	let [for_word, variable, in_word, sequence, body] = words.as_slice() else {
		return item;
	};
	let is_word = |node: &Node, word: &str| matches!(node.drop_meta(), Node::Symbol(symbol) if symbol == word);
	if !is_word(for_word, FOR_WORD) || !is_word(in_word, IN_WORD) {
		return item;
	}
	let element = match body.drop_meta() {
		Node::List(items, Bracket::Curly, _) if items.len() == 1 => items[0].clone(),
		Node::List(items, Bracket::Curly, separator) => Node::List(items.clone(), Bracket::Round, separator.clone()),
		other => other.clone(),
	};
	let header = Node::List(
		vec![for_word.clone(), variable.clone(), in_word.clone(), sequence.clone(), Node::Empty],
		Bracket::None,
		Separator::Space,
	);
	Node::List(vec![element, header], Bracket::Square, Separator::Space)
}

/// Among an element's children a tag word applied to values, as broadcasting applies a function (card ul-li):
/// `li all fruits` and `li ["apple" "pear"]` give one tag per item (`[li{item} for item in …]`), `li "apple"` the
/// one tag `li{"apple"}`; a bare word before anything else stays a child
fn tags_over_values(children: Vec<Node>, defined: &HashSet<String>) -> Vec<Node> {
	let tag_word = |node: &Node| match node.drop_meta() {
		Node::Symbol(word) if crate::web::markup::is_element_tag(word) && !defined.contains(word) => Some(word.clone()),
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
				None => result.extend([child, symbol(ALL_WORD)]),
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
	let item = symbol(TAG_ITEM);
	let header = Node::List(
		vec![symbol(FOR_WORD), item.clone(), symbol(IN_WORD), values, Node::Empty],
		Bracket::None,
		Separator::Space,
	);
	Node::List(vec![tag_node(tag, vec![item]), header], Bracket::Square, Separator::Space)
}
