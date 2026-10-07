//! Accessible markup (card web-a11y, notes/web_framework.md step 16): welcoming warnings over the markup a program
//! writes, read from the parsed program so they point at the element as written. An image needs alt (alt:"" marks a
//! decoration), a form field a label (label{for:…}, a label around it, aria-label, aria-labelledby or title; a
//! placeholder alone vanishes on typing), a button or link something to say, a link an href, headings no skipped level,
//! ids no duplicates, html a lang.

use crate::diagnostic::Diagnostic;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use std::collections::HashSet;

const IMAGE: &str = "img";
const LINK: &str = "a";
const BUTTON: &str = "button";
const LABEL: &str = "label";
const HTML: &str = "html";
const FORM_FIELDS: [&str; 3] = ["input", "select", "textarea"];
/// input types that need no label: they say what they do or are not shown
const SELF_LABELED_INPUTS: [&str; 5] = ["hidden", "submit", "button", "reset", "image"];
/// attributes that name an element for a screen reader
const NAMING_ATTRIBUTES: [&str; 3] = ["aria-label", "aria-labelledby", "title"];
const HEADINGS: [&str; 6] = ["h1", "h2", "h3", "h4", "h5", "h6"];
/// `on click { … }` among an element's items is a handler, not content
const HANDLER_WORD: &str = "on";

/// The accessibility warnings of a program's markup
pub fn warnings(program: &Node) -> Vec<Diagnostic> {
	let mut elements = vec![];
	collect(program, false, &mut elements);
	if elements.is_empty() {
		return vec![];
	}
	let labeled_ids: HashSet<String> = elements.iter().filter(|element| element.tag == LABEL).filter_map(|label| label.attribute("for")).collect();
	let mut warnings: Vec<Diagnostic> = elements.iter().filter_map(|element| element_warning(element, &labeled_ids)).collect();
	warnings.extend(skipped_headings(&elements));
	warnings.extend(duplicate_ids(&elements));
	warnings
}

/// An element as written: its tag, attributes with plain values, whether it says anything, whether a label holds it
struct Element<'a> {
	node: &'a Node,
	tag: String,
	attributes: Vec<(String, Option<String>)>,
	has_content: bool,
	inside_label: bool,
}

impl Element<'_> {
	fn has(&self, name: &str) -> bool {
		self.attributes.iter().any(|(attribute, _)| attribute == name)
	}

	fn attribute(&self, name: &str) -> Option<String> {
		self.attributes.iter().find(|(attribute, _)| attribute == name).and_then(|(_, value)| value.clone())
	}

	fn is_named(&self) -> bool {
		NAMING_ATTRIBUTES.iter().any(|name| self.has(name))
	}
}

/// `div{…}`, `h1: "Hi"` or `label(for:pwd): "Password"`: the tag, content and head attributes of an element as written;
/// `a: i32` (a typed parameter or field) and `a: 1` (a number field) are none
fn element_parts(node: &Node) -> Option<(&str, &Node, &[Node])> {
	let Node::Key(tag, Op::Colon, content) = node.drop_meta() else { return None };
	match content.drop_meta() {
		Node::Type { .. } | Node::Number(_) => return None,
		Node::Symbol(word) if crate::analyzer::type_word_kind(word).is_some() => return None,
		_ => {}
	}
	let (tag, head_attributes) = match tag.drop_meta() {
		Node::List(items, _, _) => (items.first()?, &items[1..]),
		tag => (tag, &[][..]),
	};
	match tag.drop_meta() {
		Node::Symbol(tag) if crate::markup::is_element_tag(tag) => Some((tag.as_str(), content.as_ref(), head_attributes)),
		_ => None,
	}
}

fn content_items(content: &Node) -> Vec<&Node> {
	match content.drop_meta() {
		Node::List(items, Bracket::Curly, _) => items.iter().collect(),
		Node::Empty => vec![],
		single => vec![single],
	}
}

/// `alt: "x"`: an attribute, with its value when that is a text (a one-letter text is a Char), number or name
fn attribute_parts(item: &Node) -> Option<(String, Option<String>)> {
	let Node::Key(name, Op::Colon, value) = item.drop_meta() else { return None };
	let Node::Symbol(name) = name.drop_meta() else { return None };
	if crate::markup::is_element_tag(name) {
		return None;
	}
	let value = match value.drop_meta() {
		Node::Text(text) | Node::Symbol(text) => Some(text.clone()),
		Node::Number(number) => Some(number.to_string()),
		Node::Char(character) => Some(character.to_string()),
		_ => None,
	};
	Some((name.clone(), value))
}

fn is_handler(item: &Node) -> bool {
	matches!(item.drop_meta(), Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if word == HANDLER_WORD))
}

