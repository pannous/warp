//! Text patterns (card text-patterns, decision pattern-roadmap): `"0x" + rest`, `name + ".txt"`, `"/users/" + id: int`.
//! Pieces and holes alternate, at most one hole between two pieces; a hole's type reads its text as a value (`id: int`)
//! or names the class of its characters (`"v" + n: digits`, `c: digit`). The prelude's text_holes cuts the subject and
//! tests each hole's type (fits_text_type).

use super::nodes::{call, key};
use super::switch::{bind, is_switch_head, split_switch, WILDCARD};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::{COUNT_WORD, TEXT_WORD};

/// The types of a hole: those reading its text as a value, with the conversion, and the character classes, without
const TEXT_HOLE_TYPES: [(&str, Option<&str>); 13] = [
	(TEXT_WORD, None),
	("int", Some("int")),
	("integer", Some("int")),
	("number", Some("float")),
	("float", Some("float")),
	("digit", None),
	("digits", None),
	("letter", None),
	("letters", None),
	("hex", None),
	("space", None),
	("word", None),
	(WILDCARD, None),
];
const TEXT_HOLES: &str = "text_holes";
const ADJACENT_HOLES: &str = "a text pattern needs a piece between two holes: \"(\" + a + \",\" + b + \")\"";

/// `"/users/" + id: int` as its pieces `["/users/", ""]` and holes `[id: int]`
pub(crate) struct TextPattern {
	/// the parts before, between and after the holes, one more than the holes: "" where nothing stands
	pieces: Vec<Node>,
	holes: Vec<(Node, &'static str)>,
}

impl TextPattern {
	/// A sum of parts holding a text literal and a hole: a name `is_known` does not know, `_`, or such a name typed by
	/// a hole type (`id: int`); any other sum is a value to compare
	pub(crate) fn of(pattern: &Node, is_known: &dyn Fn(&str) -> bool) -> Option<TextPattern> {
		if !matches!(pattern.drop_meta(), Node::Key(_, Op::Add | Op::Colon, _)) {
			return None;
		}
		let mut parts = vec![];
		flattened(pattern, &mut parts);
		if !parts.iter().any(|part| as_text(part).is_some()) {
			return None;
		}
		let mut pieces = vec![Node::Text(String::new())];
		let mut holes = vec![];
		for part in parts {
			if let Some(hole) = hole(&part, is_known) {
				holes.push(hole?);
				pieces.push(Node::Text(String::new()));
				continue;
			}
			let part = as_text(&part).unwrap_or(part);
			let piece = pieces.pop().expect("one piece per hole and one more");
			pieces.push(match piece.drop_meta() {
				Node::Text(empty) if empty.is_empty() => part,
				_ => key(piece, Op::Add, part),
			});
		}
		(!holes.is_empty()).then_some(TextPattern { pieces, holes })
	}

