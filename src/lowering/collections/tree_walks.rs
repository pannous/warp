//! Tree walks (tree-patterns step 5), the pattern matched at any depth of a tree of maps, pairs, lists and code, like
//! XPath `//` and jq `..`:
//! - `tree find pattern` (card tree-find) gives the values of the pattern's one name, a map of its names when it binds
//!   several, or the matching subtrees when it binds none;
//! - `tree replace pattern with replacement` (card tree-replace) rewrites each match, children first, so a rewrite
//!   inside makes its parent match (`n * 1 + 0` simplifies to `n` in one walk);
//! - `simplify = rules { data($a * 1) => a; data($a + 0) => a }` (card rewrite-rules) defines `simplify(code)`, the
//!   walk of a whole rule set: each node rewritten by its first matching rule until none fits, so a rewrite that makes
//!   a new match is rewritten again (a normal form, as in term rewriting). A rule set that never settles stops after
//!   STEP_LIMIT rewrites in a row with an error naming the rule that keeps firing (user, decision rule-sets).
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
pub(crate) const RULES_WORD: &str = "rules";
const WITH_WORD: &str = "with";
const WALK: &str = "WALK";
const PATTERN: &str = "PATTERN";
const RESULT: &str = "RESULT";
const ARMS: &str = "ARMS";
const MESSAGE: &str = "MESSAGE";
const LIMIT: &str = "LIMIT";
/// Rewrites in a row of one node before a rule set counts as never settling
const STEP_LIMIT: i64 = 1000;
/// The walker's own names, apart from the program's: `node·0`, `found·0`, `child·0`
const NODE: &str = "NODE";
const FOUND: &str = "FOUND";
const CHILD: &str = "CHILD";
const BUILT: &str = "BUILT";
const ITEMS: &str = "ITEMS";
const ENTRIES: &str = "ENTRIES";
const NAME: &str = "NAME";
const STEPS: &str = "STEPS";
const REWRITTEN: &str = "REWRITTEN";
const PLACEHOLDERS: [&str; 10] = [NODE, FOUND, CHILD, BUILT, ITEMS, ENTRIES, NAME, STEPS, REWRITTEN, WALK];
/// The walker of one tree find: the matches of the node itself, then those inside its items, values or pair value
const FINDER: &str = "WALK(NODE) := {
  FOUND = []
  match NODE { PATTERN => FOUND += [RESULT]; _ => FOUND }
  if NODE is list then for CHILD in NODE { FOUND += WALK(CHILD) }
  if NODE is map then for CHILD in values(NODE) { FOUND += WALK(CHILD) }
  if NODE is pair then FOUND += WALK(NODE#2)
  FOUND
}";
/// The walker of one tree replace or rule set: the node rebuilt of its rewritten items, values or key parts, then
/// itself rewritten by the arms
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
  match BUILT { ARMS => ARMS }
}";

/// The rewrites in a row a rule set's walker is in, its counter
const STEP_COUNTER: &str = "global STEPS = 0";
/// A rule's result walked again, the rewrites in a row counted
const REWRITE_STEP: &str = "STEPS += 1
if STEPS > LIMIT then raise MESSAGE
REWRITTEN = WALK(RESULT)
STEPS -= 1
REWRITTEN";

pub fn lower(program: Node) -> Node {
	if !program.mentions_any(&[FIND_WORD, REPLACE_WORD]) {
		return program;
	}
	let walkers = RefCell::new(vec![]);
	let program = with_walker_calls(program, &walkers);
	with_walkers(program, walkers.into_inner())
}

/// Rule sets as their walkers, before any pass reads their arms `data(…) => …` as definitions or lambdas
pub fn lower_rule_sets(program: Node) -> Node {
	if !program.mentions_any(&[RULES_WORD]) {
		return program;
	}
	let walkers = RefCell::new(vec![]);
	let program = with_rule_sets(program, &walkers);
	with_walkers(program, walkers.into_inner())
}

/// The program after the walkers' definitions
fn with_walkers(program: Node, walkers: Vec<Node>) -> Node {
	if walkers.is_empty() {
		return program;
	}
	let statements = match program {
		Node::List(items, Bracket::None, separator) if separator.separates_statements() => items,
		single => vec![single],
	};
	Node::List(walkers.into_iter().chain(statements).collect(), Bracket::None, Separator::Newline)
}

/// `simplify = rules {…}` (or `:=`) defines the walker simplify; a rule set anywhere else is a function of the node
fn with_rule_sets(node: Node, walkers: &RefCell<Vec<Node>>) -> Node {
	let node = match node {
		Node::List(items, bracket, separator) if separator.separates_statements() => {
			let kept = items.into_iter().filter(|statement| !defines_rule_set(statement, walkers)).collect();
			Node::List(kept, bracket, separator)
		}
		other => other,
	};
	let node = crate::node::children_rewritten(node, |child| with_rule_sets(child, walkers));
	let Some(rules) = rule_set(&node) else {
		return node;
	};
	let number = walkers.borrow().len();
	let name = temporary(RULES_WORD, number);
	walkers.borrow_mut().extend(rule_set_walker(&name, number, &rules));
	let parameter = Node::Symbol(temporary(NODE, number));
	key(parameter.clone(), Op::Arrow, call(&name, vec![parameter]))
}

