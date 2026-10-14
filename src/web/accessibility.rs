//! Accessible markup (card web-a11y, notes/web_framework.md step 16): welcoming warnings over the markup a program
//! writes, read from the parsed program so they point at the element as written. An image needs alt (alt:"" marks a
//! decoration), a form field a label (label{for:…}, a label around it, aria-label, aria-labelledby or title; a
//! placeholder alone vanishes on typing), a button or link something to say, a link an href, headings no skipped level,
//! ids no duplicates, html a lang.
//! Markup that would render wrong is warned about the same way (card our-html): a key that is no attribute (shown as
//! text), a handler of no event, an input type the browser does not know, content of an element that takes none.
//! Markup built at run time (a computed tag, card web-a11y-runtime) is checked as the run hands it back: its problems
//! that the markup as written did not show are runtime warnings, each once a run.

use crate::errors::diagnostic::Diagnostic;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use crate::words::STYLE_WORD;
use std::collections::HashSet;

const IMAGE: &str = "img";
const LINK: &str = "a";
const BUTTON: &str = "button";
const LABEL: &str = "label";
const HTML: &str = "html";
const META: &str = "meta";
/// Keys of html that are facts for the page's head, besides `meta: {…}` (decision html-meta, lib/markup.warp html_is_page_fact)
const PAGE_FACTS: [&str; 2] = ["title", "description"];
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
/// attributes whose value is a number (`rows: 3`, `cols: "40"` as a text of one), or a date or time (`min: "2024-01-01"`),
/// or the attribute's keyword (NUMBER_KEYWORDS)
const NUMBER_ATTRIBUTES: [&str; 10] =
	["min", "max", "step", "rows", "cols", "colspan", "rowspan", "tabindex", "width", "height"];
/// the words a number attribute takes besides numbers: `step: "any"`
const NUMBER_KEYWORDS: [(&str, &str); 1] = [("step", crate::element_events::ANY_STEP)];
/// attributes whose value is an address
const ADDRESS_ATTRIBUTES: [&str; 3] = ["href", "src", "action"];
/// elements a p cannot hold: the browser ends the p before them
const BLOCK_ELEMENTS: [&str; 27] = [
	"address",
	"article",
	"aside",
	"blockquote",
	"details",
	"div",
	"dl",
	"fieldset",
	"figure",
	"footer",
	"form",
	"h1",
	"h2",
	"h3",
	"h4",
	"h5",
	"h6",
	"header",
	"hr",
	"main",
	"nav",
	"ol",
	"p",
	"pre",
	"section",
	"table",
	"ul",
];
const LISTS: [&str; 3] = ["ul", "ol", "menu"];
const LIST_ITEMS: [&str; 3] = ["li", "script", "template"];
const CELLS: [&str; 2] = ["td", "th"];
const ROW: &str = "tr";
const TABLE: &str = "table";
const TABLE_PARTS: [&str; 5] = ["thead", "tbody", "tfoot", "caption", "colgroup"];
const ROW_HOLDERS: [&str; 4] = ["table", "thead", "tbody", "tfoot"];
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

/// An element's problem: where, what, and its fix
type Problem<'a> = (&'a Node, String, String);

/// The accessibility warnings of a program's markup
pub fn warnings(program: &Node) -> Vec<Diagnostic> {
	let mut elements = vec![];
	collect(program, &class_names(program), false, None, &mut elements);
	// a program without markup (the renderer, compiled on its own) leaves the written problems of the one it renders
	if elements.is_empty() {
		return vec![];
	}
	let defined: HashSet<String> = crate::analyzer::function_definitions(program).user_functions.into_keys().collect();
	let problems = problems(&elements, &defined);
	// `divv{ "hi" }` comes back from the run as the attribute `divv: "hi"`: said once, as the unknown tag it is
	let rendered_as_attributes = elements.iter().flat_map(|element| {
		element.blocks.iter().filter(|tag| is_unknown_tag(tag)).map(|tag| no_attribute(tag, &element.tag))
	});
	let written: HashSet<String> =
		problems.iter().map(|(_, message, _)| message.clone()).chain(rendered_as_attributes).collect();
	crate::compiler::compile_context::change(|context| context.written_problems = written);
	problems.into_iter().map(|(node, message, fix)| Diagnostic::at(node, message).fix(fix)).collect()
}

