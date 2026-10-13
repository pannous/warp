//! P52 (user, 2026-10-05): a function defined as a phrase with prepositions is called with them too:
//! `to add number a to number b: …` is called `add 1 to 2`, `square of a number = …` is called `square of 4`.
//! The definition's pattern (`_ to _`, `of _`) is learned here, before the other passes give it its parameter list;
//! the parser marks a `to` phrase with its pattern (PHRASE_MARK), a spaced definition still shows its words.
//! A call of that function whose arguments, read as values and preposition words, follow the pattern becomes
//! `name(values…)`. Elsewhere `to` stays a range and `of` a field lookup.
//! Gap feeding (wiki/call.md, card gap-feeding): a call that leaves out a slot after a phrase word takes the variable
//! named like that word, `from = 1; to = 3; integrate sin` of `to integrate a function from a to b`, and says so.

use super::nodes::{
	call, children_rewritten, in_block_as_written, is_assigned_data, is_words_call, key, spaced_statement,
	with_parts_rewritten,
};
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_name_matching::prepositions_among;
use crate::words::{PHRASE_MARK, SLOT};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq)]
enum Part {
	Slot,
	Word(String),
}

#[derive(Clone, Debug)]
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
	pattern
		.split(' ')
		.map(|part| {
			if part == SLOT {
				Part::Slot
			} else {
				Part::Word(part.to_string())
			}
		})
		.collect()
}

/// Every phrase's patterns, and the variables the program assigns, which feed a left-out slot of their word
struct Phrases {
	forms: HashMap<String, Vec<Vec<Part>>>,
	variables: HashSet<String>,
}

pub fn lower(program: Node) -> Node {
	let mut forms = HashMap::new();
	collect_patterns(&program, &mut forms);
	if forms.is_empty() {
		return program;
	}
	let variables = assigned_names(&program);
	rewrite(program, &Phrases { forms, variables })
}

fn assigned_names(program: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	program.visit(&mut |part| {
		if let Node::Key(name, Op::Assign, _) = part {
			names.extend(name.symbol_name().map(String::from));
		}
	});
	names
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
		if let Some((name, pattern)) =
			found.filter(|(_, pattern)| pattern.iter().any(|part| matches!(part, Part::Word(_))))
		{
			patterns.entry(name).or_default().push(pattern);
		}
	});
}

fn marked_pattern(head: &Node) -> Option<(String, Vec<Part>)> {
	// `to render text as pdf: …`: the head before its result type
	if let Node::Key(head, Op::As, _) = head {
		return marked_pattern(head);
	}
	let Node::Meta { node, data } = head else { return None };
	let Node::Key(mark, Op::Colon, pattern) = data.as_ref() else {
		return None;
	};
	if mark.name() != PHRASE_MARK {
		return None;
	}
	let Node::List(items, _, _) = node.drop_meta() else {
		return None;
	};
	Some(((items.first()?).symbol_name()?.to_string(), parts_of(&pattern.name())))
}

