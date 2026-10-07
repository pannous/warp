//! Markup values (card web-dom, notes/web_framework.md): `div{ class:"box" h1{"Hi"} }` is the element div with the
//! attribute class and the child h1. Which values are markup is decided here; their HTML comes from the one renderer,
//! std/markup.wasp's to_html, also for the CLI and the playground (a page runs it itself as page·html).

use crate::node::{Bracket, Node};
use crate::operators::Op;
use std::collections::HashMap;
use std::sync::OnceLock;

const MARKUP_MODULE: &str = "markup";
/// The list of std/markup.wasp naming the HTML elements a markup key may name (custom elements are not markup yet)
const ELEMENTS_LIST: &str = "html_elements";
/// The value in place of the name `rendered`, assigned first: as an argument `ul: {…}` would name a parameter
const RENDER_PROGRAM: &str = "use markup\nshown = rendered\nto_html(shown)";
const RENDERED: &str = "rendered";
/// the element of a style sheet and the attribute of an inline style (card web-styles)
const STYLE: &str = "style";
/// the attribute naming the component of an element, whose style sheets style only its elements (card web-scoped)
pub const SCOPE_ATTRIBUTE: &str = "data-wasp-scope";

/// Does the word name an HTML element
pub fn is_element_tag(word: &str) -> bool {
	static ELEMENTS: OnceLock<Vec<String>> = OnceLock::new();
	ELEMENTS.get_or_init(|| crate::modules::std_module_list(MARKUP_MODULE, ELEMENTS_LIST)).iter().any(|element| element == word)
}

/// Is the value an element (`div{…}`, `h1: "Hi"`, a key named after an element): what the CLI prints as HTML and the
/// page shows as DOM
pub fn is_markup(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(tag, Op::Colon | Op::None, _) if matches!(tag.drop_meta(), Node::Symbol(tag) if is_element_tag(tag)))
}

/// `style{ ".card": { … } }`: a style sheet among an element's items (not the inline `style: { color: … }`)
pub fn is_style_sheet(node: &Node) -> bool {
	let Node::Key(name, Op::Colon, value) = node.drop_meta() else { return false };
	name.drop_meta().name() == STYLE && !is_declarations(value)
}

/// `{ color: theme padding: 8 }`: CSS properties with plain values
fn is_declarations(value: &Node) -> bool {
	let items = match value.drop_meta() {
		Node::List(items, Bracket::Curly | Bracket::None, _) => items.iter().collect(),
		single => vec![single],
	};
	items.iter().all(|item| matches!(item.drop_meta(), Node::Key(_, Op::Colon, value) if !matches!(value.drop_meta(), Node::Key(..) | Node::List(_, Bracket::Curly, _))))
}

/// The HTML of a markup value, any other value as escaped text, by std/markup.wasp's to_html; a failed rendering is
/// its error, shown, never an empty page
pub fn to_html(node: &Node) -> String {
	let program = crate::law::substitute(&crate::wasp_parser::parse(RENDER_PROGRAM), &HashMap::from([(RENDERED.to_string(), node.clone())]));
	match crate::pipeline::eval_parsed(program, RENDER_PROGRAM).drop_meta() {
		Node::Text(html) => html.clone(),
		failed => format!("<pre>{}</pre>", failed.serialize().replace('&', "&amp;").replace('<', "&lt;")),
	}
}