/// The problems of the markup a run hands back that the markup as written did not show, as runtime warnings
pub fn check_rendered(markup: &Node) {
	let markup = computed_tags_named(markup.clone());
	let mut elements = vec![];
	collect(&markup, &HashSet::new(), false, None, &mut elements);
	for (_, message, fix) in problems(&elements, &HashSet::new()) {
		if !crate::compiler::compile_context::read(|context| context.written_problems.contains(&message)) {
			crate::errors::diagnostic::report_runtime_warning_once(&format!("{RUN_TIME_NOTICE}: {message}; fix"), &fix);
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

/// `defined`: the program's own names, whose `card{…}` is a component, no unknown tag
fn problems<'a>(elements: &[Element<'a>], defined: &HashSet<String>) -> Vec<Problem<'a>> {
	let labeled_ids: HashSet<String> =
		elements.iter().filter(|element| element.tag == LABEL).filter_map(|label| label.attribute("for")).collect();
	let mut problems: Vec<Problem> =
		elements.iter().filter_map(|element| element_problem(element, &labeled_ids)).collect();
	problems.extend(elements.iter().flat_map(|element| markup_mistakes(element, defined)));
	problems.extend(elements.iter().flat_map(value_mistakes));
	problems.extend(elements.iter().filter_map(nesting_mistake));
	problems.extend(skipped_headings(elements));
	problems.extend(duplicate_ids(elements));
	problems
}

/// An element as written: its tag, attributes with their values as written, whether it says anything, whether a label holds it
struct Element<'a> {
	node: &'a Node,
	tag: String,
	attributes: Vec<(String, &'a Node)>,
	has_content: bool,
	inside_label: bool,
	/// the element it is written in, none for markup on its own (a component's)
	parent: Option<String>,
	/// the events of its handlers (`on click {…}`)
	events: Vec<String>,
	/// the names of its items written as blocks (`divv{ "hi" }`, `my-card{…}`), no attributes
	blocks: Vec<String>,
}

impl Element<'_> {
	fn has(&self, name: &str) -> bool {
		self.attributes.iter().any(|(attribute, _)| attribute == name)
	}

	fn attribute(&self, name: &str) -> Option<String> {
		self.attributes.iter().find(|(attribute, _)| attribute == name).and_then(|(_, value)| plain_value(value))
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
		Node::Symbol(tag) if crate::web::markup::is_element_tag(tag) => {
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

/// `alt: "x"`: an attribute and its value as written
fn written_attribute(item: &Node) -> Option<(String, &Node)> {
	let Node::Key(name, Op::Colon, value) = item.drop_meta() else {
		return None;
	};
	let Node::Symbol(name) = name.drop_meta() else {
		return None;
	};
	if crate::web::markup::is_element_tag(name) || crate::web::markup::is_custom_element(name) {
		return None;
	}
	Some((name.clone(), value.drop_meta()))
}

/// An attribute's value when that is a text (a one-letter text is a Char), number or name
fn plain_value(value: &Node) -> Option<String> {
	match value {
		Node::Text(text) | Node::Symbol(text) => Some(text.clone()),
		Node::Number(number) => Some(number.to_string()),
		Node::Char(character) => Some(character.to_string()),
		_ => None,
	}
}

/// The attribute values of the wrong kind (card attribute-values): a text where yes or no belongs, a word where a
/// number does, a number for an address
fn value_mistakes<'a>(element: &Element<'a>) -> Vec<Problem<'a>> {
	let text_of = |value: &Node| match value {
		Node::Text(text) => Some(text.clone()),
		Node::Char(character) => Some(character.to_string()),
		_ => None,
	};
	element
		.attributes
		.iter()
		.filter_map(|(name, value)| {
			let mistake = |message: String, fix: String| Some((element.node, message, fix));
			match (text_of(value), value) {
				(Some(text), _) if crate::web::markup::is_boolean_attribute(name) => mistake(
					format!("{name} takes yes or no: the text {text:?} leaves it out"),
					format!("{name}: yes, or {name}: no"),
				),
				(Some(text), _) if NUMBER_ATTRIBUTES.contains(&name.as_str()) && !is_number_value(name, &text) => {
					mistake(format!("{name} takes a number, not {text:?}"), format!("{name}: 3"))
				}
				(None, Node::Number(number)) if ADDRESS_ATTRIBUTES.contains(&name.as_str()) => {
					mistake(format!("{name} takes an address, not the number {number}"), format!("{name}: \"/page\""))
				}
				_ => None,
			}
		})
		.collect()
}

