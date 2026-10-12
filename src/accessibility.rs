//! Accessible markup (card web-a11y, notes/web_framework.md step 16): welcoming warnings over the markup a program
//! writes, read from the parsed program so they point at the element as written. An image needs alt (alt:"" marks a
//! decoration), a form field a label (label{for:…}, a label around it, aria-label, aria-labelledby or title; a
//! placeholder alone vanishes on typing), a button or link something to say, a link an href, headings no skipped level,
//! ids no duplicates, html a lang.
//! Markup that would render wrong is warned about the same way (card our-html): a key that is no attribute (shown as
//! text), a handler of no event, an input type the browser does not know, content of an element that takes none.
//! Markup built at run time (a computed tag, card web-a11y-runtime) is checked as the run hands it back: its problems
//! that the markup as written did not show are runtime warnings, each once a run.

use crate::diagnostic::Diagnostic;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use std::collections::HashSet;

const IMAGE: &str = "img";
const LINK: &str = "a";
const BUTTON: &str = "button";
const LABEL: &str = "label";
const HTML: &str = "html";
/// a style sheet's items are rules: `html { … }` there styles the element, it makes none
const STYLE: &str = "style";
const FORM_FIELDS: [&str; 3] = ["input", "select", "textarea"];
/// input types that need no label: they say what they do or are not shown
const SELF_LABELED_INPUTS: [&str; 5] = ["hidden", "submit", "button", "reset", "image"];
/// attributes that name an element for a screen reader
const NAMING_ATTRIBUTES: [&str; 3] = ["aria-label", "aria-labelledby", "title"];
const HEADINGS: [&str; 6] = ["h1", "h2", "h3", "h4", "h5", "h6"];
/// `on click { … }` among an element's items is a handler, not content
const HANDLER_WORD: &str = "on";
const RUN_TIME_NOTICE: &str = "markup built at run time";
const INPUT: &str = "input";
const TYPE: &str = "type";
/// the keys of an element warp reads itself before rendering (element_events.rs; transitions.rs has its own) and
/// one lib/markup.warp writes as an attribute besides html_attributes
const WARP_ELEMENT_WORDS: [&str; 2] = ["bind", "key"];
const OWN_ATTRIBUTE_PREFIXES: [&str; 2] = ["data-", "aria-"];
const INPUT_TYPES: [&str; 22] = [
	"button",
	"checkbox",
	"color",
	"date",
	"datetime-local",
	"email",
	"file",
	"hidden",
	"image",
	"month",
	"number",
	"password",
	"radio",
	"range",
	"reset",
	"search",
	"submit",
	"tel",
	"text",
	"time",
	"url",
	"week",
];

thread_local! {
	/// The problems of the markup as written, said where it is written: the markup a run hands back does not repeat them
	static WRITTEN_PROBLEMS: std::cell::RefCell<HashSet<String>> = std::cell::RefCell::new(HashSet::new());
}

/// An element's problem: where, what, and its fix
type Problem<'a> = (&'a Node, String, String);

/// The accessibility warnings of a program's markup
pub fn warnings(program: &Node) -> Vec<Diagnostic> {
	let mut elements = vec![];
	collect(program, false, &mut elements);
	// a program without markup (the renderer, compiled on its own) leaves the written problems of the one it renders
	if elements.is_empty() {
		return vec![];
	}
	let problems = problems(&elements);
	WRITTEN_PROBLEMS
		.with(|written| *written.borrow_mut() = problems.iter().map(|(_, message, _)| message.clone()).collect());
	problems
		.into_iter()
		.map(|(node, message, fix)| Diagnostic::at(node, message).fix(fix))
		.collect()
}

/// The problems of the markup a run hands back that the markup as written did not show, as runtime warnings
pub fn check_rendered(markup: &Node) {
	let markup = computed_tags_named(markup.clone());
	let mut elements = vec![];
	collect(&markup, false, &mut elements);
	for (_, message, fix) in problems(&elements) {
		if !WRITTEN_PROBLEMS.with(|written| written.borrow().contains(&message)) {
			crate::diagnostic::report_runtime_warning_once(&format!("{RUN_TIME_NOTICE}: {message}; fix"), &fix);
		}
	}
}

