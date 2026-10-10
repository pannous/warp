//! `switch subject {key: body …}` (also `match`): map indexing that executes. Lowered to an if/else chain over the
//! subject, so only the chosen body runs; the `default` (or `_`) key catches the rest, without it a miss is the error
//! `no case for <subject> = <value>`. A list key is a structural pattern (wiki/pattern-matching.md): `["", middle, ""]`
//! matches a list of three whose first and last items are "", binding `middle`; `_` matches any item, lists nest, and a
//! pair `k: v` matches a pair whose key is k. A guard `n if n < 0 => …` binds n to the subject and tests the condition; a relational pattern `> 100 => …` compares it. The shape tests are ordinary type tests (`is_type(x, "list") and count(x) == n`).

use super::nodes::{call, key};
use crate::analyzer::extract_user_functions;
use crate::context::Context;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::cell::Cell;

pub(crate) const SWITCH_WORDS: [&str; 2] = ["switch", "match"];
const DEFAULT_KEYS: [&str; 2] = ["default", WILDCARD];
/// In a pattern: any item, bound to no name
const WILDCARD: &str = "_";
const LIST_SPEC: &str = "list";
const PAIR_SPEC: &str = "pair";
const COUNT_WORD: &str = "count";
const TEXT_WORD: &str = "str";
const SUBJECT_PREFIX: &str = "switch_subject_";
/// Pseudo-call `switch_no_case(label: value)` the emitter turns into the runtime error function `no_case_<label>`,
/// which eval reports as `no case for <label> = <value>`
pub const NO_CASE_CALL: &str = "switch_no_case";
pub const NO_CASE_PREFIX: &str = "no_case_";
const UNNAMED_SUBJECT: &str = "value";

pub fn lower(node: Node) -> Node {
	if !node.mentions_any(&SWITCH_WORDS) {
		return node;
	}
	let mut context = Context::new();
	extract_user_functions(&mut context, &node);
	let words: Vec<&str> = SWITCH_WORDS.into_iter().filter(|word| !context.user_functions.contains_key(*word)).collect();
	if words.is_empty() {
		return node;
	}
	Lowering { words, switches: Cell::new(0) }.expand(node)
}

struct Lowering<'a> {
	words: Vec<&'a str>,
	switches: Cell<usize>,
}

struct Case {
	key: Node,
	body: Node,
}

impl Lowering<'_> {
	fn expand(&self, node: Node) -> Node {
		match node {
			Node::List(items, bracket, separator) => {
				let items: Vec<Node> = items.into_iter().map(|item| self.expand(item)).collect();
				match self.switch(&items) {
					Some(switch) => switch,
					None => Node::List(self.with_switch_arguments(items), bracket, separator),
				}
			}
			Node::Key(left, op, right) => Node::Key(Box::new(self.expand(*left)), op, Box::new(self.expand(*right))),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.expand(*node)), data },
			other => other,
		}
	}

	/// `f(switch 2 {1: 10 2: 20})` arrives as the call `f switch 2 {…}`: the three items one argument
	fn with_switch_arguments(&self, items: Vec<Node>) -> Vec<Node> {
		let Some(start) = (1..items.len().saturating_sub(2)).find(|&start| self.switch(&items[start..start + 3]).is_some()) else { return items };
		let switch = self.switch(&items[start..start + 3]).expect("found");
		let rest = self.with_switch_arguments(items[start + 3..].to_vec());
		items[..start].iter().cloned().chain([switch]).chain(rest).collect()
	}

	/// `switch subject {cases}`; `switch 3 {…} == "three"` arrives as `switch 3 ({…} == "three")`: the switch's value
	/// is the left operand
	fn switch(&self, items: &[Node]) -> Option<Node> {
		let (head, subject, cases) = split_switch(items)?;
		if let Node::Key(..) = cases.drop_meta() {
			let block = leftmost(cases);
			let switch = self.switch(&[head.clone(), subject.clone(), block.clone()])?;
			return Some(with_leftmost(cases.clone(), switch));
		}
		let Node::Symbol(word) = head.drop_meta() else { return None };
		if !self.words.contains(&word.as_str()) {
			return None;
		}
		let cases = cases_of(cases)?;
		let number = self.switches.get();
		self.switches.set(number + 1);
		let subject_name = format!("{SUBJECT_PREFIX}{number}");
		let subject_value = Node::Symbol(subject_name.clone());
		let (default, cases): (Vec<Case>, Vec<Case>) = cases.into_iter().partition(|case| is_default(&case.key));
		let miss = match default.into_iter().next() {
			Some(default) => default.body,
			None => no_case(subject, &subject_value),
		};
		let chain = cases.into_iter().rev().fold(miss, |otherwise, case| {
			let (matches, body) = case_test(&subject_value, case);
			let then = key(key(Node::Empty, Op::If, matches), Op::Then, body);
			key(then, Op::Else, otherwise)
		});
		let bind = key(subject_value, Op::Assign, subject.clone());
		Some(Node::List(vec![bind, chain], Bracket::Round, Separator::Semicolon))
	}
}

