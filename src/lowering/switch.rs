//! `switch subject {key: body …}` (also `match`): map indexing that executes. Lowered to an if/else chain over the
//! subject, so only the chosen body runs; the `default` (or `_`) key catches the rest, without it a miss is the error
//! `no case for <subject> = <value>`. A list key is a structural pattern (wiki/pattern-matching.md): `["", middle, ""]`
//! matches a list of three whose first and last items are "", binding `middle`; `_` matches any item, lists nest, and a
//! pair `k: v` matches a pair whose key is k. A rest `[first, ...rest]` (`*rest`, `rest...`, unnamed `..`) matches a list of at least the other items,
//! binding the rest as a slice; items after it count from the end. A guard `n if n < 0 => …` binds n to the subject and tests the condition; a relational pattern `> 100 => …` compares it. The shape tests are ordinary type tests (`is_type(x, "list") and count(x) == n`).

use super::nodes::{call, children_rewritten, if_then, key, Counter};
use super::words::COUNT_WORD;
use crate::library_words::SLICE;
use crate::node::{error, text, Bracket, Node, Separator};
use crate::operators::Op;
use crate::tuples::STARRED;

pub(crate) const SWITCH_WORDS: [&str; 2] = ["switch", "match"];
const DEFAULT_KEYS: [&str; 2] = ["default", WILDCARD];
/// In a pattern: any item, bound to no name
const WILDCARD: &str = "_";
const LIST_SPEC: &str = "list";
const PAIR_SPEC: &str = "pair";
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
	let context = crate::analyzer::function_definitions(&node);
	let words: Vec<&str> = SWITCH_WORDS
		.into_iter()
		.filter(|word| !context.user_functions.contains_key(*word))
		.collect();
	if words.is_empty() {
		return node;
	}
	Lowering {
		words,
		switches: Counter::default(),
	}
	.expand(node)
}