/// Does an item say something: a text, a value computed at run time, or an element that does
fn says_something(item: &Node) -> bool {
	match element_parts(item) {
		Some((_, content, _)) => content_items(content).into_iter().any(|child| attribute_parts(child).is_none() && !is_handler(child) && says_something(child)),
		None => !matches!(item.drop_meta(), Node::Empty) && attribute_parts(item).is_none() && !is_handler(item) && !matches!(item.drop_meta(), Node::Text(text) if text.trim().is_empty()),
	}
}

fn collect<'a>(node: &'a Node, inside_label: bool, found: &mut Vec<Element<'a>>) {
	if let Some((tag, content, head_attributes)) = element_parts(node) {
		let items = content_items(content);
		found.push(Element {
			node,
			tag: tag.to_string(),
			attributes: head_attributes.iter().chain(items.iter().copied()).filter_map(attribute_parts).collect(),
			has_content: items.iter().any(|item| attribute_parts(item).is_none() && !is_handler(item) && says_something(item)),
			inside_label,
		});
		let inside_label = inside_label || tag == LABEL;
		return items.into_iter().for_each(|item| collect(item, inside_label, found));
	}
	match node.drop_meta() {
		Node::List(items, _, _) => items.iter().for_each(|item| collect(item, inside_label, found)),
		// the head of `add(a: i32) := …` holds parameters, not markup
		Node::Key(_, Op::Define, body) => collect(body, inside_label, found),
		// `c ? then : else`: the colon parts the branches, it makes no element
		Node::Key(condition, Op::Question, branches) if let Node::Key(then, Op::Colon, otherwise) = branches.drop_meta() => {
			[condition, then, otherwise].into_iter().for_each(|part| collect(part, inside_label, found));
		}
		Node::Key(left, _, right) => {
			collect(left, inside_label, found);
			collect(right, inside_label, found);
		}
		_ => {}
	}
}

fn element_warning(element: &Element, labeled_ids: &HashSet<String>) -> Option<Diagnostic> {
	let warning = |message: &str, fix: &str| Some(Diagnostic::at(element.node, message).fix(fix));
	match element.tag.as_str() {
		IMAGE if !element.has("alt") => warning("img without alt: a screen reader reads its file name", "alt: \"what it shows\", or alt: \"\" for a decoration"),
		tag if FORM_FIELDS.contains(&tag) && needs_label(element, labeled_ids) => match element.has("placeholder") {
			true => warning(&format!("{tag} labeled only by its placeholder, which vanishes on typing"), "a label{ for: \"id\" … } beside it, or aria-label: \"…\""),
			false => warning(&format!("{tag} without a label: a screen reader cannot say what it asks for"), "label{ \"Name\" input{…} } around it, or aria-label: \"…\""),
		},
		BUTTON if !element.has_content && !element.is_named() => warning("button without text: a screen reader announces just \"button\"", "a text inside it, or aria-label: \"…\""),
		LINK if !element.has("href") => warning("a without href: not reachable by keyboard", "href: \"…\", or a button for an action"),
		LINK if !element.has_content && !element.is_named() => warning("a without text: a screen reader reads its address", "a text inside it, or aria-label: \"…\""),
		HTML if !element.has("lang") => warning("html without lang: screen readers guess the language", "lang: \"en\""),
		_ => None,
	}
}

fn needs_label(field: &Element, labeled_ids: &HashSet<String>) -> bool {
	let self_labeled = field.attribute("type").is_some_and(|kind| SELF_LABELED_INPUTS.contains(&kind.as_str()));
	let labeled_by_id = field.attribute("id").is_some_and(|id| labeled_ids.contains(&id));
	!(self_labeled || labeled_by_id || field.inside_label || field.is_named())
}

/// `h1 … h3`: a heading more than one level below the one before it
fn skipped_headings(elements: &[Element]) -> Vec<Diagnostic> {
	let mut previous = 0;
	let mut warnings = vec![];
	for element in elements {
		let Some(level) = HEADINGS.iter().position(|heading| *heading == element.tag).map(|index| index + 1) else { continue };
		if previous > 0 && level > previous + 1 {
			warnings.push(Diagnostic::at(element.node, format!("{} after h{previous} skips a heading level", element.tag)).fix(format!("h{}", previous + 1)));
		}
		previous = level;
	}
	warnings
}

fn duplicate_ids(elements: &[Element]) -> Vec<Diagnostic> {
	let mut seen = HashSet::new();
	elements.iter().filter_map(|element| element.attribute("id").map(|id| (element, id)))
		.filter(|(_, id)| !seen.insert(id.clone()))
		.map(|(element, id)| Diagnostic::at(element.node, format!("id \"{id}\" is used twice: labels and links find only the first")).fix("a different id"))
		.collect()
}
