//! `switch subject {key: body …}` (also `match`): map indexing that executes. Lowered to an if/else chain over the
//! subject, so only the chosen body runs; the `default` key catches the rest, without it a miss is the error `no case for <subject>`.

use crate::analyzer::extract_user_functions;
use crate::context::Context;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::cell::Cell;

const SWITCH_WORDS: [&str; 2] = ["switch", "match"];
const DEFAULT_KEY: &str = "default";
const SUBJECT_PREFIX: &str = "switch_subject_";
/// Pseudo-call `switch_no_case(label)` the emitter turns into the runtime error function `no_case_<label>`,
/// which eval reports as `no case for <label>`
pub const NO_CASE_CALL: &str = "switch_no_case";
pub const NO_CASE_PREFIX: &str = "no_case_";
const UNNAMED_SUBJECT: &str = "value";

pub fn lower(node: Node) -> Node {
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
				self.switch(&items).unwrap_or(Node::List(items, bracket, separator))
			}
			Node::Key(left, op, right) => Node::Key(Box::new(self.expand(*left)), op, Box::new(self.expand(*right))),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.expand(*node)), data },
			other => other,
		}
	}

	/// `switch subject {cases}`
	fn switch(&self, items: &[Node]) -> Option<Node> {
		let (head, subject, cases) = split_switch(items)?;
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
			None => no_case(subject),
		};
		let chain = cases.into_iter().rev().fold(miss, |otherwise, case| {
			let matches = key(subject_value.clone(), Op::Eq, case.key);
			let then = key(key(Node::Empty, Op::If, matches), Op::Then, case.body);
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

fn key(left: Node, op: Op, right: Node) -> Node {
	Node::Key(Box::new(left), op, Box::new(right))
}

fn is_default(key: &Node) -> bool {
	matches!(key.drop_meta(), Node::Symbol(name) if name == DEFAULT_KEY)
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

fn no_case(subject: &Node) -> Node {
	Node::List(vec![Node::Symbol(NO_CASE_CALL.to_string()), Node::Text(subject_label(subject))], Bracket::Round, Separator::None)
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