struct Lowering<'a> {
	words: Vec<&'a str>,
	switches: Counter,
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
			other => children_rewritten(other, |child| self.expand(child)),
		}
	}

	/// `f(switch 2 {1: 10 2: 20})` arrives as the call `f switch 2 {…}`: the three items one argument
	fn with_switch_arguments(&self, items: Vec<Node>) -> Vec<Node> {
		let Some(start) =
			(1..items.len().saturating_sub(2)).find(|&start| self.switch(&items[start..start + 3]).is_some())
		else {
			return items;
		};
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
		let Node::Symbol(word) = head.drop_meta() else {
			return None;
		};
		if !self.words.contains(&word.as_str()) {
			return None;
		}
		let cases = cases_of(cases)?;
		let number = self.switches.next_number();
		let subject_name = format!("{SUBJECT_PREFIX}{number}");
		let subject_value = Node::Symbol(subject_name.clone());
		let (default, cases): (Vec<Case>, Vec<Case>) = cases.into_iter().partition(|case| is_default(&case.key));
		let miss = match default.into_iter().next() {
			Some(default) => default.body,
			None => no_case(subject, &subject_value),
		};
		let chain = cases.into_iter().rev().fold(miss, |otherwise, case| {
			let (matches, body) = case_test(&subject_value, case);
			let then = if_then(matches, body);
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
		[pair, cases] if matches!(pair.drop_meta(), Node::List(words, _, _) if words.len() == 2) => {
			match pair.drop_meta() {
				Node::List(words, _, _) => Some((&words[0], &words[1], cases)),
				_ => None,
			}
		}
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
		Node::Meta { node, data } => Node::Meta {
			node: Box::new(with_leftmost(*node, replacement)),
			data,
		},
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

/// A rest in a pattern, with its name: `...rest` and `*rest` (parsed alike), `rest...`; `..` and `...` unnamed
pub(crate) fn rest_pattern(part: &Node) -> Option<Option<String>> {
	match part.drop_meta() {
		Node::Symbol(name) => name
			.strip_prefix(STARRED)
			.filter(|rest| !rest.is_empty())
			.map(|rest| Some(rest.to_string())),
		Node::Key(start, Op::Range | Op::To, end) if matches!(end.drop_meta(), Node::Empty) => {
			match start.drop_meta() {
				Node::Empty => Some(None),
				Node::Symbol(name) => Some(Some(name.clone())),
				_ => None,
			}
		}
		_ => None,
	}
}

/// The tests the value at `path` passes when it matches `pattern`, and the assignments of the names the pattern binds
fn pattern_tests(pattern: &Node, path: Node, tests: &mut Vec<Node>, bindings: &mut Vec<Node>) {
	match pattern.drop_meta() {
		Node::List(items, Bracket::Square, _) => {
			tests.push(call(crate::type_tests::IS_TYPE, vec![path.clone(), text(LIST_SPEC)]));
			let Some(rest_index) = items.iter().position(|part| rest_pattern(part).is_some()) else {
				tests.push(key(
					call(COUNT_WORD, vec![path.clone()]),
					Op::Eq,
					Node::int(items.len() as i64),
				));
				for (index, part) in items.iter().enumerate() {
					pattern_tests(part, item(&path, index + 1), tests, bindings);
				}
				return;
			};
			// `[first, ...middle, last]`: the items before the rest count from the start, those after it from the end
			let fixed = items.len() - 1;
			let after = fixed - rest_index;
			let count = call(COUNT_WORD, vec![path.clone()]);
			tests.push(key(count.clone(), Op::Ge, Node::int(fixed as i64)));
			let from_end = |distance: usize| match distance {
				0 => count.clone(),
				_ => key(count.clone(), Op::Sub, Node::int(distance as i64)),
			};
			for (index, part) in items.iter().enumerate() {
				match index.cmp(&rest_index) {
					std::cmp::Ordering::Less => pattern_tests(part, item(&path, index + 1), tests, bindings),
					std::cmp::Ordering::Greater => pattern_tests(
						part,
						key(path.clone(), Op::Hash, from_end(items.len() - 1 - index)),
						tests,
						bindings,
					),
					std::cmp::Ordering::Equal => {
						if let Some(Some(name)) = rest_pattern(part) {
							let rest = call(SLICE, vec![path.clone(), Node::int(rest_index as i64), from_end(after)]);
							bindings.push(key(Node::Symbol(name), Op::Assign, rest));
						}
					}
				}
			}
		}
		// `a: x` matches the pair whose key is a (a name, never bound) and matches its value against x
		Node::Key(name, Op::Colon, value) => {
			tests.push(call(crate::type_tests::IS_TYPE, vec![path.clone(), text(PAIR_SPEC)]));
			tests.push(key(
				call(TEXT_WORD, vec![item(&path, 1)]),
				Op::Eq,
				Node::Text(name.name()),
			));
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
	let alternatives = alternatives_of(&pattern);
	if alternatives.len() > 1 {
		return alternatives_test(subject, &alternatives, guard, case.body);
	}
	if guard.is_none() && !matches!(pattern.drop_meta(), Node::List(_, Bracket::Square, _)) {
		return (key(subject.clone(), Op::Eq, case.key), case.body);
	}
	let (mut tests, mut bindings) = (vec![], vec![]);
	pattern_tests(&pattern, subject.clone(), &mut tests, &mut bindings);
	if let Some(guard) = guard {
		tests.push(sequence([bindings.clone(), vec![guard]].concat()));
	}
	let test = joined(tests, Op::And);
	let body = if bindings.is_empty() {
		case.body
	} else {
		sequence([bindings, vec![case.body]].concat())
	};
	(test, body)
}

/// A case key is a pattern, so `1 | 2` and `1 or 2` (both parsed as or) are its alternatives, never the logical value;
/// a computed comparison goes in a guard (user, matching roadmap)
fn alternatives_of(pattern: &Node) -> Vec<Node> {
	match pattern.drop_meta() {
		Node::Key(left, Op::Or, right) => [alternatives_of(left), alternatives_of(right)].concat(),
		_ => vec![pattern.clone()],
	}
}

/// The case matches when one alternative does; a list alternative tests its shape, any other compares the subject
fn alternatives_test(subject: &Node, alternatives: &[Node], guard: Option<Node>, body: Node) -> (Node, Node) {
	let mut matches = vec![];
	for alternative in alternatives {
		if !matches!(alternative.drop_meta(), Node::List(_, Bracket::Square, _)) {
			matches.push(key(subject.clone(), Op::Eq, alternative.clone()));
			continue;
		}
		let (mut tests, mut bindings) = (vec![], vec![]);
		pattern_tests(alternative, subject.clone(), &mut tests, &mut bindings);
		if !bindings.is_empty() {
			let message = "alternatives bind no names: write one case per pattern, or `_` for an item";
			return (error(message), body);
		}
		matches.push(joined(tests, Op::And));
	}
	let test = joined(matches, Op::Or);
	match guard {
		Some(guard) => (key(test, Op::And, guard), body),
		None => (test, body),
	}
}

/// The tests joined by `and` (true without any) or by `or`
fn joined(tests: Vec<Node>, op: Op) -> Node {
	tests
		.into_iter()
		.reduce(|all, test| key(all, op, test))
		.unwrap_or(Node::True)
}

/// `n if n < 0` (parsed as `if n < 0 then n`): the pattern n and the guard `n < 0`; any other key has no guard
fn split_guard(case_key: &Node) -> (Node, Option<Node>) {
	match case_key.drop_meta() {
		Node::Key(condition, Op::Then, pattern) => match condition.drop_meta() {
			Node::Key(empty, Op::If, guard) if matches!(empty.drop_meta(), Node::Empty) => {
				(pattern.drop_meta().clone(), Some(guard.as_ref().clone()))
			}
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
	if is_name {
		written
	} else {
		UNNAMED_SUBJECT.to_string()
	}
}

/// `switch_no_case("label": value)`: the error names the subject as written and its runtime value
fn no_case(subject: &Node, value: &Node) -> Node {
	let label_and_value = key(Node::Text(subject_label(subject)), Op::Colon, value.clone());
	call(NO_CASE_CALL, vec![label_and_value])
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
	// `1, 2 => …` parses as the items 1 and `2 => …`: the plain items before a case are its alternatives
	let mut cases = vec![];
	let mut alternatives: Vec<Node> = vec![];
	for item in items.iter().flat_map(comma_items) {
		match case_of(&item) {
			Some(case) => {
				let key = alternatives
					.drain(..)
					.rev()
					.fold(case.key, |later, earlier| key(earlier, Op::Or, later));
				cases.push(Case { key, body: case.body });
			}
			None => alternatives.push(item),
		}
	}
	alternatives.is_empty().then_some(cases)
}

/// The items of a comma list inside a case block, else the item itself
fn comma_items(item: &Node) -> Vec<Node> {
	match item.drop_meta() {
		Node::List(items, Bracket::None, Separator::Colon) => items.clone(),
		_ => vec![item.clone()],
	}
}

/// `1: 10` is a Colon key; `1: x=5` parses as `(1:x)=5`, so the colon is found down the left spine of the assignment
fn case_of(item: &Node) -> Option<Case> {
	match item.drop_meta() {
		Node::Key(key, Op::Colon, body) => Some(Case {
			key: key.as_ref().clone(),
			body: body.as_ref().clone(),
		}),
		Node::Key(left, op, right) => {
			let Case { key, body } = case_of(left)?;
			Some(Case {
				key,
				body: Node::Key(Box::new(body), *op, right.clone()),
			})
		}
		_ => None,
	}
}
