//! Transitions declared as data (card web-transitions, notes/web_framework.md step 15): in an element's block,
//! `transition: fade 200ms` is the attribute `data-wasp-transition: "fade 200ms"`, by which the page animates the
//! element in when it appears, out when it goes and to its new place in a keyed list (markup.js). The words are data, not
//! variables: names (fade, slide, scale, an easing like ease-out) and durations, normalized to milliseconds; a text is
//! taken as written. The words end where the element's children begin (`p{ transition: fade 200ms "text" }`).

use crate::element_events::element_items;
use crate::node::{Bracket, Node};
use crate::operators::Op;

const TRANSITION: &str = "transition";
const STYLE: &str = "style";
pub const TRANSITION_ATTRIBUTE: &str = "data-wasp-transition";

pub fn lower(node: Node) -> Node {
	let node = node.map_children(lower);
	// `style: { transition: "opacity 1s" }` is an inline style, its transition the CSS property
	if element_items(&node).is_none() || node.drop_meta().name() == STYLE {
		return node;
	}
	let Node::Key(tag, op, content) = node else { unreachable!("an element") };
	let Node::List(items, bracket, separator) = content.drop_meta().clone() else { unreachable!("an element's block") };
	Node::Key(tag, op, Box::new(Node::List(items.into_iter().flat_map(with_transition_attribute).collect(), bracket, separator)))
}

/// An element's item: `transition: …` (alone, or leading a line of words) as the attribute and the children after it
fn with_transition_attribute(item: Node) -> Vec<Node> {
	let (value, line_rest) = match item.drop_meta() {
		Node::Key(name, Op::Colon, value) if is_transition(name) => (value.as_ref().clone(), vec![]),
		Node::List(items, Bracket::None, _) => match items.split_first().map(|(first, rest)| (first.drop_meta(), rest)) {
			Some((Node::Key(name, Op::Colon, value), rest)) if is_transition(name) => (value.as_ref().clone(), rest.to_vec()),
			_ => return vec![item],
		},
		_ => return vec![item],
	};
	let words: Vec<Node> = words_of(value).into_iter().chain(line_rest).collect();
	if let Some((Node::Text(written), children)) = words.split_first().map(|(first, children)| (first.drop_meta(), children)) {
		return attribute(written.clone(), children);
	}
	let spec_length = words.iter().take_while(|word| spec_word(word).is_some()).count();
	let spec: Vec<String> = words[..spec_length].iter().filter_map(spec_word).collect();
	attribute(spec.join(" "), &words[spec_length..])
}

/// The words of a value: `fade 200ms "a"` may arrive as unbracketed lists within each other
fn words_of(value: Node) -> Vec<Node> {
	match value.drop_meta() {
		Node::List(items, Bracket::None, _) => items.iter().cloned().flat_map(words_of).collect(),
		_ => vec![value],
	}
}

fn is_transition(name: &Node) -> bool {
	matches!(name.drop_meta(), Node::Symbol(name) if name == TRANSITION)
}

/// `data-wasp-transition: "<spec>"` and the children that followed the words
fn attribute(spec: String, children: &[Node]) -> Vec<Node> {
	let attribute = Node::Key(Box::new(Node::Symbol(TRANSITION_ATTRIBUTE.to_string())), Op::Colon, Box::new(Node::Text(spec)));
	std::iter::once(attribute).chain(children.iter().cloned()).collect()
}

/// A word of a transition: a name, or a duration in milliseconds
fn spec_word(word: &Node) -> Option<String> {
	match word.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		duration => crate::units::milliseconds(duration).map(|amount| format!("{amount}ms")),
	}
}