/// A computed tag comes back from the run as a text, `(tag): {…}` as `"img": {…}`: named like a tag as written
fn computed_tags_named(node: Node) -> Node {
	match node.map_children(computed_tags_named) {
		Node::Key(tag, op, body) => match *tag {
			Node::Text(tag) => Node::Key(Box::new(Node::Symbol(tag)), op, body),
			tag => Node::Key(Box::new(tag), op, body),
		},
		other => other,
	}
}

fn problems<'a>(elements: &[Element<'a>]) -> Vec<Problem<'a>> {
	let labeled_ids: HashSet<String> = elements
		.iter()
		.filter(|element| element.tag == LABEL)
		.filter_map(|label| label.attribute("for"))
		.collect();
	let mut problems: Vec<Problem> = elements
		.iter()
		.filter_map(|element| element_problem(element, &labeled_ids))
		.collect();
	problems.extend(elements.iter().flat_map(markup_mistakes));
	problems.extend(skipped_headings(elements));
	problems.extend(duplicate_ids(elements));
	problems
}

/// An element as written: its tag, attributes with plain values, whether it says anything, whether a label holds it
struct Element<'a> {
	node: &'a Node,
	tag: String,
	attributes: Vec<(String, Option<String>)>,
	has_content: bool,
	inside_label: bool,
	/// the events of its handlers (`on click {…}`)
	events: Vec<String>,
}

impl Element<'_> {
	fn has(&self, name: &str) -> bool {
		self.attributes.iter().any(|(attribute, _)| attribute == name)
	}

	fn attribute(&self, name: &str) -> Option<String> {
		self.attributes
			.iter()
			.find(|(attribute, _)| attribute == name)
			.and_then(|(_, value)| value.clone())
	}

	fn is_named(&self) -> bool {
		NAMING_ATTRIBUTES.iter().any(|name| self.has(name))
	}
}