/// The word, subject and case block of a switch as the parser groups them: the items [switch, subject, {cases}],
/// the pair [(switch subject), {cases}], or, when the block juxtaposes with a variable subject, [switch, (subject {cases})]
fn split_switch(items: &[Node]) -> Option<(&Node, &Node, &Node)> {
	match items {
		[head, subject, cases] => Some((head, subject, cases)),
		[pair, cases] if matches!(pair.drop_meta(), Node::List(words, _, _) if words.len() == 2) => match pair.drop_meta() {
			Node::List(words, _, _) => Some((&words[0], &words[1], cases)),
			_ => None,
		},
		[head, juxtaposed] => match juxtaposed.drop_meta() {
			Node::List(pair, _, _) if pair.len() == 2 => Some((head, &pair[0], &pair[1])),
			_ => None,
		},
		_ => None,
	}
}

/// The leftmost operand of an operator chain
fn leftmost(node: &Node) -> &Node {
	match node.drop_meta() {
		Node::Key(left, _, _) => leftmost(left),
		_ => node,
	}
}

fn with_leftmost(node: Node, replacement: Node) -> Node {
	match node {
		Node::Key(left, op, right) => Node::Key(Box::new(with_leftmost(*left, replacement)), op, right),
		Node::Meta { node, data } => Node::Meta { node: Box::new(with_leftmost(*node, replacement)), data },
		_ => replacement,
	}
}

fn is_default(key: &Node) -> bool {
	matches!(key.drop_meta(), Node::Symbol(name) if DEFAULT_KEYS.contains(&name.as_str()))
}

/// The item at 1-based `position` of the value at `path`: `subject#2`, `subject#2#1`
fn item(path: &Node, position: usize) -> Node {
	key(path.clone(), Op::Hash, Node::int(position as i64))
}

/// The tests the value at `path` passes when it matches `pattern`, and the assignments of the names the pattern binds
fn pattern_tests(pattern: &Node, path: Node, tests: &mut Vec<Node>, bindings: &mut Vec<Node>) {
	match pattern.drop_meta() {
		Node::List(items, Bracket::Square, _) => {
			tests.push(call(crate::type_tests::IS_TYPE, vec![path.clone(), Node::Text(LIST_SPEC.to_string())]));
			tests.push(key(call(COUNT_WORD, vec![path.clone()]), Op::Eq, Node::int(items.len() as i64)));
			for (index, part) in items.iter().enumerate() {
				pattern_tests(part, item(&path, index + 1), tests, bindings);
			}
		}
		// `a: x` matches the pair whose key is a (a name, never bound) and matches its value against x
		Node::Key(name, Op::Colon, value) => {
			tests.push(call(crate::type_tests::IS_TYPE, vec![path.clone(), Node::Text(PAIR_SPEC.to_string())]));
			tests.push(key(call(TEXT_WORD, vec![item(&path, 1)]), Op::Eq, Node::Text(name.name())));
			pattern_tests(value, item(&path, 2), tests, bindings);
		}
		Node::Symbol(name) if name == WILDCARD => {}
		Node::Symbol(_) => bindings.push(key(pattern.drop_meta().clone(), Op::Assign, path)),
		literal => tests.push(key(path, Op::Eq, literal.clone())),
	}
}

