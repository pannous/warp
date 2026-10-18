//! Transitions as CSS (card web-css, P188: stay close to HTML and CSS; notes/web_framework.md step 15): in an element's
//! block `transition: opacity 200ms` is its inline CSS transition (joined to its `style`), and `starting-style: {…}` the
//! state it enters from and leaves towards, CSS's @starting-style, which an inline style cannot hold: it is the attribute
//! `data-warp-starting-style`, applied by the page (markup-transitions.js), which also glides keyed elements with a
//! transform transition to their new places. The words are data, not variables: names and durations, normalized to
//! milliseconds; a text is taken as written; the words end where the element's children begin (`p{ transition: opacity
//! 1s "text" }`). The kinds before CSS, `transition: fade 200ms` (also scale, slide), are that CSS, with a hint to it.
//! Keyframes are CSS's own (card web-keyframes, P188): `"@keyframes pop": {…}` in a style sheet, `animation: pop 400ms`
//! in an element its inline CSS animation like a transition. What CSS lacks is a removal that waits: `leave: pop 400ms
//! reverse` is the animation the page plays before it removes the element (attribute `data-warp-leave`).

use super::nodes::{key, phrase_words_of};
use crate::element_events::with_element_items;
use crate::node::{symbol, text, Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::STYLE_WORD;

const TRANSITION: &str = "transition";
const STARTING_STYLE: &str = "starting-style";
pub const STARTING_STYLE_ATTRIBUTE: &str = "data-warp-starting-style";
const ANIMATION: &str = "animation";
const LEAVE: &str = "leave";
pub const LEAVE_ATTRIBUTE: &str = "data-warp-leave";
/// The element words whose CSS words lower here: the inline style's transition and animation, the page's leave
const CSS_WORDS: [&str; 3] = [TRANSITION, ANIMATION, LEAVE];
/// The element words this pass reads, no attributes as written
pub const ELEMENT_WORDS: [&str; 4] = [TRANSITION, ANIMATION, LEAVE, STARTING_STYLE];
/// CSS timing words that may follow a transition's duration as words of their own (`opacity 200ms ease-out`)
const TIMING_WORDS: [&str; 15] = [
	"ease",
	"ease-in",
	"ease-out",
	"ease-in-out",
	"linear",
	"step-start",
	"step-end",
	// an animation's direction, fill mode and count
	"normal",
	"reverse",
	"alternate",
	"alternate-reverse",
	"forwards",
	"backwards",
	"both",
	"infinite",
];
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
	if node.drop_meta().name() == STYLE_WORD {
		return node;
	}
	with_element_items(node, |items| {
		let mut style_declarations: Vec<(&str, String)> = vec![];
		let mut style_at = None;
		let mut lowered: Vec<Node> = vec![];
		for item in with_line_words(items) {
			let Some((property, words)) = css_words_of(&item) else {
				lowered.push(page_attribute(item));
				continue;
			};
			let (spec, children) = spec_and_children(words);
			if property == LEAVE {
				lowered.push(css_pair(LEAVE_ATTRIBUTE, text(&spec)));
			} else {
				let spec = match made_up_kind(&spec).filter(|_| property == TRANSITION) {
					Some((kind, starting)) => {
						let css = css_of_kind(&spec);
						advise_css(&spec, &css, starting, kind);
						lowered.push(css_pair(STARTING_STYLE_ATTRIBUTE, declarations(starting)));
						css
					}
					None => spec,
				};
				style_at.get_or_insert(lowered.len());
				style_declarations.push((property, spec));
			}
			lowered.extend(children.into_iter().map(page_attribute));
		}
		match style_at {
			Some(at) => with_style_declarations(lowered, at, &style_declarations),
			None => lowered,
		}
	})
}

