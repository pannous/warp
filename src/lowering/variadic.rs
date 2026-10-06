//! Rest parameters and spread arguments, resolved at compile time so a call stays a plain call:
//! - `def total(xs...)`, `total(...xs)`, `total(*xs)` (Julia/Swift, JS, Python): the last parameter is the list of the
//!   arguments the others leave, `total(1, 2, 3)` → `total([1 2 3])`;
//! - `f(...xs)`, `f(xs...)`, `f(*xs)` spread a list: into a rest parameter it is passed as it is (`[2] + xs` after
//!   other leftover arguments), into fixed parameters as `xs#1, xs#2, …` up to the parameter count.

use crate::analyzer::call_name;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::tuples::STARRED;
use std::collections::HashMap;

/// A function's parameter count, and whether its last parameter is a rest parameter
#[derive(Clone, Copy)]
struct Signature {
	fixed: usize,
	rest: bool,
}

pub fn lower(node: Node) -> Node {
	let mut signatures = HashMap::new();
	collect_signatures(&node, &mut signatures);
	if !signatures.values().any(|signature| signature.rest) && !has_spread(&node) {
		return node;
	}
	Variadic { signatures }.rewrite(node)
}

/// `*xs`, `...xs` (parsed as `*xs`) and `xs...` (an open range of xs): the name xs
fn starred(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Symbol(name) => name.strip_prefix(STARRED).filter(|name| !name.is_empty()).map(str::to_string),
		Node::Key(name, Op::To, end) if matches!(end.drop_meta(), Node::Empty) => match name.drop_meta() {
			Node::Symbol(name) => Some(name.clone()),
			_ => None,
		},
		_ => None,
	}
}

/// `f(params) := body`, also with a result type `f(params): T = body`: the head's items
fn definition_head(node: &Node) -> Option<&[Node]> {
	let Node::Key(head, Op::Define | Op::Assign, _) = node.drop_meta() else { return None };
	head_items(head)
}

fn head_items(head: &Node) -> Option<&[Node]> {
	match head.drop_meta() {
		Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => Some(items),
		Node::Key(head, Op::Colon | Op::Arrow, _) => head_items(head),
		_ => None,
	}
}

fn collect_signatures(node: &Node, signatures: &mut HashMap<String, Signature>) {
	node.visit(&mut |part| {
		let Some(head) = definition_head(part) else { return };
		let rest = head.last().is_some_and(|last| head.len() > 1 && starred(last).is_some());
		signatures.insert(head[0].name(), Signature { fixed: head.len() - 1 - rest as usize, rest });
	});
}

fn has_spread(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |part| if let Node::List(items, bracket, separator) = part {
		found |= call_name(items, bracket, separator).is_some() && items[1..].iter().any(|item| starred(item).is_some());
	});
	found
}

struct Variadic {
	signatures: HashMap<String, Signature>,
}

impl Variadic {
	fn rewrite(&self, node: Node) -> Node {
		if let Some(head) = definition_head(&node) {
			let rest = head.len() > 1 && starred(head.last().expect("a name")).is_some();
			let Node::Key(target, op, body) = node.drop_meta() else { unreachable!("a definition") };
			let target = if rest { Box::new(self.unstarred_head(target)) } else { target.clone() };
			return Node::Key(target, *op, Box::new(self.rewrite(body.as_ref().clone())));
		}
		match node {
			Node::List(items, bracket, separator) if self.is_call(&items, &bracket, &separator) => self.call(items),
			Node::Key(left, op, right) => Node::Key(Box::new(self.rewrite(*left)), op, Box::new(self.rewrite(*right))),
			Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| self.rewrite(item)).collect(), bracket, separator),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.rewrite(*node)), data },
			other => other,
		}
	}

	/// `(f a xs...)` → `(f a xs)`, also under a result type `f(a, xs...):int`
	fn unstarred_head(&self, target: &Node) -> Node {
		match target.drop_meta() {
			Node::Key(head, op, result) => Node::Key(Box::new(self.unstarred_head(head)), *op, result.clone()),
			Node::List(items, bracket, separator) => {
				let mut items = items.clone();
				let last = items.pop().expect("a rest parameter");
				items.push(Node::Symbol(starred(&last).expect("a rest parameter")));
				Node::List(items, bracket.clone(), separator.clone())
			}
			other => other.clone(),
		}
	}

	/// `f(…)` with a spread argument, a call of a function with a rest parameter, also `f 1 2 3`
	fn is_call(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> bool {
		let Some(name) = items.first().map(Node::drop_meta).and_then(|head| match head {
			Node::Symbol(name) => Some(name),
			_ => None,
		}) else { return false };
		let rest = self.signatures.get(name).is_some_and(|signature| signature.rest);
		let spread = items[1..].iter().any(|item| starred(item).is_some());
		let called = call_name(items, bracket, separator).is_some() || (*separator == Separator::Space && matches!(bracket, Bracket::None | Bracket::Round));
		called && (rest || spread)
	}

	fn call(&self, items: Vec<Node>) -> Node {
		let mut items = items.into_iter();
		let head = items.next().expect("a function name");
		let arguments: Vec<Node> = items.map(|item| self.rewrite(item)).collect();
		let signature = self.signatures.get(&head.name()).copied();
		let arguments = match signature {
			Some(Signature { fixed, rest: true }) if arguments.len() >= fixed => {
				let (fixed_arguments, leftover) = arguments.split_at(fixed);
				[spread_items(fixed_arguments, None), vec![rest_list(leftover)]].concat()
			}
			Some(Signature { fixed, rest: false }) => spread_items(&arguments, Some(fixed)),
			_ => arguments.iter().map(|argument| starred(argument).map(Node::Symbol).unwrap_or_else(|| argument.clone())).collect(),
		};
		Node::List([vec![head], arguments].concat(), Bracket::Round, Separator::None)
	}
}

/// The arguments with every spread list replaced by its items: a list literal's own items, else `xs#1, xs#2, …` as many
/// as the `count` of parameters leaves for it
fn spread_items(arguments: &[Node], count: Option<usize>) -> Vec<Node> {
	let mut items = vec![];
	for (index, argument) in arguments.iter().enumerate() {
		match (starred(argument), argument.drop_meta()) {
			(Some(name), _) => {
				let left_for_it = count.unwrap_or(0).saturating_sub(items.len() + arguments.len() - index - 1);
				items.extend((1..=left_for_it).map(|position| Node::Key(Box::new(Node::Symbol(name.clone())), Op::Hash, Box::new(crate::node::int(position as i64)))));
			}
			_ => items.push(argument.clone()),
		}
	}
	items
}

/// The leftover arguments of a rest parameter as one list: `[8 9]`, a lone spread `xs` as it is, `[2] + xs` after others
fn rest_list(leftover: &[Node]) -> Node {
	let mut list: Option<Node> = None;
	let mut pending: Vec<Node> = vec![];
	let append = |list: Option<Node>, part: Node| match list {
		Some(list) => Some(Node::Key(Box::new(list), Op::Add, Box::new(part))),
		None => Some(part),
	};
	for argument in leftover {
		match starred(argument) {
			Some(name) => {
				if !pending.is_empty() {
					list = append(list, Node::List(std::mem::take(&mut pending), Bracket::Square, Separator::Space));
				}
				list = append(list, Node::Symbol(name));
			}
			None => pending.push(argument.clone()),
		}
	}
	if !pending.is_empty() || list.is_none() {
		list = append(list, Node::List(pending, Bracket::Square, Separator::Space));
	}
	list.expect("a list")
}
