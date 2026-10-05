//! P52 (user, 2026-10-05): a function defined as a phrase with prepositions is called with them too:
//! `to add number a to number b: …` is called `add 1 to 2`, `square of a number = …` is called `square of 4`.
//! The definition's pattern (`_ to _`, `of _`) is learned here, before the other passes give it its parameter list;
//! the parser marks a `to` phrase with its pattern (PHRASE_MARK), a spaced definition still shows its words.
//! A call of that function whose arguments, read as values and preposition words, follow the pattern becomes
//! `name(values…)`. Elsewhere `to` stays a range and `of` a field lookup.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_name_matching::PREPOSITIONS;
use std::collections::HashMap;

/// The Meta key the parser puts on the head of a `to` phrase: its pattern as text, `_ to _`
pub const PHRASE_MARK: &str = "phrase";
/// A parameter slot in a pattern
pub const SLOT: &str = "_";

#[derive(Clone, Debug, PartialEq)]
enum Part {
	Slot,
	Word(String),
}

#[derive(Debug)]
enum Token {
	Value(Node),
	Word(String),
}

/// The pattern of the words after a phrase's name: each run of words between prepositions is one slot, whatever its
/// articles and type words (`a number`, `number a`, `a`)
pub fn pattern_text(words: &[&str]) -> String {
	let mut parts: Vec<&str> = vec![];
	for word in words {
		if PREPOSITIONS.contains(word) {
			parts.push(word);
		} else if parts.last() != Some(&SLOT) {
			parts.push(SLOT);
		}
	}
	parts.join(" ")
}

fn parts_of(pattern: &str) -> Vec<Part> {
	pattern.split(' ').map(|part| if part == SLOT { Part::Slot } else { Part::Word(part.to_string()) }).collect()
}

pub fn lower(program: Node) -> Node {
	let mut patterns = HashMap::new();
	collect_patterns(&program, &mut patterns);
	if patterns.is_empty() {
		return program;
	}
	rewrite(program, &patterns)
}

fn word(node: &Node) -> Option<&str> {
	match node.drop_meta() {
		Node::Symbol(name) => Some(name),
		_ => None,
	}
}

/// Patterns with a preposition: from the parser's mark on a `to` phrase and from spaced definitions `name words… = body`
fn collect_patterns(node: &Node, patterns: &mut HashMap<String, Vec<Part>>) {
	node.visit(&mut |part| match part {
		Node::Key(head, Op::Define | Op::Assign, _) => {
			if let Some((name, pattern)) = marked_pattern(head) {
				patterns.insert(name, pattern);
			}
		}
		Node::List(items, Bracket::None, Separator::Space) => {
			if let Some((name, pattern)) = spaced_pattern(items) {
				patterns.insert(name, pattern);
			}
		}
		_ => {}
	});
	patterns.retain(|_, pattern| pattern.iter().any(|part| matches!(part, Part::Word(_))));
}

fn marked_pattern(head: &Node) -> Option<(String, Vec<Part>)> {
	let Node::Meta { node, data } = head else { return None };
	let Node::Key(mark, Op::Colon, pattern) = data.as_ref() else { return None };
	if mark.name() != PHRASE_MARK {
		return None;
	}
	let Node::List(items, _, _) = node.drop_meta() else { return None };
	Some((word(items.first()?)?.to_string(), parts_of(&pattern.name())))
}

/// `square of a number = it*it` parses as the items `square of a (number = it*it)`
fn spaced_pattern(items: &[Node]) -> Option<(String, Vec<Part>)> {
	let (name, rest) = items.split_first()?;
	let (last, middle) = rest.split_last()?;
	let Node::Key(last_word, Op::Assign | Op::Define, _) = last.drop_meta() else { return None };
	let words: Vec<&str> = middle.iter().chain(std::iter::once(last_word.as_ref())).map(word).collect::<Option<_>>()?;
	Some((word(name)?.to_string(), parts_of(&pattern_text(&words))))
}

/// The arguments of a call as values and preposition words: `1 to 2` parses as the range `1 to 2`, which is two values
/// around the word `to` here
fn tokens(node: &Node, words: &[Part], found: &mut Vec<Token>) {
	let is_word = |text: &str| words.contains(&Part::Word(text.to_string()));
	match node.drop_meta() {
		Node::Key(left, op, right) if is_word(op.as_str()) => {
			tokens(left, words, found);
			found.push(Token::Word(op.as_str().to_string()));
			tokens(right, words, found);
		}
		Node::Symbol(name) if is_word(name) => found.push(Token::Word(name.clone())),
		other => found.push(Token::Value(other.clone())),
	}
}

/// The values of a call that follows the pattern, in slot order
fn matched(arguments: &[Node], pattern: &[Part]) -> Option<Vec<Node>> {
	let mut found = vec![];
	arguments.iter().for_each(|argument| tokens(argument, pattern, &mut found));
	if found.len() != pattern.len() {
		return None;
	}
	let mut values = vec![];
	for (token, part) in found.into_iter().zip(pattern) {
		match (token, part) {
			(Token::Value(value), Part::Slot) => values.push(value),
			(Token::Word(written), Part::Word(wanted)) if written == *wanted => {}
			_ => return None,
		}
	}
	Some(values)
}

fn rewrite(node: Node, patterns: &HashMap<String, Vec<Part>>) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = items.into_iter().map(|item| rewrite(item, patterns)).collect();
			let call = match items.split_first() {
				// a spaced definition (`foo of int = …`) has the shape of a call but defines the phrase
				Some((head, arguments)) if bracket == Bracket::None && separator == Separator::Space && spaced_pattern(&items).is_none() => {
					word(head).and_then(|name| patterns.get(name).map(|pattern| (name, pattern))).and_then(|(name, pattern)| {
						let values = matched(arguments, pattern)?;
						Some(Node::List([vec![Node::Symbol(name.to_string())], values].concat(), Bracket::Round, Separator::None))
					})
				}
				_ => None,
			};
			call.unwrap_or(Node::List(items, bracket, separator))
		}
		Node::Key(left, op, right) => Node::Key(Box::new(rewrite(*left, patterns)), op, Box::new(rewrite(*right, patterns))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(rewrite(*node, patterns)), data },
		other => other,
	}
}
