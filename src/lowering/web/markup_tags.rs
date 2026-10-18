//! wiki/mark.md: inside a tag's block `div(class:"form-group")` is the tag `div{class:"form-group"}`, and
//! `label(for:pwd):"Password"` the tag `label{for:pwd "Password"}` (samples/web/html.warp, card g-_alg). Only data takes
//! this reading: a name the program defines stays its call, and outside a tag block a call of an unbound name stays
//! the loud error (P92). A comprehension or method call among an element's children gives children (card web-keyed).

use super::nodes::key;
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::{ALL_WORD, FOR_WORD, IN_WORD};
use std::collections::{HashMap, HashSet};

const TAG_ITEM: &str = "tag·item"; // the loop variable of `li all xs`

pub fn lower(node: Node) -> Node {
	// `label(for:pwd):"Password"` itself would read as a definition: only `:=`, function keywords and assignments define
	let defined = crate::library_words::defined_names(&node);
	let arities = fixed_arities(&node);
	outside_tags(spaced_elements(node, &defined), &defined, &arities)
}

/// The program's functions taking a fixed number of values, by name
fn fixed_arities(program: &Node) -> HashMap<String, usize> {
	let mut arities = HashMap::new();
	let mut overloaded = HashSet::new();
	for head in crate::modules::statements_of(program).iter().filter_map(super::nodes::function_head) {
		match head.arity() {
			(fewest, most) if fewest == most && arities.get(head.name).is_none_or(|known| *known == fewest) => {
				arities.insert(head.name.to_string(), fewest);
			}
			_ => {
				overloaded.insert(head.name.to_string());
			}
		}
	}
	arities.retain(|name, _| !overloaded.contains(name));
	arities
}

/// How many values `name` takes when that is fixed: the program's function, else the standard function it calls, whose
/// module joins the program only later (modules::resolve)
fn fixed_arity(name: &str, arities: &HashMap<String, usize>, defined: &HashSet<String>) -> Option<usize> {
	if defined.contains(name) {
		return arities.get(name).copied();
	}
	if !crate::modules::std_function_names().contains(name) {
		return None;
	}
	let module = crate::modules::parsed_std_module(crate::modules::std_module_defining(name)?)?;
	fixed_arities(&module).get(name).copied()
}

/// Only whole statements of the main level: deeper `p { … }` is `match p { … }`, a parameter p and its block
fn spaced_elements(program: Node, defined: &HashSet<String>) -> Node {
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

fn outside_tags(node: Node, defined: &HashSet<String>, arities: &HashMap<String, usize>) -> Node {
	match tag(&node, defined) {
		Some(_) => tag_with_attributes(node, defined, arities),
		None => node.map_children(|child| outside_tags(child, defined, arities)),
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

/// `name{…}` of a name the program does not define: its body items
fn tag<'a>(node: &'a Node, defined: &HashSet<String>) -> Option<&'a [Node]> {
	let Node::Key(name, Op::Colon, body) = node.drop_meta() else {
		return None;
	};
	let name = name.symbol_name()?;
	match body.drop_meta() {
		Node::List(items, Bracket::Curly, _) if !defined.contains(name) => Some(items),
		_ => None,
	}
}

/// A tag whose items, and their items in turn, read `name(key:value…)` as the tag `name{key:value…}`
fn tag_with_attributes(node: Node, defined: &HashSet<String>, arities: &HashMap<String, usize>) -> Node {
	match node {
		Node::Key(name, Op::Colon, body) => match *body {
			Node::List(items, Bracket::Curly, separator) => {
				let is_element =
					matches!(name.drop_meta(), Node::Symbol(tag) if crate::web::markup::is_element_tag(tag));
				let items = if is_element {
					let children = tags_over_values(spaced_children(items, &separator, defined), defined);
					calls_among(children, arities, defined)
				} else {
					items
				};
				let items = handlers_kept(items, |item| {
					let item = tag_with_attributes(attributed_tag(item, defined), defined, arities);
					let item = if is_element { loop_as_comprehension(item) } else { item };
					if is_element && is_computed_children(&item) {
						Node::List(vec![item], Bracket::Square, Separator::None)
					} else {
						item
					}
				});
				Node::Key(name, Op::Colon, Box::new(Node::List(items, Bracket::Curly, separator)))
			}
			body => Node::Key(name, Op::Colon, Box::new(tag_with_attributes(body, defined, arities))),
		},
		// `[td{…} for job in jobs]`, also from a loop among the children: its elements read as the others do
		other => other.map_children(|child| tag_with_attributes(child, defined, arities)),
	}
}

/// A tag's items, each rewritten by `rewrite` but its handlers (`on click { emit saved{id: 7} }`): their bodies are
/// code, kept as written, not markup (card page-custom-event: `emit saved{…}` read as the tag `emit{saved{…}}`)
fn handlers_kept(items: Vec<Node>, mut rewrite: impl FnMut(Node) -> Node) -> Vec<Node> {
	let mut kept = Vec::with_capacity(items.len());
	let mut index = 0;
	while index < items.len() {
		index += match crate::element_events::handler_at(&items, index) {
			Some((_, _, length)) => {
				kept.extend(items[index..index + length].iter().cloned());
				length
			}
			None => {
				kept.push(rewrite(items[index].clone()));
				1
			}
		};
	}
	kept
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
	let Node::List(items, _, _) = node.drop_meta() else {
		return None;
	};
	// `on submit { … }` on a line of its own is a handler (element_events.rs), not the tag `on{submit{…}}`
	if crate::element_events::handler_at(items, 0).is_some() {
		return None;
	}
	let (name, pairs) = items.split_first()?;
	let pairs = match pairs {
		[group] => match group.drop_meta() {
			Node::List(grouped, Bracket::Round, _) => grouped.as_slice(),
			_ => pairs,
		},
		_ => pairs,
	};
	let name = name.symbol_name()?;
	let all_pairs =
		!pairs.is_empty() && pairs.iter().all(|pair| matches!(pair.drop_meta(), Node::Key(_, Op::Colon, _)));
	(all_pairs && !defined.contains(name)).then(|| (name.to_string(), pairs.to_vec()))
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

/// `li{ form post "/t" { … } "tea" }`: among an element's children a function word takes as many as it has parameters,
/// as everywhere (decision function-words-consume), and the children after them stay children
fn calls_among(children: Vec<Node>, arities: &HashMap<String, usize>, defined: &HashSet<String>) -> Vec<Node> {
	let whole = children.len();
	let mut result = Vec::with_capacity(whole);
	let mut rest = children.into_iter();
	while let Some(child) = rest.next() {
		match child.drop_meta().symbol_name().and_then(|name| fixed_arity(name, arities, defined)) {
			Some(count) if count > 0 && rest.len() >= count && whole > count + 1 => {
				let call = std::iter::once(child).chain(rest.by_ref().take(count)).collect();
				result.push(Node::List(call, Bracket::Round, Separator::Space));
			}
			_ => result.push(child),
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

fn tag_node(name: String, items: Vec<Node>) -> Node {
	key(Node::Symbol(name), Op::Colon, Node::List(items, Bracket::Curly, Separator::Space))
}
