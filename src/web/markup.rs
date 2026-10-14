//! Markup values (card web-dom, notes/web_framework.md): `div{ class:"box" h1{"Hi"} }` is the element div with the
//! attribute class and the child h1. Which values are markup is decided here; their HTML comes from the one renderer,
//! lib/markup.warp's to_html, also for the CLI and the playground (a page runs it itself as page·html).

use crate::node::{Bracket, Node};
use crate::operators::Op;
use crate::words::STYLE_WORD;
use std::collections::HashMap;
use std::sync::OnceLock;

const MARKUP_MODULE: &str = "markup";
/// The lists of lib/markup.warp naming the HTML elements a markup key may name (custom elements are not markup yet)
/// and the attributes
const ELEMENTS_LIST: &str = "html_elements";
const ATTRIBUTES_LIST: &str = "html_attributes";
const META_ATTRIBUTES_LIST: &str = "html_meta_attributes";
const VOID_ELEMENTS_LIST: &str = "html_void_elements";
const BOOLEAN_ATTRIBUTES_LIST: &str = "html_boolean_attributes";
const HEAD: &str = "head";
/// `data-warp-click` (element_events.rs) and any other data attribute
const DATA_ATTRIBUTE_PREFIX: &str = "data-";
const ARIA_ATTRIBUTE_PREFIX: &str = "aria-";
/// `li{ key: todo.id … }` names a list item (card web-keyed)
const KEY: &str = "key";
/// The value in place of the name `rendered`, assigned first: as an argument `ul: {…}` would name a parameter
const RENDER_PROGRAM: &str = "use markup\nshown = rendered\nto_html(shown)";
const RENDERED: &str = "rendered";
/// the attribute naming the component of an element, whose style sheets style only its elements (card web-scoped)
pub const SCOPE_ATTRIBUTE: &str = "data-warp-scope";
/// how deep holes go: a component showing itself inside would go on
const MAX_HOLE_DEPTH: usize = 64;
/// Elements whose text lib/markup.warp renders its own way (style rules, raw script, the chosen option): no text holes
const TEXT_ELEMENTS: [&str; 5] = ["style", "script", "option", "select", "textarea"];

/// Does the word name an HTML element
pub fn is_element_tag(word: &str) -> bool {
	element_names().iter().any(|element| element == word)
}

/// `my-card`: a page's own element is named with a dash, as no attribute is (html_is_custom_element of lib/markup.warp)
pub fn is_custom_element(word: &str) -> bool {
	word.contains('-') && !names_attribute(word) && !word.starts_with(ARIA_ATTRIBUTE_PREFIX)
}

/// The HTML elements lib/markup.warp renders
pub fn element_names() -> &'static [String] {
	static ELEMENTS: OnceLock<Vec<String>> = OnceLock::new();
	ELEMENTS.get_or_init(|| crate::modules::std_module_list(MARKUP_MODULE, ELEMENTS_LIST))
}

/// Does the word name an HTML attribute: a listed one or a `data-` one
pub fn names_attribute(word: &str) -> bool {
	word.starts_with(DATA_ATTRIBUTE_PREFIX) || is_attribute_name(word)
}

fn is_attribute_name(word: &str) -> bool {
	attribute_names().iter().any(|attribute| attribute == word)
}

/// The attributes lib/markup.warp writes as attributes (besides `data-…`, `aria-…` and key)
pub fn attribute_names() -> &'static [String] {
	static ATTRIBUTES: OnceLock<Vec<String>> = OnceLock::new();
	ATTRIBUTES.get_or_init(|| crate::modules::std_module_list(MARKUP_MODULE, ATTRIBUTES_LIST))
}

/// A meta element's own attributes: `meta{ charset: "utf-8" }` is the element, `meta: { author: "Ann" }` names metas
pub fn meta_attribute_names() -> &'static [String] {
	static ATTRIBUTES: OnceLock<Vec<String>> = OnceLock::new();
	ATTRIBUTES.get_or_init(|| crate::modules::std_module_list(MARKUP_MODULE, META_ATTRIBUTES_LIST))
}

/// An attribute present or absent (`checked: done`), set by yes or no
pub fn is_boolean_attribute(name: &str) -> bool {
	static BOOLEAN: OnceLock<Vec<String>> = OnceLock::new();
	BOOLEAN
		.get_or_init(|| crate::modules::std_module_list(MARKUP_MODULE, BOOLEAN_ATTRIBUTES_LIST))
		.iter()
		.any(|attribute| attribute == name)
}

