//! Tree walks (tree-patterns step 5), the pattern matched at any depth of a tree of maps, pairs, lists and code, like
//! XPath `//` and jq `..`:
//! - `tree find pattern` (card tree-find) gives the values of the pattern's one name, a map of its names when it binds
//!   several, or the matching subtrees when it binds none;
//! - `tree replace pattern with replacement` (card tree-replace) rewrites each match, children first, so a rewrite
//!   inside makes its parent match (`n * 1 + 0` simplifies to `n` in one walk).
//!
//! Each use becomes a call of its own recursive walker, defined as source before the passes that lower functions,
//! matches and loops. The pattern is a map, list or code pattern (or a named one, named_patterns.rs): `find(xs, f)` with
//! a function stays the first item f accepts, `replace(text, a, b)` the text word.

use super::blocks::DATA_WORD;
use super::nodes::{call, key, template};
use super::switch::{code_pattern, is_list_pattern, is_map_pattern, pattern_bindings};
use crate::analyzer::TEMPORARY_SEPARATOR;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::cell::RefCell;

pub(crate) const FIND_WORD: &str = "find";
pub(crate) const REPLACE_WORD: &str = "replace";
const WITH_WORD: &str = "with";
const WALK: &str = "WALK";
const PATTERN: &str = "PATTERN";
const RESULT: &str = "RESULT";
/// The walker's own names, apart from the program's: `node·0`, `found·0`, `child·0`
const NODE: &str = "NODE";
const FOUND: &str = "FOUND";
const CHILD: &str = "CHILD";
const BUILT: &str = "BUILT";
const ITEMS: &str = "ITEMS";
const ENTRIES: &str = "ENTRIES";
const NAME: &str = "NAME";
const PLACEHOLDERS: [&str; 8] = [NODE, FOUND, CHILD, BUILT, ITEMS, ENTRIES, NAME, WALK];
/// The walker of one tree find: the matches of the node itself, then those inside its items, values or pair value
const FINDER: &str = "WALK(NODE) := {
  FOUND = []
  match NODE { PATTERN => FOUND += [RESULT]; _ => FOUND }
  if NODE is list then for CHILD in NODE { FOUND += WALK(CHILD) }
  if NODE is map then for CHILD in values(NODE) { FOUND += WALK(CHILD) }
  if NODE is pair then FOUND += WALK(NODE#2)
  FOUND
}";
/// The walker of one tree replace: the node rebuilt of its rewritten items, values or key parts, then itself rewritten
const REPLACER: &str = "WALK(NODE) := {
  BUILT = NODE
  if NODE is list then {
    ITEMS = []
    for CHILD in NODE { ITEMS += [WALK(CHILD)] }
    BUILT = separated(bracketed(ITEMS, NODE.bracket), NODE.separator)
  }
  if NODE is map then {
    ENTRIES = {}
    for NAME in keys(NODE) { ENTRIES[NAME] = WALK(NODE[NAME]) }
    BUILT = ENTRIES
  }
  if NODE is key then BUILT = key(WALK(NODE.key), NODE.op, WALK(NODE.value))
  match BUILT { PATTERN => RESULT; _ => BUILT }
}";

pub fn lower(program: Node) -> Node {
	if !program.mentions_any(&[FIND_WORD, REPLACE_WORD]) {
		return program;
	}
	let walkers = RefCell::new(vec![]);
	let program = with_walker_calls(program, &walkers);
	let walkers = walkers.into_inner();
	if walkers.is_empty() {
		return program;
	}
	let statements = match program {
		Node::List(items, Bracket::None, separator) if separator.separates_statements() => items,
		single => vec![single],
	};
	Node::List(walkers.into_iter().chain(statements).collect(), Bracket::None, Separator::Newline)
}

fn with_walker_calls(node: Node, walkers: &RefCell<Vec<Node>>) -> Node {
	let node = crate::node::children_rewritten(node, |child| with_walker_calls(child, walkers));
	let Node::List(items, bracket, separator) = node else {
		return node;
	};
	let words = flattened(&items);
	let Some((at, length)) = (0..words.len()).find_map(|at| walk_length(&words[at..]).map(|length| (at, length)))
	else {
		return Node::List(items, bracket, separator);
	};
	let items = words;
	let number = walkers.borrow().len();
	let words = &items[at + 1..at + length];
	let name = temporary(words[0].name().as_str(), number);
	walkers.borrow_mut().push(walker(number, words));
	let walked = call(&name, vec![items[at].clone()]);
	if items.len() == length && bracket == Bracket::None {
		return walked;
	}
	let mut items = items;
	items.splice(at..at + length, [walked]);
	with_walker_calls(Node::List(items, bracket, separator), walkers)
}