/// A number attribute's value: written with digits (a number, date or time) or the attribute's keyword (`step: "any"`)
fn is_number_value(attribute: &str, value: &str) -> bool {
	value.chars().any(|character| character.is_ascii_digit()) || NUMBER_KEYWORDS.contains(&(attribute, value.trim()))
}

/// An element where HTML does not let it be, which the browser moves or drops (card markup-content)
fn nesting_mistake<'a>(element: &Element<'a>) -> Option<Problem<'a>> {
	let parent = element.parent.as_deref()?;
	let tag = element.tag.as_str();
	let mistake = |reason: &str, fix: String| Some((element.node, format!("{tag} inside {parent}: {reason}"), fix));
	match tag {
		HTML => mistake("html is the page itself", format!("drop html{{ … }} around the {parent}'s content")),
		_ if parent == "p" && BLOCK_ELEMENTS.contains(&tag) => {
			mistake("the browser ends the p before it", "make the p a div".into())
		}
		_ if LISTS.contains(&parent) && !LIST_ITEMS.contains(&tag) => {
			mistake("a list holds li items", format!("li{{ {tag}{{ … }} }}"))
		}
		_ if CELLS.contains(&tag) && parent != ROW => mistake("it belongs in a tr", format!("tr{{ {tag}{{ … }} }}")),
		ROW if !ROW_HOLDERS.contains(&parent) => mistake("it belongs in a table", "table{ tr{ … } }".into()),
		_ if TABLE_PARTS.contains(&tag) && parent != TABLE => {
			mistake("it belongs in a table", format!("table{{ {tag}{{ … }} }}"))
		}
		_ => None,
	}
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
fn markup_mistakes<'a>(element: &Element<'a>, defined: &HashSet<String>) -> Vec<Problem<'a>> {
	let near = |word: &str, names: Vec<String>, otherwise: String| {
		crate::extensions::strings::near_miss(word, names).unwrap_or(otherwise)
	};
	let mut mistakes = vec![];
	// a style sheet's items are rules, no attributes; `meta: {author: "Ann"}` of html names metas (page facts)
	if element.tag == STYLE_WORD || names_metas(element) {
		return mistakes;
	}
	for tag in element.blocks.iter().filter(|tag| is_unknown_tag(tag) && !defined.contains(*tag)) {
		let fix = near(tag, crate::web::markup::element_names().to_vec(), "div".to_string());
		mistakes.push((element.node, format!("{tag} is no element: it shows as text"), fix));
	}
	for (name, _) in element
		.attributes
		.iter()
		.filter(|(name, _)| !element.blocks.contains(name))
		.filter(|(name, _)| !(element.tag == HTML && PAGE_FACTS.contains(&name.as_str())))
	{
		let renders = crate::web::markup::names_attribute(name)
			|| WARP_ELEMENT_WORDS.contains(&name.as_str())
			|| crate::transitions::ELEMENT_WORDS.contains(&name.as_str())
			|| OWN_ATTRIBUTE_PREFIXES.iter().any(|prefix| name.starts_with(prefix));
		if !renders {
			let fix = near(name, crate::web::markup::attribute_names().to_vec(), format!("data-{name}"));
			mistakes.push((element.node, no_attribute(name, &element.tag), fix));
		}
	}
	let events = || crate::event_signals::element_events().map(|event| event.to_string()).collect::<Vec<_>>();
	for event in element.events.iter().filter(|event| !crate::event_signals::is_element_event(event)) {
		let message = format!("{event} is no event of {}: the handler shows as text", element.tag);
		mistakes.push((element.node, message, near(event, events(), format!("one of {}", events().join(", ")))));
	}
	if element.tag == INPUT {
		if let Some(kind) = element.attribute(TYPE).filter(|kind| !INPUT_TYPES.contains(&kind.as_str())) {
			let fix = near(&kind, INPUT_TYPES.iter().map(|kind| kind.to_string()).collect(), "text".to_string());
			mistakes.push((element.node, format!("{kind} is no input type: the browser shows a text field"), fix));
		}
	}
	if element.has_content && crate::web::markup::is_void_element(&element.tag) {
		let fix = format!("the content beside {}, or an element that holds it", element.tag);
		mistakes.push((element.node, format!("{} takes no content: it is dropped", element.tag), fix));
	}
	mistakes
}