/// Does the element take no content (`img`, `input`, `br`): lib/markup.warp writes no closing tag, nor what it holds
pub fn is_void_element(tag: &str) -> bool {
	static VOID_ELEMENTS: OnceLock<Vec<String>> = OnceLock::new();
	VOID_ELEMENTS
		.get_or_init(|| crate::modules::std_module_list(MARKUP_MODULE, VOID_ELEMENTS_LIST))
		.iter()
		.any(|element| element == tag)
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
	let Node::Key(name, Op::Colon, value) = node.drop_meta() else {
		return None;
	};
	let Node::Symbol(name) = name.drop_meta() else {
		return None;
	};
	let is_attribute = name.starts_with(DATA_ATTRIBUTE_PREFIX)
		|| name == KEY
		|| (is_attribute_name(name) && !(parent == HEAD && is_element_tag(name)));
	is_attribute.then_some((name.as_str(), value.as_ref()))
}

/// The items of an element's content: its attributes and children in order; an element handler (`on click {…}`, still
/// written so in a component's markup) is neither
fn content_items(content: &Node) -> Vec<&Node> {
	match content.drop_meta() {
		Node::List(items, Bracket::Curly | Bracket::None, _) => {
			let mut kept = vec![];
			let mut index = 0;
			while index < items.len() {
				match crate::element_events::handler_at(items, index) {
					Some((_, _, length)) => index += length,
					None => {
						kept.push(&items[index]);
						index += 1;
					}
				}
			}
			kept
		}
		Node::Empty => vec![],
		single => vec![single],
	}
}

/// A computed text between an element's children (card web-fine-text-nodes): the page sets that text node only
pub struct TextHole {
	/// the element's path, as a hole's
	pub path: Vec<usize>,
	/// the element's children before the text, so the text node follows the child `after - 1` (or comes first)
	pub after: usize,
	pub text: Node,
}

/// The parts of shown markup the page reads on their own: elements, each with its path, and texts
pub type Holes = (Vec<(Vec<usize>, Node)>, Vec<TextHole>);

/// The holes of markup as written (card web-fine-holes, notes/web_framework.md step 3): the outermost elements holding
/// a computed text, attribute or child directly, each with its path, the element indices from the root down (as the
/// page's `children` count them). The other elements are fixed. Empty when the root itself is one. An element whose
/// computed parts are all text_holes is none, the root too (card web-fine-rest)
pub fn holes(markup: &Node) -> Vec<(Vec<usize>, Node)> {
	holes_through(markup, &HashMap::new()).0
}

/// The computed texts standing alone between an element's children, the root's too, which the page sets as text nodes;
/// empty when the root itself is an element hole
pub fn text_holes(markup: &Node) -> Vec<TextHole> {
	holes_through(markup, &HashMap::new()).1
}

/// The holes of markup calling components (card web-fine-components): a call of one of `components` (by name, with the
/// markup it shows) is that markup's element, its holes the instance's
pub fn holes_through<'a>(markup: &'a Node, components: &'a HashMap<String, Node>) -> Holes {
	let mut holes = (vec![], vec![]);
	Shapes { components }.collect(markup, &mut vec![], &mut holes);
	if holes.0.iter().any(|(path, _)| path.is_empty()) {
		return (vec![], vec![]);
	}
	holes
}

/// What the children of markup show: a component's call its markup, any other child itself
struct Shapes<'a> {
	components: &'a HashMap<String, Node>,
}

