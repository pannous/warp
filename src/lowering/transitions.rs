//! Transitions as CSS (card web-css, P188: stay close to HTML and CSS; notes/web_framework.md step 15): in an element's
//! block `transition: opacity 200ms` is its inline CSS transition (joined to its `style`), and `starting-style: {…}` the
//! state it enters from and leaves towards, CSS's @starting-style, which an inline style cannot hold: it is the attribute
//! `data-wasp-starting-style`, applied by the page (markup-transitions.js), which also glides keyed elements with a
//! transform transition to their new places. The words are data, not variables: names and durations, normalized to
//! milliseconds; a text is taken as written; the words end where the element's children begin (`p{ transition: opacity
//! 1s "text" }`). The kinds before CSS, `transition: fade 200ms` (also scale, slide), are that CSS, with a hint to it.

use crate::element_events::with_element_items;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const TRANSITION: &str = "transition";
const STYLE: &str = "style";
const STARTING_STYLE: &str = "starting-style";
pub const STARTING_STYLE_ATTRIBUTE: &str = "data-wasp-starting-style";
/// CSS timing words that may follow a transition's duration as words of their own (`opacity 200ms ease-out`)
const TIMING_WORDS: [&str; 7] = ["ease", "ease-in", "ease-out", "ease-in-out", "linear", "step-start", "step-end"];
/// The kinds before CSS and the starting style each means; they transition opacity and, for moves, transform
const MADE_UP_KINDS: [(&str, &[(&str, &str)]); 3] = [
	("fade", &[("opacity", "0")]),
	("scale", &[("opacity", "0"), ("transform", "scale(0.8)")]),
	("slide", &[("opacity", "0"), ("transform", "translateY(-1em)")]),
];
const MADE_UP_PROPERTIES: [&str; 2] = ["opacity", "transform"];
const DEFAULT_DURATION: &str = "200ms";

pub fn lower(node: Node) -> Node {
	let node = node.map_children(lower);
	// `style: { transition: "opacity 1s" }` is an inline style, its transition the CSS property
	if node.drop_meta().name() == STYLE {
		return node;
	}
	with_element_items(node, |items| {
		let mut transition_style = None;
		let mut lowered: Vec<Node> = vec![];
		for item in with_line_words(items) {
			let Some(words) = transition_of(&item) else {
				lowered.push(starting_style_attribute(item));
				continue;
			};
			let (spec, children) = spec_and_children(words);
			let spec = match made_up_kind(&spec) {
				Some((kind, starting)) => {
					let css = css_of_kind(&spec);
					advise_css(&spec, &css, starting, kind);
					lowered.push(css_pair(STARTING_STYLE_ATTRIBUTE, declarations(starting)));
					css
				}
				None => spec,
			};
			transition_style = Some((lowered.len(), spec.clone()));
			lowered.push(css_pair(STYLE, declarations(&[(TRANSITION, &spec)])));
			lowered.extend(children);
		}
		match transition_style {
			Some((at, spec)) => with_style_transition(lowered, at, spec),
			None => lowered,
		}
	})
}

/// A transition's words, as one item: `transition: …` leading a line of words, and the timing words after it
fn with_line_words(items: Vec<Node>) -> Vec<Node> {
	let mut joined: Vec<Node> = vec![];
	for item in items {
		let follows_transition = joined.last().is_some_and(|last| transition_of(last).is_some());
		if follows_transition && is_timing_word(&item) {
			let last = joined.pop().expect("a transition");
			let Node::Key(name, op, value) = last.drop_meta().clone() else { unreachable!("a transition") };
			let words = Node::List(vec![*value, item], Bracket::None, Separator::Space);
			joined.push(Node::Key(name, op, Box::new(words)));
		} else {
			joined.push(item);
		}
	}
	joined
}

/// The words of `transition: …` (alone, or leading a line of words that ends with the children)
fn transition_of(item: &Node) -> Option<Vec<Node>> {
	match item.drop_meta() {
		Node::Key(name, Op::Colon, value) if is_named(name, TRANSITION) => Some(words_of(value.as_ref().clone())),
		Node::List(items, Bracket::None, _) => match items.split_first().map(|(first, rest)| (first.drop_meta(), rest)) {
			Some((Node::Key(name, Op::Colon, value), rest)) if is_named(name, TRANSITION) => Some(words_of(value.as_ref().clone()).into_iter().chain(rest.iter().cloned()).collect()),
			_ => None,
		},
		_ => None,
	}
}

/// The spec as text and the children after its words: a text is taken as written
fn spec_and_children(words: Vec<Node>) -> (String, Vec<Node>) {
	if let Some((Node::Text(written), children)) = words.split_first().map(|(first, children)| (first.drop_meta(), children)) {
		return (written.clone(), children.to_vec());
	}
	let spec_length = words.iter().take_while(|word| spec_word(word).is_some()).count();
	let spec: Vec<String> = words[..spec_length].iter().filter_map(spec_word).collect();
	(spec.join(" "), words[spec_length..].to_vec())
}

