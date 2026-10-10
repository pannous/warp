//! P52 (user, 2026-10-05): a function defined as a phrase with prepositions is called with them too:
//! `to add number a to number b: …` is called `add 1 to 2`, `square of a number = …` is called `square of 4`.
//! The definition's pattern (`_ to _`, `of _`) is learned here, before the other passes give it its parameter list;
//! the parser marks a `to` phrase with its pattern (PHRASE_MARK), a spaced definition still shows its words.
//! A call of that function whose arguments, read as values and preposition words, follow the pattern becomes
//! `name(values…)`. Elsewhere `to` stays a range and `of` a field lookup.

use super::nodes::{call, key, spaced_statement, word_of};
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_name_matching::prepositions_among;
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

/// The pattern of the words after a phrase's name: each run of words between its phrase words (prepositions,
/// type_name_matching::phrase_words) is one slot, whatever its articles and type words (`a number`, `number a`, `a`)
pub fn pattern_text(words: &[&str], in_phrase: &[bool]) -> String {
	let mut parts: Vec<&str> = vec![];
	for (word, phrase_word) in words.iter().zip(in_phrase) {
		if *phrase_word {
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

/// Patterns with a preposition: from the parser's mark on a `to` phrase and from spaced definitions `name words… = body`
/// Every pattern of a name: `to play f:` and `to play f for d:` are two (overloads.rs tells them apart by arity)
fn collect_patterns(node: &Node, patterns: &mut HashMap<String, Vec<Vec<Part>>>) {
	node.visit(&mut |part| {
		let found = match part {
			Node::Key(head, Op::Define | Op::Assign, _) => marked_pattern(head),
			Node::List(items, Bracket::None, Separator::Space) => spaced_pattern(items),
			_ => None,
		};
		if let Some((name, pattern)) = found.filter(|(_, pattern)| pattern.iter().any(|part| matches!(part, Part::Word(_)))) {
			patterns.entry(name).or_default().push(pattern);
		}
	});
}

fn marked_pattern(head: &Node) -> Option<(String, Vec<Part>)> {
	let Node::Meta { node, data } = head else { return None };
	let Node::Key(mark, Op::Colon, pattern) = data.as_ref() else { return None };
	if mark.name() != PHRASE_MARK {
		return None;
	}
	let Node::List(items, _, _) = node.drop_meta() else { return None };
	Some((word_of(items.first()?)?.to_string(), parts_of(&pattern.name())))
}

/// `square of a number = it*it` parses as the items `square of a (number = it*it)`
fn spaced_pattern(items: &[Node]) -> Option<(String, Vec<Part>)> {
	let (name, words, _) = spaced_statement(items)?;
	Some((word_of(name)?.to_string(), parts_of(&pattern_text(&words, &prepositions_among(&words)))))
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

/// `x = f [1] by 5`: an assigned value nests its words in pairs, `[[f, [1]], [by, 5]]`, and `y = add 1 to 2` reads
/// `(add 1) to 2`; a phrase call reads them flat
fn assigned_words(value: Node, patterns: &HashMap<String, Vec<Vec<Part>>>) -> Node {
	// `y = add 1 to 2 == 3` compares the phrase's value
	if let Node::Key(left, op, right) = value.drop_meta() {
		if op.is_comparison() {
			return Node::Key(Box::new(assigned_words(left.as_ref().clone(), patterns)), *op, right.clone());
		}
	}
	let flat = spaced_words(std::slice::from_ref(&value));
	let Some((head, arguments)) = flat.split_first() else { return value };
	let calls_a_phrase = word_of(head).and_then(|name| patterns.get(name)).is_some_and(|forms| forms.iter().any(|pattern| phrase_call("", arguments, pattern).is_some()));
	if calls_a_phrase && flat.len() > 1 { Node::List(flat, Bracket::None, Separator::Space) } else { value }
}

fn spaced_words(items: &[Node]) -> Vec<Node> {
	items.iter().flat_map(|item| match item.drop_meta() {
		Node::List(words, Bracket::None, Separator::Space) => spaced_words(words),
		Node::Key(left, op, right) if op.as_str().chars().all(char::is_alphabetic) => {
			[spaced_words(std::slice::from_ref(left)), vec![symbol(op.as_str())], spaced_words(std::slice::from_ref(right))].concat()
		}
		_ => vec![item.clone()],
	}).collect()
}

fn rewrite(node: Node, patterns: &HashMap<String, Vec<Vec<Part>>>) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = items.into_iter().map(|item| rewrite(item, patterns)).collect();
			let call = match items.split_first() {
				// a spaced definition (`foo of int = …`) has the shape of a call but defines the phrase
				Some((head, arguments)) if bracket == Bracket::None && separator == Separator::Space && spaced_pattern(&items).is_none() => {
					let name = word_of(head);
					name.and_then(|name| patterns.get(name)?.iter().find_map(|pattern| phrase_call(name, arguments, pattern)))
				}
				_ => None,
			};
			call.unwrap_or(Node::List(items, bracket, separator))
		}
		Node::Key(left, Op::Assign, right) => key(rewrite(*left, patterns), Op::Assign, rewrite(assigned_words(*right, patterns), patterns)),
		Node::Key(left, op, right) => key(rewrite(*left, patterns), op, rewrite(*right, patterns)),
		Node::Meta { node, data } => Node::Meta { node: Box::new(rewrite(*node, patterns)), data },
		other => other,
	}
}

/// The call `name(values…)` of arguments that follow the pattern
fn phrase_call(name: &str, arguments: &[Node], pattern: &[Part]) -> Option<Node> {
	// `square of x is 9`, `add 1 to 2 == 3`: the phrase's slots are nouns (`a number`), the comparison is about its
	// value (P149)
	let (mut values, mut compared) = match (matched(arguments, pattern), arguments.split_last().map(|(last, leading)| (last.drop_meta(), leading))) {
		(Some(values), _) => (values, None),
		(None, Some((Node::Key(value, op, other), leading))) if op.is_comparison() => {
			(matched(&[leading, &[value.as_ref().clone()]].concat(), pattern)?, Some((*op, other.clone())))
		}
		_ => return None,
	};
	if compared.is_none() {
		if let Some(Node::Key(value, op, other)) = values.last().map(|last| last.drop_meta().clone()).filter(|last| matches!(last, Node::Key(_, op, _) if op.is_comparison())) {
			*values.last_mut().expect("not empty") = *value;
			compared = Some((op, other));
		}
	}
	let call = call(name, values);
	Some(match compared {
		Some((op, other)) => Node::Key(Box::new(call), op, other),
		None => call,
	})
}