/// A transition's words, as one item: `transition: …` leading a line of words, and the timing words after it
fn with_line_words(items: Vec<Node>) -> Vec<Node> {
	let mut joined: Vec<Node> = vec![];
	for item in items {
		let follows_transition = joined.last().is_some_and(|last| css_words_of(last).is_some());
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

/// The CSS word and the words of `transition: …`, `animation: …` or `leave: …` (alone, or leading a line of words that
/// ends with the children)
fn css_words_of(item: &Node) -> Option<(&'static str, Vec<Node>)> {
	let css_word = |name: &Node| CSS_WORDS.iter().copied().find(|word| name.is_symbol(word));
	match item.drop_meta() {
		Node::Key(name, Op::Colon, value) => Some((css_word(name)?, phrase_words_of(value))),
		Node::List(items, Bracket::None, _) => match items.split_first().map(|(first, rest)| (first.drop_meta(), rest))
		{
			Some((Node::Key(name, Op::Colon, value), rest)) => {
				Some((css_word(name)?, phrase_words_of(value).into_iter().chain(rest.iter().cloned()).collect()))
			}
			_ => None,
		},
		_ => None,
	}
}

/// The spec as text and the children after its words: a text is taken as written
fn spec_and_children(words: Vec<Node>) -> (String, Vec<Node>) {
	if let Some((Node::Text(written), children)) =
		words.split_first().map(|(first, children)| (first.drop_meta(), children))
	{
		return (written.clone(), children.to_vec());
	}
	let spec_length = words.iter().take_while(|word| spec_word(word).is_some()).count();
	let spec: Vec<String> = words[..spec_length].iter().filter_map(spec_word).collect();
	(spec.join(" "), words[spec_length..].to_vec())
}

fn is_timing_word(item: &Node) -> bool {
	matches!(item.drop_meta(), Node::Symbol(word) if TIMING_WORDS.contains(&word.as_str()))
		|| crate::numbers::units::milliseconds(item.drop_meta()).is_some()
}

fn css_pair(name: &str, value: Node) -> Node {
	key(symbol(name), Op::Colon, value)
}

/// `starting-style: { opacity: 0 }` is the attribute the page applies
fn page_attribute(item: Node) -> Node {
	match item.drop_meta() {
		Node::Key(name, Op::Colon, value) if name.is_symbol(STARTING_STYLE) => {
			css_pair(STARTING_STYLE_ATTRIBUTE, value.as_ref().clone())
		}
		_ => item,
	}
}

/// A word of a transition: a name, or a duration in milliseconds
fn spec_word(word: &Node) -> Option<String> {
	match word.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		duration => crate::numbers::units::milliseconds(duration).map(|amount| format!("{amount}ms")),
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
	let timing = if has_duration {
		timing.join(" ")
	} else {
		[DEFAULT_DURATION].iter().chain(&timing).copied().collect::<Vec<_>>().join(" ")
	};
	MADE_UP_PROPERTIES.map(|property| format!("{property} {timing}")).join(", ")
}

/// `{ opacity: "0" }`: CSS declarations as an element's block holds them
fn declarations(pairs: &[(&str, &str)]) -> Node {
	Node::List(
		pairs.iter().map(|(name, value)| css_pair(name, text(value))).collect(),
		Bracket::Curly,
		Separator::Space,
	)
}

/// `transition: fade 200ms` → `transition: "opacity 200ms, transform 200ms" starting-style: { opacity: 0 }`
fn advise_css(spec: &str, css: &str, starting: &[(&str, &str)], kind: &str) {
	let starting: Vec<String> = starting
		.iter()
		.map(
			|(name, value)| {
				if value.parse::<f64>().is_ok() {
					format!("{name}: {value}")
				} else {
					format!("{name}: \"{value}\"")
				}
			},
		)
		.collect();
	let canonical = format!("transition: \"{css}\" starting-style: {{ {} }}", starting.join(" "));
	let (written, reason) = (format!("transition: {spec}"), format!("{kind} is no CSS: a transition names CSS properties, the state an element enters from is its starting-style"));
	crate::normalize::advise(
		&written,
		&canonical,
		&reason,
		Some(crate::normalize::rewrite(&written, &canonical, &reason)),
	);
}

/// The CSS transition, the style at `at`, joins the element's own inline style when it has one: its declarations or text
fn with_style_declarations(mut items: Vec<Node>, at: usize, added: &[(&str, String)]) -> Vec<Node> {
	let is_style =
		|item: &Node| matches!(item.drop_meta(), Node::Key(name, Op::Colon, _) if name.is_symbol(STYLE_WORD));
	let pairs: Vec<(&str, &str)> = added.iter().map(|(property, spec)| (*property, spec.as_str())).collect();
	let Some(own) = items.iter().position(is_style) else {
		items.insert(at, css_pair(STYLE_WORD, declarations(&pairs)));
		return items;
	};
	let Node::Key(_, _, value) = items[own].drop_meta() else { unreachable!("a style") };
	let Node::List(added, _, _) = declarations(&pairs) else { unreachable!("declarations") };
	let joined = match value.drop_meta() {
		Node::Text(written) => Node::Text(
			pairs.iter().fold(written.clone(), |joined, (property, spec)| format!("{joined}; {property}: {spec}")),
		),
		Node::List(own, bracket, separator) => {
			Node::List(own.iter().cloned().chain(added).collect(), bracket.clone(), separator.clone())
		}
		one => Node::List([vec![one.clone()], added].concat(), Bracket::Curly, Separator::Space), // `{ color: red }`, one declaration
	};
	items[own] = css_pair(STYLE_WORD, joined);
	items
}
