//! Markup values as HTML (card web-dom, notes/web_framework.md step 1): `div{ class:"box" h1{"Hi"} }` is the element
//! div with the attribute class and the child h1. A key named after an HTML element whose value is a block (or, for a
//! name that is no attribute, a text) is an element; inside it a key named after an attribute with a plain value is an
//! attribute, anything else a child in order. Text is escaped. The CLI prints a markup value this way, the playground
//! shows it as DOM (src/web.rs report `html`).

use crate::node::{Bracket, Node};
use crate::operators::Op;

/// The HTML elements a markup key may name (custom elements are not markup yet)
const ELEMENTS: [&str; 74] = [
	"html", "head", "body", "title", "meta", "link", "style", "script", "base",
	"header", "footer", "main", "nav", "section", "article", "aside", "div", "span", "p", "h1", "h2", "h3", "h4", "h5", "h6",
	"a", "em", "strong", "b", "i", "u", "small", "code", "pre", "blockquote", "br", "hr", "img", "figure", "figcaption",
	"ul", "ol", "li", "dl", "dt", "dd", "table", "thead", "tbody", "tfoot", "tr", "th", "td", "caption",
	"form", "label", "input", "button", "select", "option", "textarea", "fieldset", "legend",
	"canvas", "svg", "video", "audio", "source", "iframe", "details", "summary", "dialog", "template", "slot",
];
/// Elements without content or closing tag
const VOID_ELEMENTS: [&str; 9] = ["br", "hr", "img", "input", "meta", "link", "base", "source", "col"];
/// Names that are attributes when their value is plain (`title` is the element only in head)
const ATTRIBUTES: [&str; 40] = [
	"class", "id", "href", "src", "alt", "title", "style", "type", "name", "value", "for", "rel", "charset", "content",
	"lang", "dir", "width", "height", "placeholder", "action", "method", "target", "disabled", "checked", "selected",
	"readonly", "required", "min", "max", "step", "rows", "cols", "colspan", "rowspan", "role", "tabindex", "hidden",
	"download", "label", "media",
];
const HEAD: &str = "head";

/// Is the value an element (or a list of them): what the CLI prints as HTML and the page shows as DOM
pub fn is_markup(node: &Node) -> bool {
	element_parts(node).is_some()
}

/// The HTML of a markup value; any other value as escaped text
pub fn to_html(node: &Node) -> String {
	let mut html = String::new();
	write_node(node, &mut html);
	html
}

/// An element's tag and content: `div{…}`, `h1: "Hi"`, `html:body:…` (a key named after an element)
fn element_parts(node: &Node) -> Option<(&str, &Node)> {
	match node.drop_meta() {
		Node::Key(tag, Op::Colon | Op::None, content) => match tag.drop_meta() {
			Node::Symbol(tag) if ELEMENTS.contains(&tag.as_str()) => Some((tag.as_str(), content.as_ref())),
			_ => None,
		},
		_ => None,
	}
}

/// `class:"box"` inside an element: an attribute when its name is one and its value plain
fn attribute_parts<'a>(node: &'a Node, parent: &str) -> Option<(&'a str, &'a Node)> {
	let Node::Key(name, Op::Colon, value) = node.drop_meta() else { return None };
	let Node::Symbol(name) = name.drop_meta() else { return None };
	let is_attribute = ATTRIBUTES.contains(&name.as_str()) && !(parent == HEAD && ELEMENTS.contains(&name.as_str()));
	let plain = matches!(value.drop_meta(), Node::Text(_) | Node::Char(_) | Node::Symbol(_) | Node::Number(_) | Node::True | Node::False)
		|| matches!(value.drop_meta(), Node::List(_, Bracket::Square, _));
	(is_attribute && plain).then_some((name.as_str(), value.as_ref()))
}

fn write_node(node: &Node, html: &mut String) {
	if let Some((tag, content)) = element_parts(node) {
		return write_element(tag, content, html);
	}
	match node.drop_meta() {
		Node::List(items, _, _) => items.iter().for_each(|item| write_node(item, html)),
		Node::Empty => {}
		other => html.push_str(&escaped(&plain_text(other))),
	}
}

fn write_element(tag: &str, content: &Node, html: &mut String) {
	let items: Vec<&Node> = match content.drop_meta() {
		Node::List(items, Bracket::Curly | Bracket::None, _) => items.iter().collect(),
		Node::Empty => vec![],
		single => vec![single],
	};
	let (attributes, children): (Vec<&Node>, Vec<&Node>) = items.into_iter().partition(|item| attribute_parts(item, tag).is_some());
	html.push('<');
	html.push_str(tag);
	for attribute in attributes {
		let (name, value) = attribute_parts(attribute, tag).expect("partitioned");
		html.push_str(&format!(" {name}=\"{}\"", escaped(&attribute_value(value))));
	}
	html.push('>');
	if VOID_ELEMENTS.contains(&tag) {
		return;
	}
	children.into_iter().for_each(|child| write_node(child, html));
	html.push_str(&format!("</{tag}>"));
}

/// `class:['btn' 'btn-info']` is the attribute `btn btn-info`
fn attribute_value(value: &Node) -> String {
	match value.drop_meta() {
		Node::List(items, _, _) => items.iter().map(plain_text).collect::<Vec<_>>().join(" "),
		other => plain_text(other),
	}
}

/// A value as the text it shows: a text without quotes, anything else as wasp writes it
fn plain_text(node: &Node) -> String {
	match node.drop_meta() {
		Node::Text(text) | Node::Symbol(text) => text.clone(),
		Node::Char(character) => character.to_string(),
		other => other.serialize(),
	}
}

fn escaped(text: &str) -> String {
	text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}