/// Whether `statement` is `name = rules {…}`, whose walker it adds
fn defines_rule_set(statement: &Node, walkers: &RefCell<Vec<Node>>) -> bool {
	let Node::Key(name, Op::Assign | Op::Define, value) = statement.drop_meta() else {
		return false;
	};
	let (Node::Symbol(name), Some(rules)) = (name.drop_meta(), rule_set(value)) else {
		return false;
	};
	let number = walkers.borrow().len();
	walkers.borrow_mut().extend(rule_set_walker(name, number, &rules));
	true
}

/// The rules of `rules {…}`
fn rule_set(node: &Node) -> Option<Vec<Node>> {
	let Node::List(items, Bracket::None, Separator::Space) = node.drop_meta() else {
		return None;
	};
	let [word, block] = items.as_slice() else {
		return None;
	};
	let Node::List(rules, Bracket::Curly, _) = block.drop_meta() else {
		return None;
	};
	word.is_symbol(RULES_WORD).then(|| block.body_statements()).filter(|_| !rules.is_empty())
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
	let walk = temporary(words[0].name().as_str(), number);
	let own_names = own_names(&walk, number);
	match words {
		[_, pattern, _, replacement] => {
			replacer(&own_names, vec![key(pattern.clone(), Op::Arrow, replacement.clone())])
		}
		[_, pattern] => {
			filled(FINDER, &own_names, &[(PATTERN, pattern), (RESULT, &found_result(pattern, &own_names[0].1))])
		}
		_ => unreachable!("walk_length gives 3 or 5 words"),
	}
}

/// The walker `walk(node) := {…}` of a rule set and its step counter, each rule's result walked again
fn rule_set_walker(walk: &str, number: usize, rules: &[Node]) -> Vec<Node> {
	let own_names = own_names(walk, number);
	let limit = Node::int(STEP_LIMIT);
	let walked = |rule: &Node, result: &Node| {
		let message = format!(
			"the rule set {walk} never settles: {} keeps firing ({STEP_LIMIT} rewrites in a row)",
			rule.serialize()
		);
		let bindings = [(RESULT, result), (LIMIT, &limit), (MESSAGE, &Node::text(&message))];
		let names = own_names.iter().map(|(placeholder, name)| (*placeholder, name));
		Node::List(
			template(REWRITE_STEP, &names.chain(bindings).collect::<Vec<_>>()),
			Bracket::Curly,
			Separator::Newline,
		)
	};
	let arms = rules.iter().map(|rule| match rule.drop_meta() {
		Node::Key(pattern, op @ (Op::Arrow | Op::FatArrow | Op::Colon), result) => {
			key(*pattern.clone(), *op, walked(rule, result))
		}
		other => other.clone(),
	});
	let arms = arms.collect();
	vec![filled(STEP_COUNTER, &own_names, &[]), replacer(&own_names, arms)]
}

/// The replacing walker whose match holds `arms`, then `_ => BUILT`
fn replacer(own_names: &[(&str, Node)], mut arms: Vec<Node>) -> Node {
	let built = &own_names.iter().find(|(placeholder, _)| *placeholder == BUILT).expect("BUILT is a placeholder").1;
	arms.push(key(Node::symbol("_"), Op::Arrow, built.clone()));
	crate::welcome_forms::lower_match_arms(with_arms(filled(REPLACER, own_names, &[]), &arms))
}

/// The match block `{ ARMS => ARMS }` of a walker's source holding `arms`
fn with_arms(node: Node, arms: &[Node]) -> Node {
	match node {
		Node::List(items, Bracket::Curly, _) if matches!(items.as_slice(), [only] if is_arms_placeholder(only)) => {
			Node::List(arms.to_vec(), Bracket::Curly, Separator::Newline)
		}
		other => crate::node::children_rewritten(other, |child| with_arms(child, arms)),
	}
}

fn is_arms_placeholder(arm: &Node) -> bool {
	matches!(arm.drop_meta(), Node::Key(pattern, _, _) if pattern.is_symbol(ARMS))
}

/// The walker's own names, `node·0` …, its own name `walk`
fn own_names(walk: &str, number: usize) -> Vec<(&'static str, Node)> {
	let name = |placeholder: &str| if placeholder == WALK { walk.to_string() } else { temporary(placeholder, number) };
	PLACEHOLDERS.iter().map(|placeholder| (*placeholder, Node::Symbol(name(placeholder)))).collect()
}

/// The walker's source with its own names and `bindings` filled in, as one definition
fn filled(source: &str, own_names: &[(&str, Node)], bindings: &[(&str, &Node)]) -> Node {
	let names = own_names.iter().map(|(placeholder, name)| (*placeholder, name));
	let bindings: Vec<(&str, &Node)> = names.chain(bindings.iter().copied()).collect();
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
			Separator::Comma,
		),
	}
}