/// The words of a walk at the start of `words`, its subject included: `tree find P` or `tree replace P with Q`
fn walk_length(words: &[Node]) -> Option<usize> {
	match words {
		[_, word, pattern, ..] if word.is_symbol(FIND_WORD) && is_tree_pattern(pattern) => Some(3),
		[_, word, pattern, with, _, ..]
			if word.is_symbol(REPLACE_WORD) && with.is_symbol(WITH_WORD) && is_tree_pattern(pattern) =>
		{
			Some(5)
		}
		_ => None,
	}
}

/// The words of a walk among the others: `found = xs find [1, x]` parses as `found = [xs, (find [1, x])]`, and in a
/// function body the words come in nested pairs, `[[[[code, replace], (data $a*1)], with], a]`, and `with {…}` may be
/// a group of its own
fn flattened(items: &[Node]) -> Vec<Node> {
	with_quoted_patterns(items.iter().flat_map(spliced).collect())
}

fn spliced(item: &Node) -> Vec<Node> {
	match item.drop_meta() {
		Node::List(words, Bracket::None, _) if holds_walk_word(words) => words.iter().flat_map(spliced).collect(),
		_ => vec![item.clone()],
	}
}

fn holds_walk_word(words: &[Node]) -> bool {
	words.iter().any(|word| {
		is_walk_word(word)
			|| word.is_symbol(WITH_WORD)
			|| matches!(word.drop_meta(), Node::List(inner, Bracket::None, _) if holds_walk_word(inner))
	})
}

fn is_walk_word(word: &Node) -> bool {
	word.is_symbol(FIND_WORD) || word.is_symbol(REPLACE_WORD)
}

/// `replace data ($a * 1) with a`: the words `data` and `($a * 1)` after the walk word are one quoted pattern
fn with_quoted_patterns(words: Vec<Node>) -> Vec<Node> {
	let mut joined: Vec<Node> = vec![];
	let mut words = words.into_iter().peekable();
	while let Some(word) = words.next() {
		let after_walk_word = joined.last().is_some_and(is_walk_word);
		match (words.peek(), word.drop_meta()) {
			// `(data $a*1)`, a group the prefix took
			(_, Node::List(quoted, Bracket::Round, _))
				if after_walk_word && quoted.len() > 1 && quoted[0].is_symbol(DATA_WORD) =>
			{
				joined.push(Node::List(quoted.clone(), Bracket::None, Separator::Space));
			}
			(Some(_), _) if after_walk_word && word.is_symbol(DATA_WORD) => {
				let quoted = words.next().into_iter();
				joined.push(Node::List([word].into_iter().chain(quoted).collect(), Bracket::None, Separator::Space));
			}
			_ => joined.push(word),
		}
	}
	joined
}

fn is_tree_pattern(pattern: &Node) -> bool {
	is_map_pattern(pattern) || is_list_pattern(pattern) || code_pattern(pattern).is_some()
}

fn temporary(name: &str, number: usize) -> String {
	format!("{}{TEMPORARY_SEPARATOR}{number}", name.to_lowercase())
}

/// The walker's definition, `find·0(node·0) := {…}`, of the words `find P` or `replace P with Q`
fn walker(number: usize, words: &[Node]) -> Node {
	let own_names = PLACEHOLDERS.map(|placeholder| {
		let name = if placeholder == WALK { words[0].name() } else { placeholder.to_string() };
		Node::Symbol(temporary(&name, number))
	});
	let (source, pattern, result) = match words {
		[_, pattern, _, replacement] => (REPLACER, pattern, replacement.clone()),
		[_, pattern] => (FINDER, pattern, found_result(pattern, &own_names[0])),
		_ => unreachable!("walk_length gives 3 or 5 words"),
	};
	let mut bindings: Vec<(&str, &Node)> = PLACEHOLDERS.iter().copied().zip(own_names.iter()).collect();
	bindings.extend([(PATTERN, pattern), (RESULT, &result)]);
	let statements = template(source, &bindings);
	match <[Node; 1]>::try_from(statements) {
		Ok([definition]) => definition,
		Err(statements) => Node::List(statements, Bracket::None, Separator::Newline),
	}
}

/// What a find collects of a match: its one name, a map of its names, or the matching node itself
fn found_result(pattern: &Node, node: &Node) -> Node {
	match pattern_bindings(pattern).as_slice() {
		[] => node.clone(),
		[name] => name.clone(),
		names => Node::List(
			names.iter().map(|name| key(name.clone(), Op::Colon, name.clone())).collect(),
			Bracket::Curly,
			Separator::Colon,
		),
	}
}