/// `div{…}`, `h1: "Hi"` or `label(for:pwd): "Password"`: the tag, content and head attributes of an element as written;
/// `a: i32` (a typed parameter or field) and `a: 1` (a number field) are none
fn element_parts(node: &Node) -> Option<(&str, &Node, &[Node])> {
	let Node::Key(tag, Op::Colon, content) = node.drop_meta() else {
		return None;
	};
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
		Node::Symbol(tag) if crate::markup::is_element_tag(tag) => {
			Some((tag.as_str(), content.as_ref(), head_attributes))
		}
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
	let Node::Key(name, Op::Colon, value) = item.drop_meta() else {
		return None;
	};
	let Node::Symbol(name) = name.drop_meta() else {
		return None;
	};
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

/// An element's items without its handlers (`on click {…}`, as a line of its own or as the items `on`, `click: {…}`),
/// the events they handle and their bodies
fn without_handlers(items: Vec<&Node>) -> (Vec<&Node>, Vec<String>, Vec<&Node>) {
	let (mut kept, mut events, mut bodies) = (vec![], vec![], vec![]);
	let mut index = 0;
	while index < items.len() {
		let item = items[index];
		let handler: Vec<&Node> = match item.drop_meta() {
			Node::List(line, _, _) if is_handler(item) => line.iter().skip(1).collect(),
			_ if item.is_symbol(HANDLER_WORD) => match items.get(index + 1).map(|event| event.drop_meta()) {
				Some(Node::Symbol(_)) => items[index + 1..items.len().min(index + 3)].to_vec(),
				Some(Node::Key(_, Op::Colon, _)) => vec![items[index + 1]],
				_ => vec![],
			},
			_ => {
				kept.push(item);
				index += 1;
				continue;
			}
		};
		index += match item.drop_meta() {
			Node::List(..) => 1,
			_ => 1 + handler.len(),
		};
		for part in handler {
			match part.drop_meta() {
				Node::Key(event, Op::Colon, body) => {
					events.extend(event_word(event));
					bodies.push(body.as_ref());
				}
				Node::Symbol(event) => events.push(event.clone()),
				_ => bodies.push(part),
			}
		}
	}
	(kept, events, bodies)
}

fn event_word(event: &Node) -> Option<String> {
	match event.drop_meta() {
		Node::Key(event, Op::Colon, _) => event_word(event),
		Node::Symbol(event) => Some(event.clone()),
		_ => None,
	}
}

/// The markup an element as written would render wrong, each with the near name as its fix
fn markup_mistakes<'a>(element: &Element<'a>) -> Vec<Problem<'a>> {
	let near = |word: &str, names: Vec<String>, otherwise: String| {
		crate::extensions::strings::near_miss(word, names).unwrap_or(otherwise)
	};
	let mut mistakes = vec![];
	// a style sheet's items are rules, no attributes
	if element.tag == STYLE {
		return mistakes;
	}
	for (name, _) in &element.attributes {
		let renders = crate::markup::names_attribute(name)
			|| WARP_ELEMENT_WORDS.contains(&name.as_str())
			|| crate::transitions::ELEMENT_WORDS.contains(&name.as_str())
			|| OWN_ATTRIBUTE_PREFIXES.iter().any(|prefix| name.starts_with(prefix));
		if !renders {
			let fix = near(name, crate::markup::attribute_names().to_vec(), format!("data-{name}"));
			mistakes.push((
				element.node,
				format!("{name} is no attribute of {}: it shows as text", element.tag),
				fix,
			));
		}
	}
	let events = || {
		crate::event_signals::PAGE_EVENTS
			.iter()
			.map(|event| event.to_string())
			.collect::<Vec<_>>()
	};
	for event in element
		.events
		.iter()
		.filter(|event| !crate::event_signals::PAGE_EVENTS.contains(&event.as_str()))
	{
		let message = format!("{event} is no event of {}: the handler shows as text", element.tag);
		mistakes.push((
			element.node,
			message,
			near(event, events(), format!("one of {}", events().join(", "))),
		));
	}
	if element.tag == INPUT {
		if let Some(kind) = element
			.attribute(TYPE)
			.filter(|kind| !INPUT_TYPES.contains(&kind.as_str()))
		{
			let fix = near(
				&kind,
				INPUT_TYPES.iter().map(|kind| kind.to_string()).collect(),
				"text".to_string(),
			);
			mistakes.push((
				element.node,
				format!("{kind} is no input type: the browser shows a text field"),
				fix,
			));
		}
	}
	if element.has_content && crate::markup::is_void_element(&element.tag) {
		let fix = format!("the content beside {}, or an element that holds it", element.tag);
		mistakes.push((
			element.node,
			format!("{} takes no content: it is dropped", element.tag),
			fix,
		));
	}
	mistakes
}

fn is_handler(item: &Node) -> bool {
	matches!(item.drop_meta(), Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if word == HANDLER_WORD))
}

/// Does an item say something: a text, a value computed at run time, or an element that does
fn says_something(item: &Node) -> bool {
	match element_parts(item) {
		Some((_, content, _)) => content_items(content)
			.into_iter()
			.any(|child| attribute_parts(child).is_none() && !is_handler(child) && says_something(child)),
		None => {
			!matches!(item.drop_meta(), Node::Empty)
				&& attribute_parts(item).is_none()
				&& !is_handler(item)
				&& !matches!(item.drop_meta(), Node::Text(text) if text.trim().is_empty())
		}
	}
}