/// A case as its test and its body: a list key is a structural pattern whose names are bound before the body runs; a
/// guard `n if n < 0` binds n to the subject and holds when its condition does
fn case_test(subject: &Node, case: Case) -> (Node, Node) {
	if let Node::Key(empty, op, bound) = case.key.drop_meta() {
		if matches!(empty.drop_meta(), Node::Empty) && op.is_ordering() {
			return (key(subject.clone(), *op, bound.as_ref().clone()), case.body); // `> 100 =>` compares the subject
		}
	}
	let (pattern, guard) = split_guard(&case.key);
	if guard.is_none() && !matches!(pattern.drop_meta(), Node::List(_, Bracket::Square, _)) {
		return (key(subject.clone(), Op::Eq, case.key), case.body);
	}
	let (mut tests, mut bindings) = (vec![], vec![]);
	pattern_tests(&pattern, subject.clone(), &mut tests, &mut bindings);
	if let Some(guard) = guard {
		tests.push(sequence([bindings.clone(), vec![guard]].concat()));
	}
	let test = tests.into_iter().reduce(|all, test| key(all, Op::And, test)).unwrap_or(Node::True);
	let body = if bindings.is_empty() { case.body } else { sequence([bindings, vec![case.body]].concat()) };
	(test, body)
}

/// `n if n < 0` (parsed as `if n < 0 then n`): the pattern n and the guard `n < 0`; any other key has no guard
fn split_guard(case_key: &Node) -> (Node, Option<Node>) {
	match case_key.drop_meta() {
		Node::Key(condition, Op::Then, pattern) => match condition.drop_meta() {
			Node::Key(empty, Op::If, guard) if matches!(empty.drop_meta(), Node::Empty) => (pattern.drop_meta().clone(), Some(guard.as_ref().clone())),
			_ => (case_key.clone(), None),
		},
		_ => (case_key.clone(), None),
	}
}

fn sequence(statements: Vec<Node>) -> Node {
	match <[Node; 1]>::try_from(statements) {
		Ok([single]) => single,
		Err(statements) => Node::List(statements, Bracket::Round, Separator::Semicolon),
	}
}

/// What the error names: a literal or a variable subject as written, anything else just "value"
fn subject_label(subject: &Node) -> String {
	let written = match subject.drop_meta() {
		Node::Number(number) => number.to_string(),
		Node::Text(text) | Node::Symbol(text) => text.clone(),
		Node::Char(letter) => letter.to_string(),
		_ => String::new(),
	};
	let is_name = !written.is_empty() && written.chars().all(|c| c.is_alphanumeric() || c == '_');
	if is_name { written } else { UNNAMED_SUBJECT.to_string() }
}

/// `switch_no_case("label": value)`: the error names the subject as written and its runtime value
fn no_case(subject: &Node, value: &Node) -> Node {
	let label_and_value = key(Node::Text(subject_label(subject)), Op::Colon, value.clone());
	Node::List(vec![Node::Symbol(NO_CASE_CALL.to_string()), label_and_value], Bracket::Round, Separator::None)
}

/// The `key: body` entries of a block
fn cases_of(block: &Node) -> Option<Vec<Case>> {
	let single;
	let items: &[Node] = match block.drop_meta() {
		Node::List(items, Bracket::Curly, _) if !items.is_empty() => items,
		Node::Key(..) => {
			single = [block.clone()];
			&single
		}
		_ => return None,
	};
	items.iter().map(case_of).collect()
}

/// `1: 10` is a Colon key; `1: x=5` parses as `(1:x)=5`, so the colon is found down the left spine of the assignment
fn case_of(item: &Node) -> Option<Case> {
	match item.drop_meta() {
		Node::Key(key, Op::Colon, body) => Some(Case { key: key.as_ref().clone(), body: body.as_ref().clone() }),
		Node::Key(left, op, right) => {
			let Case { key, body } = case_of(left)?;
			Some(Case { key, body: Node::Key(Box::new(body), *op, right.clone()) })
		}
		_ => None,
	}
}
