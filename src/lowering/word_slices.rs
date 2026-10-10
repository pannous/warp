//! Card word-slices (user 2026-10-09, notes/decisions.md): list slices in words, by position like `xs#1` (first = 1),
//! both ends inclusive: `xs from #2 to #4`, `xs starting from second`, `xs up to 2nd`, `xs from (n)th to last`.
//! A position is marked, `#n` or an ordinal; a bare number (`xs up to 2`) may mean the item 2, so it is a loud error
//! naming both forms. The slice is the one `xs#(2…4)` makes, `slice(xs, start, end)` 0-based and end-exclusive.
//! The parser nests the words (`xs (to (# from 2) (# ø 4))`): they are read flat again, `xs from #2 to #4`.
//! Only a variable of the program or a list literal is sliced, so `count from 1 to 10` and a phrase call
//! `move x from a to b` keep their meaning.

use super::nodes::symbol;
use crate::diagnostic::Diagnostic;
use crate::extensions::numbers::Number;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const FROM: &str = "from";
const TO: &str = "to";
const UP: &str = "up";
const STARTING: &str = "starting";
/// `xs from #2 to last`: the end of the list
const LAST: &str = "last";
/// `xs up to nth`: the position n
const NTH: (&str, &str) = ("nth", "n");
const ORDINAL_WORDS: [&str; 10] = ["first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth", "tenth"];
const SLICE: &str = "slice";

/// A bound of a word slice
enum Bound {
	/// a marked 1-based position
	Position(Node),
	/// `last`, or no bound written: the end (or start) of the list
	Open,
	/// a bare number, which may be a value
	Bare(Number),
}

pub fn lower(program: Node) -> Node {
	// `to` is the operator, no word
	if !program.mentions_any(&[FROM, UP]) {
		return program;
	}
	slices(program.clone(), &program)
}

fn slices(node: Node, program: &Node) -> Node {
	let mut words = vec![];
	match node.drop_meta() {
		// `print(xs up to #3)`, `f(xs up to #3)`: the words inside a group or call
		Node::List(items, bracket, _) if *bracket != Bracket::Square => items.iter().for_each(|item| flat_words(item, &mut words)),
		_ => flat_words(&node, &mut words),
	}
	match word_slice(&words, program).or_else(|| sliced_argument(&words, program)) {
		Some(slice) => slice,
		None => node.map_children(|child| slices(child, program)),
	}
}

/// `[f, xs, up, to, #3]`: the call of f on the slice
fn sliced_argument(words: &[Node], program: &Node) -> Option<Node> {
	let (callee @ Node::Symbol(_), argument) = words.split_first()? else { return None };
	let slice = word_slice(argument, program)?;
	Some(Node::List(vec![callee.clone(), slice], Bracket::Round, Separator::None))
}

/// `xs (to (# from 2) (# ø 4))` read flat: `[xs, from, #2, to, #4]`
fn flat_words(node: &Node, words: &mut Vec<Node>) {
	match node.drop_meta() {
		Node::Key(left, Op::To, right) => {
			flat_words(left, words);
			words.push(symbol(TO));
			flat_words(right, words);
		}
		// `from #2` parses as the index `from#2`
		Node::Key(word, Op::Hash, position) if is_word(word, &[FROM, UP, TO, STARTING]) => {
			words.push(word.as_ref().clone());
			words.push(Node::Key(Box::new(Node::Empty), Op::Hash, position.clone()));
		}
		Node::List(items, Bracket::None, Separator::Space) => items.iter().for_each(|item| flat_words(item, words)),
		_ => words.push(node.clone()),
	}
}