fn collect<'a>(node: &'a Node, inside_label: bool, found: &mut Vec<Element<'a>>) {
	if let Some((tag, content, head_attributes)) = element_parts(node) {
		let (items, events, handler_bodies) = without_handlers(content_items(content));
		found.push(Element {
			node,
			tag: tag.to_string(),
			attributes: head_attributes
				.iter()
				.chain(items.iter().copied())
				.filter_map(attribute_parts)
				.collect(),
			has_content: items
				.iter()
				.any(|item| attribute_parts(item).is_none() && says_something(item)),
			inside_label,
			events,
		});
		if tag == STYLE {
			return;
		}
		let inside_label = inside_label || tag == LABEL;
		return items
			.into_iter()
			.chain(handler_bodies)
			.for_each(|item| collect(item, inside_label, found));
	}
	match node.drop_meta() {
		Node::List(items, _, _) => items.iter().for_each(|item| collect(item, inside_label, found)),
		// the head of `add(a: i32) := …` holds parameters, the target of `a:V = …` a typed variable: not markup
		Node::Key(_, Op::Define | Op::Assign, body) => collect(body, inside_label, found),
		// `c ? then : else`: the colon parts the branches, it makes no element
		Node::Key(condition, Op::Question, branches)
			if let Node::Key(then, Op::Colon, otherwise) = branches.drop_meta() =>
		{
			[condition, then, otherwise]
				.into_iter()
				.for_each(|part| collect(part, inside_label, found));
		}
		Node::Key(left, _, right) => {
			collect(left, inside_label, found);
			collect(right, inside_label, found);
		}
		_ => {}
	}
}

fn element_problem<'a>(element: &Element<'a>, labeled_ids: &HashSet<String>) -> Option<Problem<'a>> {
	let warning = |message: &str, fix: &str| Some((element.node, message.to_string(), fix.to_string()));
	match element.tag.as_str() {
		IMAGE if !element.has("alt") => warning(
			"img without alt: a screen reader reads its file name",
			"alt: \"what it shows\", or alt: \"\" for a decoration",
		),
		tag if FORM_FIELDS.contains(&tag) && needs_label(element, labeled_ids) => match element.has("placeholder") {
			true => warning(
				&format!("{tag} labeled only by its placeholder, which vanishes on typing"),
				"a label{ for: \"id\" … } beside it, or aria-label: \"…\"",
			),
			false => warning(
				&format!("{tag} without a label: a screen reader cannot say what it asks for"),
				"label{ \"Name\" input{…} } around it, or aria-label: \"…\"",
			),
		},
		BUTTON if !element.has_content && !element.is_named() => warning(
			"button without text: a screen reader announces just \"button\"",
			"a text inside it, or aria-label: \"…\"",
		),
		LINK if !element.has("href") => warning(
			"a without href: not reachable by keyboard",
			"href: \"…\", or a button for an action",
		),
		LINK if !element.has_content && !element.is_named() => warning(
			"a without text: a screen reader reads its address",
			"a text inside it, or aria-label: \"…\"",
		),
		HTML if !element.has("lang") => warning("html without lang: screen readers guess the language", "lang: \"en\""),
		_ => None,
	}
}

fn needs_label(field: &Element, labeled_ids: &HashSet<String>) -> bool {
	let self_labeled = field
		.attribute("type")
		.is_some_and(|kind| SELF_LABELED_INPUTS.contains(&kind.as_str()));
	let labeled_by_id = field.attribute("id").is_some_and(|id| labeled_ids.contains(&id));
	!(self_labeled || labeled_by_id || field.inside_label || field.is_named())
}

/// `h1 … h3`: a heading more than one level below the one before it
fn skipped_headings<'a>(elements: &[Element<'a>]) -> Vec<Problem<'a>> {
	let mut previous = 0;
	let mut warnings = vec![];
	for element in elements {
		let Some(level) = HEADINGS
			.iter()
			.position(|heading| *heading == element.tag)
			.map(|index| index + 1)
		else {
			continue;
		};
		if previous > 0 && level > previous + 1 {
			warnings.push((
				element.node,
				format!("{} after h{previous} skips a heading level", element.tag),
				format!("h{}", previous + 1),
			));
		}
		previous = level;
	}
	warnings
}

fn duplicate_ids<'a>(elements: &[Element<'a>]) -> Vec<Problem<'a>> {
	let mut seen = HashSet::new();
	elements
		.iter()
		.filter_map(|element| element.attribute("id").map(|id| (element, id)))
		.filter(|(_, id)| !seen.insert(id.clone()))
		.map(|(element, id)| {
			(
				element.node,
				format!("id \"{id}\" is used twice: labels and links find only the first"),
				"a different id".to_string(),
			)
		})
		.collect()
}
