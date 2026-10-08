//! Reflection words as dot forms (card reflection, notes/reflection.md): `f.effects` is `effects of f` (effects.rs),
//! `e.listeners` is `listeners of e` (event_signals.rs, signal_values.rs), so each word has one implementation.
//! A real field comes first: only a function's `.effects` and a listened name's `.listeners` are rewritten, a map's
//! own field `listeners` stays its field. `listeners of x` for a name nothing listens to is loud (card listeners-tick).

use std::collections::HashSet;

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const OF_WORD: &str = "of";
const EFFECTS_WORD: &str = "effects";
const LISTENERS_WORD: &str = "listeners";
/// `on alarm {…}`, `once tick {…}`, `on set x {…}`: the words before a listened name
const LISTENING_WORDS: [&str; 2] = ["on", "once"];
const VARIABLE_EVENTS: [&str; 2] = ["set", "change"];

pub fn lower(node: Node) -> Node {
	if !node.mentions_any(&[EFFECTS_WORD, LISTENERS_WORD]) {
		return node;
	}
	let defined = crate::library_words::defined_names(&node);
	let mut listened = HashSet::new();
	node.visit(&mut |child| if let Some(name) = listened_name(child) {
		listened.insert(name);
	});
	if let Some(name) = unlistened_reflection(&node, &listened, &defined) {
		return crate::node::error(&format!("nothing listens to {name}: `listeners of {name}` needs an `on {name} {{…}}` handler or a variable {name}"));
	}
	let mut context = crate::context::Context::new();
	crate::analyzer::extract_user_functions(&mut context, &node);
	let functions: HashSet<String> = context.user_functions.into_keys().collect();
	as_reflection_words(node, &functions, &listened)
}

/// The name an `on` or `once` statement listens to
fn listened_name(node: &Node) -> Option<String> {
	let Node::List(items, _, _) = node.drop_meta() else { return None };
	// `h = on alarm {…}` arrives grouped `on (alarm {…})`
	let words: Vec<&str> = items.iter().flat_map(|item| match item.drop_meta() {
		Node::List(parts, Bracket::None, _) => parts.iter().collect(),
		_ => vec![item],
	}).map(|item| symbol(item).unwrap_or("")).collect();
	match words.as_slice() {
		[listening, event, variable, ..] if LISTENING_WORDS.contains(listening) && VARIABLE_EVENTS.contains(event) && !variable.is_empty() => Some(variable.to_string()),
		[listening, event, ..] if LISTENING_WORDS.contains(listening) && !event.is_empty() => Some(event.to_string()),
		_ => None,
	}
}

fn symbol(node: &Node) -> Option<&str> {
	match node.drop_meta() {
		Node::Symbol(word) => Some(word),
		_ => None,
	}
}

/// A main-level `listeners of x` whose x is neither listened to nor a variable (a function's parameter may be)
fn unlistened_reflection(node: &Node, listened: &HashSet<String>, defined: &HashSet<String>) -> Option<String> {
	match node.drop_meta() {
		Node::Key(_, Op::Define, _) => None,
		Node::List(items, _, _) => items.windows(3).find_map(|window| match [symbol(&window[0]), symbol(&window[1]), symbol(&window[2])] {
			[Some(LISTENERS_WORD), Some(OF_WORD), Some(name)] if !listened.contains(name) && !defined.contains(name) => Some(name.to_string()),
			_ => None,
		}).or_else(|| items.iter().find_map(|item| unlistened_reflection(item, listened, defined))),
		Node::Key(left, _, right) => unlistened_reflection(left, listened, defined).or_else(|| unlistened_reflection(right, listened, defined)),
		_ => None,
	}
}

/// `f.effects` → `effects of f` for a function f, `e.listeners` → `listeners of e` for a listened e
fn as_reflection_words(node: Node, functions: &HashSet<String>, listened: &HashSet<String>) -> Node {
	if let Node::Key(subject, Op::Dot, word) = node.drop_meta() {
		if let (Some(name), Some(word)) = (symbol(subject), symbol(word)) {
			let reflected = match word {
				EFFECTS_WORD => functions.contains(name),
				LISTENERS_WORD => listened.contains(name),
				_ => false,
			};
			if reflected {
				return Node::List(vec![Node::Symbol(word.into()), Node::Symbol(OF_WORD.into()), Node::Symbol(name.into())], Bracket::None, Separator::Space);
			}
		}
	}
	node.map_children(|child| as_reflection_words(child, functions, listened))
}
