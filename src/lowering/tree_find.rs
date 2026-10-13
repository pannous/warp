//! `tree find pattern` (card tree-find, tree-patterns step 5): the pattern matched at any depth of a tree of maps, pairs
//! and lists, like XPath `//` and jq `..`. It gives the values of the pattern's one name, a map of its names when it binds
//! several, or the matching subtrees when it binds none. Each use becomes a call of its own recursive walker, defined as
//! source before the passes that lower functions, matches and loops. `find` with a function (`find(xs, f)`) stays the
//! first item f accepts: only a map or list pattern (or a named one, named_patterns.rs) makes a tree find.

use super::generator_consumers::template;
use super::nodes::call;
use super::switch::{bound_names, is_list_pattern, is_map_pattern};
use crate::analyzer::TEMPORARY_SEPARATOR;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::cell::RefCell;

pub(crate) const FIND_WORD: &str = "find";
const WALK: &str = "WALK";
const PATTERN: &str = "PATTERN";
const RESULT: &str = "RESULT";
/// The walker's own names, apart from the program's: `node·0`, `found·0`, `child·0`
const NODE: &str = "NODE";
const FOUND: &str = "FOUND";
const CHILD: &str = "CHILD";
/// The walker of one tree find: the matches of the node itself, then those inside its items, values or pair value
const WALKER: &str = "WALK(NODE) := {
  FOUND = []
  match NODE { PATTERN => FOUND += [RESULT]; _ => FOUND }
  if NODE is list then for CHILD in NODE { FOUND += WALK(CHILD) }
  if NODE is map then for CHILD in values(NODE) { FOUND += WALK(CHILD) }
  if NODE is pair then FOUND += WALK(NODE#2)
  FOUND
}";

pub fn lower(program: Node) -> Node {
	if !program.mentions_any(&[FIND_WORD]) {
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
	Node::List(
		walkers.into_iter().chain(statements).collect(),
		Bracket::None,
		Separator::Newline,
	)
}

fn with_walker_calls(node: Node, walkers: &RefCell<Vec<Node>>) -> Node {
	let node = crate::node::children_rewritten(node, |child| with_walker_calls(child, walkers));
	let Node::List(items, bracket, separator) = node else {
		return node;
	};
	let items = flattened(items);
	let Some(at) = items.windows(3).position(|window| is_tree_find(&window[1], &window[2])) else {
		return Node::List(items, bracket, separator);
	};
	let number = walkers.borrow().len();
	let name = temporary(FIND_WORD, number);
	let walker = walker(number, &items[at + 2]);
	walkers.borrow_mut().push(walker);
	let found = call(&name, vec![items[at].clone()]);
	if items.len() == 3 && bracket == Bracket::None {
		return found;
	}
	let mut items = items;
	items.splice(at..at + 3, [found]);
	with_walker_calls(Node::List(items, bracket, separator), walkers)
}

/// `found = xs find [1, x]` parses as `found = [xs, (find [1, x])]`: the find's words among the others
fn flattened(items: Vec<Node>) -> Vec<Node> {
	items
		.into_iter()
		.flat_map(|item| match item.drop_meta() {
			Node::List(words, Bracket::None, _) if words.len() == 2 && is_tree_find(&words[0], &words[1]) => {
				words.clone()
			}
			_ => vec![item],
		})
		.collect()
}

fn is_tree_find(word: &Node, pattern: &Node) -> bool {
	word.is_symbol(FIND_WORD) && (is_map_pattern(pattern) || is_list_pattern(pattern))
}

fn temporary(name: &str, number: usize) -> String {
	format!("{}{TEMPORARY_SEPARATOR}{number}", name.to_lowercase())
}

/// The walker's definition: `find·0(node·0) := {…}`
fn walker(number: usize, pattern: &Node) -> Node {
	let [walk, node, found, child] = [FIND_WORD, NODE, FOUND, CHILD].map(|name| Node::Symbol(temporary(name, number)));
	let result = match bound_names(pattern).as_slice() {
		[] => node.clone(),
		[name] => name.clone(),
		names => Node::List(
			names
				.iter()
				.map(|name| Node::Key(Box::new(name.clone()), Op::Colon, Box::new(name.clone())))
				.collect(),
			Bracket::Curly,
			Separator::Colon,
		),
	};
	let statements = template(
		WALKER,
		&[
			(WALK, &walk),
			(NODE, &node),
			(FOUND, &found),
			(CHILD, &child),
			(PATTERN, pattern),
			(RESULT, &result),
		],
	);
	match <[Node; 1]>::try_from(statements) {
		Ok([definition]) => definition,
		Err(statements) => Node::List(statements, Bracket::None, Separator::Newline),
	}
}
