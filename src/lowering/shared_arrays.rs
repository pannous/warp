//! Shared arrays (user decision P33 step 6, P44): `shared xs = int[n]` makes n Ints every task of the run shares (held by
//! the host: src/shared.rs natively, host.js in the browser); `go f(xs)` passes that same array, the one exception to
//! copying. The array is an Int naming it; its uses become host words, atomic in the host:
//! `xs#i` → `shared_get(xs, i)`, `xs#i = v` → `shared_set(xs, i, v)`, `xs#i += v` → `shared_add(xs, i, v)`,
//! `#xs` / `count(xs)` → `shared_count(xs)`. A parameter given a shared array (by a call or a `go`) is shared too.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

const SHARED_WORD: &str = "shared";
/// The host words (src/host.rs SHARED_WORDS)
const SHARED_NEW: &str = crate::host::SHARED_WORDS[0];
const SHARED_GET: &str = crate::host::SHARED_WORDS[1];
const SHARED_SET: &str = crate::host::SHARED_WORDS[2];
const SHARED_ADD: &str = crate::host::SHARED_WORDS[3];
const SHARED_COUNT: &str = crate::host::SHARED_WORDS[4];
const COUNTING_WORDS: [&str; 3] = ["count", "length", "size"];

pub fn lower(node: Node) -> Node {
	let mut declared = HashSet::new();
	node.visit(&mut |part| if let Some((name, _)) = declaration(part) { declared.insert(name); });
	if declared.is_empty() {
		return node;
	}
	let functions = definitions(&node);
	let shared_parameters = shared_parameters(&node, &functions, &declared);
	Rewrite { declared, functions: &functions, shared_parameters: &shared_parameters }.node(node, None)
}

/// `shared xs = int[n]`: the name and the count n
fn declaration(node: &Node) -> Option<(String, Node)> {
	let Node::List(items, _, _) = node.drop_meta() else { return None };
	let [word, assignment] = items.as_slice() else { return None };
	if !matches!(word.drop_meta(), Node::Symbol(w) if w == SHARED_WORD) {
		return None;
	}
	let Node::Key(target, Op::Assign, value) = assignment.drop_meta() else { return None };
	let Node::Symbol(name) = target.drop_meta() else { return None };
	let Node::Key(_, Op::Hash, index) = value.drop_meta() else { return None };
	Some((name.clone(), written_count(index)))
}

/// `int[n]` arrives as the 1-based `int#(n+1)`: n back
fn written_count(index: &Node) -> Node {
	match (crate::wasp_parser::subscript_key(index), index.drop_meta()) {
		(Some(count), _) => count.clone(),
		(None, Node::Number(number)) => Node::Number(*number - crate::extensions::numbers::Number::Int(1)),
		(None, other) => Node::Key(Box::new(other.clone()), Op::Sub, Box::new(crate::node::int(1))),
	}
}

/// The parameters of each function
fn definitions(node: &Node) -> HashMap<String, Vec<String>> {
	let mut functions = HashMap::new();
	node.visit(&mut |part| if let Node::Key(head, Op::Define | Op::Assign, _) = part {
		if let Node::List(items, Bracket::Round, _) = head.drop_meta() {
			if let Some(Node::Symbol(name)) = items.first().map(Node::drop_meta) {
				functions.insert(name.clone(), items[1..].iter().map(|parameter| match parameter.drop_meta() {
					Node::Key(name, _, _) => name.name(),
					other => other.name(),
				}).collect());
			}
		}
	});
	functions
}

/// `f(xs)` or `go f(xs)` (lower_tasks' `task·go(f, xs)`) with a shared xs: f's parameter is shared, until nothing changes
fn shared_parameters(node: &Node, functions: &HashMap<String, Vec<String>>, declared: &HashSet<String>) -> HashMap<String, HashSet<usize>> {
	let mut shared: HashMap<String, HashSet<usize>> = HashMap::new();
	loop {
		let mut changed = false;
		node.visit(&mut |part| {
			let Some((callee, arguments)) = call(part) else { return };
			if !functions.contains_key(&callee) {
				return;
			}
			for (index, argument) in arguments.iter().enumerate() {
				let Node::Symbol(name) = argument.drop_meta() else { continue };
				let is_shared = declared.contains(name) || functions.iter().any(|(function, parameters)| {
					shared.get(function).is_some_and(|indexes| indexes.iter().any(|index| parameters.get(*index) == Some(name)))
				});
				if is_shared && shared.entry(callee.clone()).or_default().insert(index) {
					changed = true;
				}
			}
		});
		if !changed {
			return shared;
		}
	}
}

