//! Markup values (card web-dom, notes/web_framework.md): `div{ class:"box" h1{"Hi"} }` is the element div with the
//! attribute class and the child h1. Which values are markup is decided here; their HTML comes from the one renderer,
//! lib/markup.warp's to_html, also for the CLI and the playground (a page runs it itself as page·html).

use crate::node::{Bracket, Node};
use crate::operators::Op;
use std::collections::HashMap;
use std::sync::OnceLock;

const MARKUP_MODULE: &str = "markup";
/// The lists of lib/markup.warp naming the HTML elements a markup key may name (custom elements are not markup yet)
/// and the attributes
const ELEMENTS_LIST: &str = "html_elements";
const ATTRIBUTES_LIST: &str = "html_attributes";
const HEAD: &str = "head";
/// `data-warp-click` (element_events.rs) and any other data attribute
const DATA_ATTRIBUTE_PREFIX: &str = "data-";
/// `li{ key: todo.id … }` names a list item (card web-keyed)
const KEY: &str = "key";
/// The value in place of the name `rendered`, assigned first: as an argument `ul: {…}` would name a parameter
const RENDER_PROGRAM: &str = "use markup\nshown = rendered\nto_html(shown)";
const RENDERED: &str = "rendered";
/// the element of a style sheet and the attribute of an inline style (card web-styles)
const STYLE: &str = "style";
/// the attribute naming the component of an element, whose style sheets style only its elements (card web-scoped)
pub const SCOPE_ATTRIBUTE: &str = "data-warp-scope";

/// Does the word name an HTML element
pub fn is_element_tag(word: &str) -> bool {
	static ELEMENTS: OnceLock<Vec<String>> = OnceLock::new();
	ELEMENTS.get_or_init(|| crate::modules::std_module_list(MARKUP_MODULE, ELEMENTS_LIST)).iter().any(|element| element == word)
}

/// Does the word name an HTML attribute: a listed one or a `data-` one
pub fn names_attribute(word: &str) -> bool {
	word.starts_with(DATA_ATTRIBUTE_PREFIX) || is_attribute_name(word)
}

fn is_attribute_name(word: &str) -> bool {
	static ATTRIBUTES: OnceLock<Vec<String>> = OnceLock::new();
	ATTRIBUTES.get_or_init(|| crate::modules::std_module_list(MARKUP_MODULE, ATTRIBUTES_LIST)).iter().any(|attribute| attribute == word)
}

/// Is the value an element (`div{…}`, `h1: "Hi"`, a key named after an element): what the CLI prints as HTML and the
/// page shows as DOM
pub fn is_markup(node: &Node) -> bool {
	element_parts(node).is_some()
}

/// An element's tag and content
fn element_parts(node: &Node) -> Option<(&str, &Node)> {
	match node.drop_meta() {
		Node::Key(tag, Op::Colon | Op::None, content) => match tag.drop_meta() {
			Node::Symbol(tag) if is_element_tag(tag) => Some((tag.as_str(), content.as_ref())),
			_ => None,
		},
		_ => None,
	}
}

/// `class: …` inside an element, its value plain or still to be computed (`title` is the element only in head)
fn named_attribute<'a>(node: &'a Node, parent: &str) -> Option<(&'a str, &'a Node)> {
	let Node::Key(name, Op::Colon, value) = node.drop_meta() else { return None };
	let Node::Symbol(name) = name.drop_meta() else { return None };
	let is_attribute = name.starts_with(DATA_ATTRIBUTE_PREFIX) || name == KEY || (is_attribute_name(name) && !(parent == HEAD && is_element_tag(name)));
	is_attribute.then_some((name.as_str(), value.as_ref()))
}

/// The items of an element's content: its attributes and children in order
fn content_items(content: &Node) -> Vec<&Node> {
	match content.drop_meta() {
		Node::List(items, Bracket::Curly | Bracket::None, _) => items.iter().collect(),
		Node::Empty => vec![],
		single => vec![single],
	}
}

/// The holes of markup as written (card web-fine-holes, notes/web_framework.md step 3): the outermost elements holding
/// a computed text, attribute or child directly, each with its path, the element indices from the root down (as the
/// page's `children` count them). The other elements are fixed. Empty when the root itself holds one.
pub fn holes(markup: &Node) -> Vec<(Vec<usize>, Node)> {
	let mut holes = vec![];
	collect_holes(markup, &mut vec![], &mut holes);
	if holes.iter().any(|(path, _)| path.is_empty()) {
		return vec![];
	}
	holes
}

fn collect_holes(element: &Node, path: &mut Vec<usize>, holes: &mut Vec<(Vec<usize>, Node)>) {
	let Some((tag, content)) = element_parts(element) else { return };
	let items = content_items(content);
	let computed = |item: &&Node| match named_attribute(item, tag) {
		Some((_, value)) => !is_literal(value),
		None => !is_fixed(item),
	};
	if items.iter().any(computed) {
		return holes.push((path.clone(), element.clone()));
	}
	let children = items.into_iter().filter(|item| named_attribute(item, tag).is_none());
	for (index, child) in children.flat_map(child_elements).enumerate() {
		path.push(index);
		collect_holes(child, path, holes);
		path.pop();
	}
}

/// The elements a fixed child shows: itself, or those of a literal list
fn child_elements(child: &Node) -> Vec<&Node> {
	match child.drop_meta() {
		Node::List(items, Bracket::Square, _) => items.iter().flat_map(child_elements).collect(),
		_ if element_parts(child).is_some() => vec![child],
		_ => vec![],
	}
}

/// A child the page shows the same after any change: a literal, an element, or a list of them
fn is_fixed(child: &Node) -> bool {
	match child.drop_meta() {
		Node::List(items, Bracket::Square, _) => items.iter().all(is_fixed),
		_ => element_parts(child).is_some() || is_literal(child),
	}
}

fn is_literal(node: &Node) -> bool {
	match node.drop_meta() {
		Node::List(items, Bracket::Square, _) => items.iter().all(is_literal),
		other => matches!(other, Node::Text(_) | Node::Char(_) | Node::Number(_) | Node::True | Node::False | Node::Empty),
	}
}

/// `style{ ".card": { … } }`: a style sheet among an element's items (not the inline `style: { color: … }`)
pub fn is_style_sheet(node: &Node) -> bool {
	let Node::Key(name, Op::Colon, value) = node.drop_meta() else { return false };
	name.drop_meta().name() == STYLE && !is_declarations(value)
}

/// `{ color: theme padding: 8 }`: CSS properties with plain values
fn is_declarations(value: &Node) -> bool {
	content_items(value).iter().all(|item| matches!(item.drop_meta(), Node::Key(_, Op::Colon, value) if !matches!(value.drop_meta(), Node::Key(..) | Node::List(_, Bracket::Curly, _))))
}

/// The HTML of a markup value, any other value as escaped text, by lib/markup.warp's to_html; a failed rendering is
/// its error, shown, never an empty page. Quietly: the program that made the value already warned about it, and the
/// renderer's positions are not the program's
pub fn to_html(node: &Node) -> String {
	let program = crate::law::substitute(&crate::warp_parser::parse(RENDER_PROGRAM), &HashMap::from([(RENDERED.to_string(), node.clone())]));
	match crate::diagnostic::quietly(|| crate::pipeline::eval_parsed(program, RENDER_PROGRAM)).drop_meta() {
		Node::Text(html) => html.clone(),
		failed => format!("<pre>{}</pre>", failed.serialize().replace('&', "&amp;").replace('<', "&lt;")),
	}
}