/// `divv{ "hi" }`: the name of an item written as a block
fn block_name(item: &Node) -> Option<String> {
	match item.drop_meta() {
		Node::Key(name, Op::Colon | Op::None, content) => match (name.drop_meta(), content.drop_meta()) {
			(Node::Symbol(name), Node::List(_, Bracket::Curly, _)) => Some(name.clone()),
			_ => None,
		},
		_ => None,
	}
}

/// `meta: {author: "Ann"}` of html: page facts, no meta element of its own attributes (`charset`, `name`)
fn names_metas(element: &Element) -> bool {
	let meta_attributes = crate::web::markup::meta_attribute_names();
	element.tag == META
		&& element.parent.as_deref() == Some(HTML)
		&& !element.attributes.iter().any(|(name, _)| meta_attributes.contains(name))
}

/// A block's name no element, attribute nor warp word: `divv` of `divv{ "hi" }`
fn is_unknown_tag(tag: &str) -> bool {
	let known = crate::web::markup::is_element_tag(tag)
		|| crate::web::markup::names_attribute(tag)
		|| WARP_ELEMENT_WORDS.contains(&tag)
		|| crate::transitions::ELEMENT_WORDS.contains(&tag)
		|| crate::event_signals::is_element_event(tag)
		// a custom element of the page's own (`my-card{…}`) is named with a dash
		|| tag.contains('-');
	!known
}

fn no_attribute(name: &str, tag: &str) -> String {
	format!("{name} is no attribute of {tag}: it shows as text")
}

fn is_handler(item: &Node) -> bool {
	matches!(item.drop_meta(), Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if word == HANDLER_WORD))
}

/// Does an item say something: a text, a value computed at run time, or an element that does
fn says_something(item: &Node) -> bool {
	match element_parts(item) {
		Some((_, content, _)) => content_items(content)
			.into_iter()
			.any(|child| written_attribute(child).is_none() && !is_handler(child) && says_something(child)),
		None => {
			!matches!(item.drop_meta(), Node::Empty)
				&& written_attribute(item).is_none()
				&& !is_handler(item)
				&& !matches!(item.drop_meta(), Node::Text(text) if text.trim().is_empty())
		}
	}
}