/// A call of a user function: `f(a, b)` or the task start `task·go(f, a, b)`
fn call(node: &Node) -> Option<(String, Vec<Node>)> {
	let Node::List(items, _, _) = node.drop_meta() else { return None };
	let Node::Symbol(head) = items.first()?.drop_meta() else { return None };
	if head == crate::declarations::TASK_GO {
		return Some((items.get(1)?.name(), items[2..].to_vec()));
	}
	Some((head.clone(), items[1..].to_vec()))
}

fn builtin(word: &str, arguments: Vec<Node>) -> Node {
	Node::List([vec![Node::Symbol(word.to_string())], arguments].concat(), Bracket::Round, Separator::None)
}

struct Rewrite<'a> {
	declared: HashSet<String>,
	functions: &'a HashMap<String, Vec<String>>,
	shared_parameters: &'a HashMap<String, HashSet<usize>>,
}

impl Rewrite<'_> {
	/// The shared names in a function's body: its shared parameters and the declared arrays it does not shadow
	fn names(&self, function: Option<&str>) -> HashSet<String> {
		let mut names = self.declared.clone();
		if let Some(function) = function {
			let parameters = &self.functions[function];
			names.retain(|name| !parameters.contains(name));
			let indexes = self.shared_parameters.get(function).cloned().unwrap_or_default();
			names.extend(indexes.iter().filter_map(|index| parameters.get(*index).cloned()));
		}
		names
	}

	fn node(&self, node: Node, function: Option<&str>) -> Node {
		if let Some((name, count)) = declaration(&node) {
			return Node::Key(Box::new(Node::Symbol(name)), Op::Assign, Box::new(builtin(SHARED_NEW, vec![self.node(count, function)])));
		}
		match node {
			// a definition: its body sees its own shared parameters
			Node::Key(head, op @ (Op::Define | Op::Assign), body) if matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if items.first().is_some_and(|name| self.functions.contains_key(&name.name()))) => {
				let name = match head.drop_meta() { Node::List(items, _, _) => items[0].name(), _ => unreachable!("guarded") };
				Node::Key(head, op, Box::new(self.node(*body, Some(&name))))
			}
			Node::Key(target, op, value) => {
				let names = self.names(function);
				match (target.drop_meta(), op) {
					(Node::Key(array, Op::Hash, index), Op::Assign) if self.is_shared(array, &names) => {
						builtin(SHARED_SET, vec![array.as_ref().clone(), self.node(index.as_ref().clone(), function), self.node(*value, function)])
					}
					(Node::Key(array, Op::Hash, index), Op::AddAssign | Op::SubAssign) if self.is_shared(array, &names) => {
						let value = self.node(*value, function);
						let added = if op == Op::SubAssign { Node::Key(Box::new(Node::Empty), Op::Neg, Box::new(value)) } else { value };
						builtin(SHARED_ADD, vec![array.as_ref().clone(), self.node(index.as_ref().clone(), function), added])
					}
					(array, Op::Hash) if self.is_shared(array, &names) && !matches!(value.drop_meta(), Node::Empty) => {
						builtin(SHARED_GET, vec![array.clone(), self.node(*value, function)])
					}
					// `#xs`, `xs.count`
					(Node::Empty, Op::Hash) if self.is_shared(&value, &names) => builtin(SHARED_COUNT, vec![*value]),
					(array, Op::Dot) if self.is_shared(array, &names) && COUNTING_WORDS.contains(&value.name().as_str()) => builtin(SHARED_COUNT, vec![array.clone()]),
					_ => Node::Key(Box::new(self.node(*target, function)), op, Box::new(self.node(*value, function))),
				}
			}
			Node::List(items, bracket, separator) => {
				let names = self.names(function);
				match items.as_slice() {
					[word, array] if COUNTING_WORDS.contains(&word.name().as_str()) && self.is_shared(array, &names) => builtin(SHARED_COUNT, vec![array.clone()]),
					_ => Node::List(items.into_iter().map(|item| self.node(item, function)).collect(), bracket, separator),
				}
			}
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.node(*node, function)), data },
			other => other,
		}
	}

	fn is_shared(&self, node: &Node, names: &HashSet<String>) -> bool {
		matches!(node.drop_meta(), Node::Symbol(name) if names.contains(name))
	}
}