/// `square of a number = it*it` parses as the items `square of a (number = it*it)`
fn spaced_pattern(items: &[Node]) -> Option<(String, Vec<Part>)> {
	let (name, words, _) = spaced_statement(items)?;
	Some((
		name.symbol_name()?.to_string(),
		parts_of(&pattern_text(&words, &prepositions_among(&words))),
	))
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
fn matched(name: &str, arguments: &[Node], pattern: &[Part], variables: &HashSet<String>) -> Option<Vec<Node>> {
	let mut found = vec![];
	arguments
		.iter()
		.for_each(|argument| tokens(argument, pattern, &mut found));
	let found = with_gaps_fed(found, pattern, variables, (name, arguments));
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

/// The written tokens with each left-out `word _` of the pattern fed the variable named word, when the program has it
/// `call` is the phrase's name and written arguments, the hint's text; a trial match (no name) says nothing
fn with_gaps_fed(
	found: Vec<Token>,
	pattern: &[Part],
	variables: &HashSet<String>,
	call: (&str, &[Node]),
) -> Vec<Token> {
	if found.len() >= pattern.len() {
		return found;
	}
	let (mut fed, mut fed_words) = (vec![], vec![]);
	let mut written = found.iter().cloned().peekable();
	let mut parts = pattern.iter().peekable();
	while let Some(part) = parts.next() {
		let is_written = |token: Option<&Token>| matches!((part, token), (Part::Word(word), Some(Token::Word(given))) if given == word);
		match part {
			Part::Word(word)
				if !is_written(written.peek()) && parts.peek() == Some(&&Part::Slot) && variables.contains(word) =>
			{
				parts.next();
				fed.extend([Token::Word(word.clone()), Token::Value(symbol(word))]);
				fed_words.push(word.as_str());
			}
			_ => match written.next() {
				Some(token) => fed.push(token),
				None => return found,
			},
		}
	}
	if written.next().is_some() || fed_words.is_empty() {
		return found;
	}
	if !call.0.is_empty() {
		said_fed(call, &fed_words);
	}
	fed
}

/// Gap feeding is no silent default: the hint names the variables and the written form
fn said_fed((name, arguments): (&str, &[Node]), words: &[&str]) {
	if let Some(first) = arguments.first() {
		crate::normalize::set_position_of(first);
	}
	let written: Vec<String> = arguments
		.iter()
		.map(|argument| argument.serialize().trim().to_string())
		.collect();
	let written = format!("{name} {}", written.join(" "));
	let spelled: Vec<String> = words.iter().map(|word| format!("{word} {word}")).collect();
	crate::normalize::advise(
		&written,
		&format!("{written} {}", spelled.join(" ")),
		&format!(
			"the left-out slots take the variables {} (gap feeding)",
			words.join(", ")
		),
		None,
	);
}

/// `x = f [1] by 5`: an assigned value nests its words in pairs, `[[f, [1]], [by, 5]]`, and `y = add 1 to 2` reads
/// `(add 1) to 2`; a phrase call reads them flat
fn assigned_words(value: Node, phrases: &Phrases) -> Node {
	// `y = add 1 to 2 == 3` compares the phrase's value
	if let Node::Key(left, op, right) = value.drop_meta() {
		if op.is_relation() {
			return Node::Key(
				Box::new(assigned_words(left.as_ref().clone(), phrases)),
				*op,
				right.clone(),
			);
		}
	}
	let flat = spaced_words(std::slice::from_ref(&value));
	let Some((head, arguments)) = flat.split_first() else {
		return value;
	};
	let calls_a_phrase = head
		.symbol_name()
		.and_then(|name| phrases.forms.get(name))
		.is_some_and(|forms| {
			forms
				.iter()
				.any(|pattern| phrase_call("", arguments, pattern, &phrases.variables).is_some())
		});
	if calls_a_phrase && flat.len() > 1 {
		Node::List(flat, Bracket::None, Separator::Space)
	} else {
		value
	}
}

fn spaced_words(items: &[Node]) -> Vec<Node> {
	items
		.iter()
		.flat_map(|item| match item.drop_meta() {
			Node::List(words, Bracket::None, Separator::Space) => spaced_words(words),
			Node::Key(left, op, right) if op.as_str().chars().all(char::is_alphabetic) => [
				spaced_words(std::slice::from_ref(left)),
				vec![symbol(op.as_str())],
				spaced_words(std::slice::from_ref(right)),
			]
			.concat(),
			_ => vec![item.clone()],
		})
		.collect()
}

fn rewrite(node: Node, phrases: &Phrases) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = items.into_iter().map(|item| rewrite(item, phrases)).collect();
			let call = match items.split_first() {
				// a spaced definition (`foo of int = …`) has the shape of a call but defines the phrase
				Some((head, arguments)) if is_words_call(&bracket, &separator) && spaced_pattern(&items).is_none() => {
					let name = head.symbol_name();
					let call = name.and_then(|name| {
						phrases
							.forms
							.get(name)?
							.iter()
							.find_map(|pattern| phrase_call(name, arguments, pattern, &phrases.variables))
					});
					call.map(|call| in_block_as_written(&bracket, call))
				}
				_ => None,
			};
			call.unwrap_or(Node::List(items, bracket, separator))
		}
		Node::Key(left, Op::Assign, right) if is_assigned_data(&left, &right) => key(
			*left,
			Op::Assign,
			with_parts_rewritten(*right, &mut |part| rewrite(part, phrases)),
		),
		Node::Key(left, Op::Assign, right) => key(
			rewrite(*left, phrases),
			Op::Assign,
			rewrite(assigned_words(*right, phrases), phrases),
		),
		other => children_rewritten(other, |child| rewrite(child, phrases)),
	}
}

/// The call `name(values…)` of arguments that follow the pattern
fn phrase_call(name: &str, arguments: &[Node], pattern: &[Part], variables: &HashSet<String>) -> Option<Node> {
	// `square of x is 9`, `add 1 to 2 == 3`: the phrase's slots are nouns (`a number`), the comparison is about its
	// value (P149)
	let (mut values, mut compared) = match (
		matched(name, arguments, pattern, variables),
		arguments
			.split_last()
			.map(|(last, leading)| (last.drop_meta(), leading)),
	) {
		(Some(values), _) => (values, None),
		(None, Some((Node::Key(value, op, other), leading))) if op.is_relation() => (
			matched(name, &[leading, &[value.as_ref().clone()]].concat(), pattern, variables)?,
			Some((*op, other.clone())),
		),
		_ => return None,
	};
	if compared.is_none() {
		if let Some(Node::Key(value, op, other)) = values
			.last()
			.map(|last| last.drop_meta().clone())
			.filter(|last| matches!(last, Node::Key(_, op, _) if op.is_relation()))
		{
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