/// `classes`: the program's class names, whose literal `Parser{source: "ab"}` holds fields, no elements
fn collect<'a>(
	node: &'a Node,
	classes: &HashSet<String>,
	inside_label: bool,
	parent: Option<&str>,
	found: &mut Vec<Element<'a>>,
) {
	if let Some(fields) = class_literal_fields(node, classes).or_else(|| data_map_fields(node, parent)) {
		return fields
			.iter()
			.map(|field| match field.drop_meta() {
				Node::Key(_, Op::Colon, value) => value.as_ref(),
				positional => positional,
			})
			.for_each(|value| collect(value, classes, inside_label, parent, found));
	}
	if let Some((tag, content, head_attributes)) = element_parts(node) {
		let (items, events, handler_bodies) = without_handlers(content_items(content));
		found.push(Element {
			node,
			tag: tag.to_string(),
			attributes: head_attributes.iter().chain(items.iter().copied()).filter_map(written_attribute).collect(),
			has_content: items.iter().any(|item| written_attribute(item).is_none() && says_something(item)),
			inside_label,
			parent: parent.map(str::to_string),
			events,
			blocks: items.iter().filter_map(|item| block_name(item)).collect(),
		});
		if tag == STYLE_WORD {
			return;
		}
		let inside_label = inside_label || tag == LABEL;
		items.into_iter().for_each(|item| collect(item, classes, inside_label, Some(tag), found));
		return handler_bodies.into_iter().for_each(|body| collect(body, classes, inside_label, None, found));
	}
	match node.drop_meta() {
		Node::List(items, _, _) => items.iter().for_each(|item| collect(item, classes, inside_label, parent, found)),
		// the head of `add(a: i32) := …` holds parameters, the target of `a:V = …` a typed variable: not markup
		Node::Key(_, Op::Define | Op::Assign, body) => collect(body, classes, inside_label, parent, found),
		// `c ? then : else`: the colon parts the branches, it makes no element
		Node::Key(condition, Op::Question, branches)
			if let Node::Key(then, Op::Colon, otherwise) = branches.drop_meta() =>
		{
			[condition, then, otherwise]
				.into_iter()
				.for_each(|part| collect(part, classes, inside_label, parent, found));
		}
		// `{a: 2} is {a: h}`: compared values, or a pattern, are never rendered, so they hold no elements
		Node::Key(_, op, _) if op.is_relation() => {}
		Node::Key(left, _, right) => {
			collect(left, classes, inside_label, parent, found);
			collect(right, classes, inside_label, parent, found);
		}
		_ => {}
	}
}

/// The names of the classes a program declares
fn class_names(program: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	program.visit(&mut |node| {
		if let Node::Type { name, .. } = node {
			names.insert(name.drop_meta().name());
		}
	});
	names
}

/// `Parser{source: "ab"}` of a class `Parser`: its fields
fn class_literal_fields<'a>(node: &'a Node, classes: &HashSet<String>) -> Option<&'a [Node]> {
	let (class, fields) = match node.drop_meta() {
		Node::Key(class, _, fields) => (class.as_ref(), fields.as_ref()),
		Node::List(items, _, _) if let [class, fields] = items.as_slice() => (class, fields),
		_ => return None,
	};
	match (class.drop_meta(), fields.drop_meta()) {
		(Node::Symbol(class), Node::List(fields, Bracket::Curly, _)) if classes.contains(class) => Some(fields),
		_ => None,
	}
}

/// `{status: 200, body: {…}}` outside markup: a map with a key named like no element, at any depth of its maps
/// (`{body: {user: {age}}}`), is data, its keys no elements (cards data-map, body-key-markup)
fn data_map_fields<'a>(node: &'a Node, parent: Option<&str>) -> Option<&'a [Node]> {
	let fields = map_fields(node)?;
	(parent.is_none() && names_no_element(fields)).then_some(fields)
}

/// The fields of `{a: 1, b: 2}`: a curly list of colon keys only
fn map_fields(node: &Node) -> Option<&[Node]> {
	match node.drop_meta() {
		Node::List(fields, Bracket::Curly, _)
			if fields.iter().all(|field| matches!(field.drop_meta(), Node::Key(_, Op::Colon, _))) =>
		{
			Some(fields)
		}
		_ => None,
	}
}

fn names_no_element(fields: &[Node]) -> bool {
	fields.iter().any(|field| match field.drop_meta() {
		Node::Key(name, _, value) => {
			name.symbol_name().is_some_and(|name| !crate::web::markup::is_element_tag(name))
				|| map_fields(value).is_some_and(names_no_element)
		}
		_ => false,
	})
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
		LINK if !element.has("href") => {
			warning("a without href: not reachable by keyboard", "href: \"…\", or a button for an action")
		}
		LINK if !element.has_content && !element.is_named() => {
			warning("a without text: a screen reader reads its address", "a text inside it, or aria-label: \"…\"")
		}
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
fn skipped_headings<'a>(elements: &[Element<'a>]) -> Vec<Problem<'a>> {
	let mut previous = 0;
	let mut warnings = vec![];
	for element in elements {
		let Some(level) = HEADINGS.iter().position(|heading| *heading == element.tag).map(|index| index + 1) else {
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