	/// The subject at `path` is a text the pieces cut, each hole's text fitting its type; the holes bound by name
	pub(crate) fn matched(&self, path: &Node, tests: &mut Vec<Node>, bindings: &mut Vec<Node>) {
		let middle = &self.pieces[1..self.pieces.len() - 1];
		if middle.iter().any(|piece| matches!(piece.drop_meta(), Node::Text(empty) if empty.is_empty())) {
			tests.push(crate::node::error(ADJACENT_HOLES));
			// the names still bound, so the error is the one the arm reports
			let unbound = self.holes.iter().filter(|(name, _)| !name.is_symbol(WILDCARD));
			bindings.extend(unbound.map(|(name, _)| bind(name.clone(), Node::Text(String::new()))));
			return;
		}
		let types = self.holes.iter().map(|(_, type_word)| Node::Text(type_word.to_string())).collect();
		let list = |items| Node::List(items, Bracket::Square, Separator::Comma);
		let cut = call(TEXT_HOLES, vec![path.clone(), list(self.pieces.clone()), list(types)]);
		tests.push(key(call(COUNT_WORD, vec![cut.clone()]), Op::Gt, Node::int(0)));
		for (index, (name, type_word)) in self.holes.iter().enumerate() {
			let hole_text = key(cut.clone(), Op::Hash, Node::int(index as i64 + 1));
			let value = match conversion(type_word) {
				Some(convert) => call(convert, vec![hole_text]),
				None => hole_text,
			};
			if !name.is_symbol(WILDCARD) {
				bindings.push(bind(name.clone(), value));
			}
		}
	}
}

/// The prelude word a program's text pattern calls once lowered (needed_definitions IMPLIED_WORDS): a switch whose
/// case keys hold one, or `s is "0x" + rest` (parsed as ==). Before lowering no name is known yet, so a comparison with
/// a sum of a text and a variable loads the word too
pub(crate) fn implied_word(node: &Node) -> Option<&'static str> {
	let is_text_pattern = |part: &Node| TextPattern::of(part, &|_| false).is_some();
	let implied = match node {
		Node::List(items, _, _) => split_switch(items).is_some_and(|(head, _, cases)| match cases.drop_meta() {
			Node::List(arms, Bracket::Curly, _) if is_switch_head(head) => {
				arms.iter().any(|arm| left_spine(arm).any(is_text_pattern))
			}
			_ => false,
		}),
		Node::Key(_, Op::Eq, pattern) => is_text_pattern(pattern),
		_ => false,
	};
	implied.then_some(TEXT_HOLES)
}

/// A key and the left sides below it: an arm `"0x" + rest => rest` and its pattern
fn left_spine(node: &Node) -> impl Iterator<Item = &Node> {
	std::iter::successors(Some(node), |node| match node.drop_meta() {
		Node::Key(left, _, _) => Some(left.as_ref()),
		_ => None,
	})
}

/// A text literal, also of one character, `"v"` (parsed as a character)
fn as_text(part: &Node) -> Option<Node> {
	match part.drop_meta() {
		Node::Text(_) => Some(part.drop_meta().clone()),
		Node::Char(character) => Some(Node::Text(character.to_string())),
		_ => None,
	}
}

/// The parts of a sum in order; `(a + h): T` and `h: T + b` both type the hole h they meet at the colon
fn flattened(node: &Node, parts: &mut Vec<Node>) {
	match node.drop_meta() {
		Node::Key(left, Op::Add, right) => {
			flattened(left, parts);
			flattened(right, parts);
		}
		Node::Key(left, Op::Colon, right) => {
			flattened(left, parts);
			let mut after = vec![];
			flattened(right, &mut after);
			let typed = parts.pop().expect("a colon has a left side");
			let type_word = after.remove(0);
			parts.push(key(typed, Op::Colon, type_word));
			parts.extend(after);
		}
		_ => parts.push(node.clone()),
	}
}

/// A hole and its type: `rest`, `_`, `id: int`; None inside for a typed name no text reads (`x: Point`)
fn hole(part: &Node, is_known: &dyn Fn(&str) -> bool) -> Option<Option<(Node, &'static str)>> {
	match part.drop_meta() {
		Node::Symbol(name) if name == WILDCARD || !is_known(name) => Some(Some((part.drop_meta().clone(), ""))),
		Node::Key(name, Op::Colon, type_node) if name.symbol_name().is_some() => {
			let type_word = type_node.drop_meta().name();
			let known = TEXT_HOLE_TYPES.iter().find(|(word, _)| *word == type_word);
			Some(known.map(|(word, _)| (name.drop_meta().clone(), *word)))
		}
		_ => None,
	}
}

/// `int` of a hole `id: int`: the word converting its text
fn conversion(type_word: &str) -> Option<&'static str> {
	TEXT_HOLE_TYPES.iter().find(|(word, _)| *word == type_word).and_then(|(_, convert)| *convert)
}
