//! The items of a tag's block, `word{…}` of a word the program does not define (wiki/mark.md, data.md): `div(class:"x")`
//! among them is the tag `div{class:"x"}`, `label(for:pwd):"Password"` the tag `label{for:pwd "Password"}` (card g-_alg),
//! and a function word takes as many items as it has parameters. Only data takes this reading: a name the program
//! defines stays its call, and outside a tag block a call of an unbound name stays the loud error (P92). An HTML
//! element's children have rules of their own (markup_tags.rs).

use crate::lowering::markup_tags;
use crate::lowering::nodes::key;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

pub fn lower(node: Node) -> Node {
	// `label(for:pwd):"Password"` itself would read as a definition: only `:=`, function keywords and assignments define
	let defined = crate::library_words::defined_names(&node);
	let arities = fixed_arities(&node);
	outside_tags(markup_tags::spaced_elements(node, &defined), &defined, &arities)
}

/// The program's functions taking a fixed number of values, by name
fn fixed_arities(program: &Node) -> HashMap<String, usize> {
	let mut arities = HashMap::new();
	let mut overloaded = HashSet::new();
	for head in crate::modules::statements_of(program).iter().filter_map(crate::lowering::nodes::function_head) {
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

fn outside_tags(node: Node, defined: &HashSet<String>, arities: &HashMap<String, usize>) -> Node {
	match tag(&node, defined) {
		Some(_) => tag_with_attributes(node, defined, arities),
		None => node.map_children(|child| outside_tags(child, defined, arities)),
	}
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
				let items = match is_element {
					true => markup_tags::element_children(items, &separator, defined),
					false => items,
				};
				let items = calls_among(items, arities, defined);
				let items = handlers_kept(items, |item| {
					let item = tag_with_attributes(attributed_tag(item, defined), defined, arities);
					match is_element {
						true => markup_tags::element_child(item),
						false => item,
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
pub(crate) fn attributes(node: &Node, defined: &HashSet<String>) -> Option<(String, Vec<Node>)> {
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

/// `li{ form post "/t" { … } "tea" }`, `friends{ g 2 "x" }`: among a tag's items a function word takes as many as it has
/// parameters, as everywhere (decision function-words-consume), and the items after them stay items
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

pub(crate) fn tag_node(name: String, items: Vec<Node>) -> Node {
	key(Node::Symbol(name), Op::Colon, Node::List(items, Bracket::Curly, Separator::Space))
}