/// The words of a value: `fade 200ms "a"` may arrive as unbracketed lists within each other
fn words_of(value: Node) -> Vec<Node> {
	match value.drop_meta() {
		Node::List(items, Bracket::None, _) => items.iter().cloned().flat_map(words_of).collect(),
		_ => vec![value],
	}
}

fn is_named(name: &Node, wanted: &str) -> bool {
	matches!(name.drop_meta(), Node::Symbol(name) if name == wanted)
}

fn is_timing_word(item: &Node) -> bool {
	matches!(item.drop_meta(), Node::Symbol(word) if TIMING_WORDS.contains(&word.as_str())) || crate::units::milliseconds(item.drop_meta()).is_some()
}

fn css_pair(name: &str, value: Node) -> Node {
	Node::Key(Box::new(Node::Symbol(name.to_string())), Op::Colon, Box::new(value))
}

/// `starting-style: { opacity: 0 }` is the attribute the page applies
fn starting_style_attribute(item: Node) -> Node {
	match item.drop_meta() {
		Node::Key(name, Op::Colon, value) if is_named(name, STARTING_STYLE) => css_pair(STARTING_STYLE_ATTRIBUTE, value.as_ref().clone()),
		_ => item,
	}
}

/// A word of a transition: a name, or a duration in milliseconds
fn spec_word(word: &Node) -> Option<String> {
	match word.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		duration => crate::units::milliseconds(duration).map(|amount| format!("{amount}ms")),
	}
}

fn made_up_kind(spec: &str) -> Option<&'static (&'static str, &'static [(&'static str, &'static str)])> {
	let first = spec.split_whitespace().next()?;
	MADE_UP_KINDS.iter().find(|(kind, _)| *kind == first)
}

/// `fade 1s ease-out` → `opacity 1s ease-out, transform 1s ease-out` (200ms when it names no duration)
fn css_of_kind(spec: &str) -> String {
	let timing: Vec<&str> = spec.split_whitespace().skip(1).collect();
	let has_duration = timing.iter().any(|word| word.starts_with(|first: char| first.is_ascii_digit()));
	let timing = if has_duration { timing.join(" ") } else { [DEFAULT_DURATION].iter().chain(&timing).copied().collect::<Vec<_>>().join(" ") };
	MADE_UP_PROPERTIES.map(|property| format!("{property} {timing}")).join(", ")
}

/// `{ opacity: "0" }`: CSS declarations as an element's block holds them
fn declarations(pairs: &[(&str, &str)]) -> Node {
	Node::List(pairs.iter().map(|(name, value)| css_pair(name, Node::Text(value.to_string()))).collect(), Bracket::Curly, Separator::Space)
}

/// `transition: fade 200ms` → `transition: "opacity 200ms, transform 200ms" starting-style: { opacity: 0 }`
fn advise_css(spec: &str, css: &str, starting: &[(&str, &str)], kind: &str) {
	let starting: Vec<String> = starting.iter().map(|(name, value)| if value.parse::<f64>().is_ok() { format!("{name}: {value}") } else { format!("{name}: \"{value}\"") }).collect();
	let canonical = format!("transition: \"{css}\" starting-style: {{ {} }}", starting.join(" "));
	crate::normalize::advise(&format!("transition: {spec}"), &canonical, &format!("{kind} is no CSS: a transition names CSS properties, the state an element enters from is its starting-style"));
}

/// The CSS transition, the style at `at`, joins the element's own inline style when it has one: its declarations or text
fn with_style_transition(mut items: Vec<Node>, at: usize, spec: String) -> Vec<Node> {
	let is_style = |item: &Node| matches!(item.drop_meta(), Node::Key(name, Op::Colon, _) if is_named(name, STYLE));
	let Some(own) = items.iter().enumerate().position(|(index, item)| index != at && is_style(item)) else { return items };
	let declaration = css_pair(TRANSITION, Node::Text(spec.clone()));
	let Node::Key(_, _, value) = items[own].drop_meta() else { unreachable!("a style") };
	let joined = match value.drop_meta() {
		Node::Text(written) => Node::Text(format!("{written}; {TRANSITION}: {spec}")),
		Node::List(declarations, bracket, separator) => Node::List(declarations.iter().cloned().chain([declaration]).collect(), bracket.clone(), separator.clone()),
		one => Node::List(vec![one.clone(), declaration], Bracket::Curly, Separator::Space), // `{ color: red }`, one declaration
	};
	items[own] = css_pair(STYLE, joined);
	items.remove(at);
	items
}