/// `[xs, from, P, to, Q]`, `[xs, starting, from, P]`, `[xs, up, to, Q]`: the slice of xs, or the error of a bare number
fn word_slice(words: &[Node], program: &Node) -> Option<Node> {
	let (receiver, rest) = words.split_first()?;
	if !is_sliceable(receiver, program) {
		return None;
	}
	let at = |index: usize, word: &str| rest.get(index).is_some_and(|node| is_word(node, &[word]));
	let (start, end) = if at(0, UP) && at(1, TO) {
		(Bound::Open, only_bound(&rest[2..])?)
	} else {
		let from_at = if at(0, STARTING) { 1 } else { 0 };
		if !at(from_at, FROM) {
			return None;
		}
		let rest = &rest[from_at + 1..];
		let (start, used) = bound(rest)?;
		if matches!(start, Bound::Open) {
			return None;
		}
		match rest.get(used) {
			None => (start, Bound::Open),
			Some(word) if is_word(word, &[TO]) => (start, only_bound(&rest[used + 1..])?),
			Some(_) => return None,
		}
	};
	if matches!(start, Bound::Bare(_)) || matches!(end, Bound::Bare(_)) {
		return Some(bare_number_error(words, &start, &end));
	}
	let start = match start {
		Bound::Position(position) => minus_one(position),
		_ => Node::Empty,
	};
	let end = match end {
		Bound::Position(position) => position,
		_ => Node::Empty,
	};
	Some(Node::List(vec![symbol(SLICE), receiver.clone(), start, end], Bracket::Round, Separator::None))
}

/// A variable the program names or a list literal; a word of the language (`count from 1 to 10`) is no list
fn is_sliceable(receiver: &Node, program: &Node) -> bool {
	match receiver.drop_meta() {
		Node::Symbol(name) => crate::soft_keywords::program_names(program, name),
		Node::List(_, Bracket::Square, _) | Node::Text(_) => true,
		_ => false,
	}
}

/// The bound that is all of `words`
fn only_bound(words: &[Node]) -> Option<Bound> {
	let (found, used) = bound(words)?;
	(used == words.len()).then_some(found)
}

/// The bound at the start of `words` and how many words it takes: `#2`, `second`, `2nd`, `(n)th`, `nth`, `last`, `2`
fn bound(words: &[Node]) -> Option<(Bound, usize)> {
	let first = words.first()?;
	let ordinal_suffix = words.get(1).is_some_and(|word| is_word(word, &crate::warp_parser::ORDINAL_SUFFIXES));
	match first.drop_meta() {
		Node::Key(empty, Op::Hash, position) if matches!(empty.drop_meta(), Node::Empty) => Some((Bound::Position(position.as_ref().clone()), 1)),
		Node::Number(_) | Node::List(_, Bracket::Round, _) if ordinal_suffix => Some((Bound::Position(first.clone()), 2)),
		Node::Number(number) => Some((Bound::Bare(*number), 1)),
		Node::Symbol(word) if word == LAST => Some((Bound::Open, 1)),
		Node::Symbol(word) if word == NTH.0 => Some((Bound::Position(symbol(NTH.1)), 1)),
		Node::Symbol(word) => ORDINAL_WORDS.iter().position(|ordinal| ordinal == word)
			.map(|index| (Bound::Position(Node::int(index as i64 + 1)), 1)),
		_ => None,
	}
}

/// `xs up to 2`: "a position: xs up to #2 or xs up to second"
fn bare_number_error(words: &[Node], start: &Bound, end: &Bound) -> Node {
	let written = |marked: &dyn Fn(&Number) -> String| {
		let mut bounds = [start, end].into_iter().filter(|bound| matches!(bound, Bound::Bare(_) | Bound::Position(_)));
		words.iter().map(|word| match word.drop_meta() {
			Node::Number(_) => match bounds.next() {
				Some(Bound::Bare(number)) => marked(number),
				_ => word.serialize(),
			},
			_ => word.serialize(),
		}).collect::<Vec<_>>().join(" ")
	};
	let hashed = written(&|number| format!("#{number}"));
	let ordinal = written(&ordinal_word);
	let message = format!("a position: {hashed} or {ordinal} (a bare number may be an item of the list)");
	Diagnostic::at(&words[0], message).into_error()
}

/// `second` for 2, `12th` for 12
fn ordinal_word(number: &Number) -> String {
	let position = f64::from(*number) as usize;
	match ORDINAL_WORDS.get(position.wrapping_sub(1)) {
		Some(word) => word.to_string(),
		None => format!("{position}th"),
	}
}

fn minus_one(position: Node) -> Node {
	match position.drop_meta() {
		Node::Number(number) => Node::Number(*number + Number::Int(-1)),
		_ => Node::Key(Box::new(position), Op::Sub, Box::new(Node::int(1))),
	}
}

fn is_word(node: &Node, words: &[&str]) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if words.contains(&word.as_str()))
}