impl<'a> Shapes<'a> {
	fn shown(&self, child: &'a Node) -> &'a Node {
		let Node::List(items, Bracket::Round | Bracket::None, _) = child.drop_meta() else {
			return child;
		};
		items.first().and_then(Node::symbol_name).and_then(|name| self.components.get(name)).unwrap_or(child)
	}

	fn collect(&self, element: &'a Node, path: &mut Vec<usize>, holes: &mut Holes) {
		let element = self.shown(element);
		let Some((tag, content)) = element_parts(element) else {
			return;
		};
		// a component showing itself inside: its elements stay one hole
		if path.len() > MAX_HOLE_DEPTH {
			return holes.0.push((path.clone(), element.clone()));
		}
		let items = content_items(content);
		let computed = |item: &&Node| match named_attribute(item, tag) {
			Some((_, value)) => !is_literal(value),
			None => !self.is_fixed(item),
		};
		if items.iter().any(computed) {
			let Some(texts) = self.lone_texts(tag, &items) else {
				return holes.0.push((path.clone(), element.clone()));
			};
			holes.1.extend(texts.into_iter().map(|(after, text)| TextHole {
				path: path.clone(),
				after,
				text: text.clone(),
			}));
		}
		let children = items.into_iter().filter(|item| named_attribute(item, tag).is_none());
		for (index, child) in children.flat_map(|child| self.child_elements(child)).enumerate() {
			path.push(index);
			self.collect(child, path, holes);
			path.pop();
		}
	}

	/// The computed parts of an element's items as text holes, each with the child elements before it: only when every
	/// computed part is a text (`"count " + n`), its neighbours are elements (a text beside it would share its DOM text
	/// node), no attribute is computed, and the element has child elements (else the element is as small as its text)
	fn lone_texts(&self, tag: &str, items: &[&'a Node]) -> Option<Vec<(usize, &'a Node)>> {
		if TEXT_ELEMENTS.contains(&tag) {
			return None;
		}
		let mut children: Vec<&Node> = vec![];
		for item in items {
			match named_attribute(item, tag) {
				Some((_, value)) if !is_literal(value) => return None,
				Some(_) => {}
				None if self.is_fixed(item) => children.extend(self.fixed_nodes(item)),
				None => children.push(item),
			}
		}
		let is_element = |child: &&Node| element_parts(child).is_some();
		let mut texts = vec![];
		for (position, child) in children.iter().enumerate() {
			if is_element(child) || self.is_fixed(child) {
				continue;
			}
			let neighbours = [position.checked_sub(1), Some(position + 1)];
			let alone =
				neighbours.into_iter().flatten().filter_map(|neighbour| children.get(neighbour)).all(is_element);
			if !is_computed_text(child) || !alone {
				return None;
			}
			texts.push((children[..position].iter().filter(|before| is_element(before)).count(), *child));
		}
		(children.iter().any(is_element)).then_some(texts)
	}

	/// The DOM nodes of a fixed child as it is written: its items if it is a literal list
	fn fixed_nodes(&self, child: &'a Node) -> Vec<&'a Node> {
		match child.drop_meta() {
			Node::List(items, Bracket::Square, _) => items.iter().flat_map(|item| self.fixed_nodes(item)).collect(),
			_ => vec![self.shown(child)],
		}
	}

	/// The elements a fixed child shows: itself, or those of a literal list
	fn child_elements(&self, child: &'a Node) -> Vec<&'a Node> {
		let child = self.shown(child);
		match child.drop_meta() {
			Node::List(items, Bracket::Square, _) => items.iter().flat_map(|item| self.child_elements(item)).collect(),
			_ if element_parts(child).is_some() => vec![child],
			_ => vec![],
		}
	}

	/// A child the page shows the same after any change: a literal, an element, or a list of them
	fn is_fixed(&self, child: &'a Node) -> bool {
		let child = self.shown(child);
		match child.drop_meta() {
			Node::List(items, Bracket::Square, _) => items.iter().all(|item| self.is_fixed(item)),
			_ => element_parts(child).is_some() || is_literal(child),
		}
	}
}

/// `"count " + n`, `"a" + b + "c"`: a sum starting with a literal text, which shows as text whatever it adds
fn is_computed_text(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(left, Op::Add, _) => left.is_text_literal() || is_computed_text(left),
		_ => false,
	}
}

fn is_literal(node: &Node) -> bool {
	match node.drop_meta() {
		Node::List(items, Bracket::Square, _) => items.iter().all(is_literal),
		other => {
			matches!(other, Node::Text(_) | Node::Char(_) | Node::Number(_) | Node::True | Node::False | Node::Empty)
		}
	}
}

/// `style{ ".card": { … } }`: a style sheet among an element's items (not the inline `style: { color: … }`)
pub fn is_style_sheet(node: &Node) -> bool {
	let Node::Key(name, Op::Colon, value) = node.drop_meta() else {
		return false;
	};
	name.drop_meta().name() == STYLE_WORD && !is_declarations(value)
}

/// `{ color: theme padding: 8 }`: CSS properties with plain values
fn is_declarations(value: &Node) -> bool {
	content_items(value).iter().all(|item| matches!(item.drop_meta(), Node::Key(_, Op::Colon, value) if !matches!(value.drop_meta(), Node::Key(..) | Node::List(_, Bracket::Curly, _))))
}

/// Text made safe inside HTML: its markup characters as entities
pub fn escaped(text: &str) -> String {
	text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// The HTML of a markup value, any other value as escaped text, by lib/markup.warp's to_html; a failed rendering is
/// its error, shown, never an empty page. Quietly: the program that made the value already warned about it, and the
/// renderer's positions are not the program's
pub fn to_html(node: &Node) -> String {
	let program = crate::law::substitute(
		&crate::warp_parser::parse(RENDER_PROGRAM),
		&HashMap::from([(RENDERED.to_string(), node.clone())]),
	);
	match crate::errors::diagnostic::quietly(|| crate::pipeline::eval_parsed(program, RENDER_PROGRAM)).drop_meta() {
		Node::Text(html) => html.clone(),
		failed => format!("<pre>{}</pre>", escaped(&failed.serialize())),
	}
}
