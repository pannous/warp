//! Shared arrays (user decision P33 step 6, P44): `shared xs = int[n]` makes n Ints every task of the run shares (held by
//! the host: src/shared.rs natively, host.js in the browser), `shared xs = float[n]` n floats; `go f(xs)` passes that
//! same array, the one exception to copying. The array is an Int naming it; its uses become host words, atomic in the
//! host: `xs#i` → `shared_get(xs, i)`, `xs#i = v` → `shared_set(xs, i, v)`, `xs#i += v` → `shared_add(xs, i, v)`
//! (`shared_getf` … for floats), `#xs` / `count(xs)` → `shared_count(xs)`. A parameter given a shared array (by a call
//! or a `go`) is shared too, of the same element type.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashMap;

const SHARED_WORD: &str = "shared";
/// The host words (src/host.rs SHARED_WORDS, SHARED_FLOAT_WORDS)
const SHARED_NEW: &str = crate::host::SHARED_WORDS[0];
const SHARED_COUNT: &str = crate::host::SHARED_WORDS[4];
const COUNTING_WORDS: [&str; 3] = ["count", "length", "size"];
const FLOAT_WORDS: [&str; 3] = ["float", "real", "double"];

/// The element type of a shared array: whether it holds floats (else Ints)
type Floats = bool;

/// The words of reading, writing and adding an element of a shared array of Ints or of floats
fn element_words(floats: Floats) -> [&'static str; 3] {
	let [_, get, set, add, _] = crate::host::SHARED_WORDS;
	if floats { crate::host::SHARED_FLOAT_WORDS } else { [get, set, add] }
}

pub fn lower(node: Node) -> Node {
	let mut declared = HashMap::new();
	node.visit(&mut |part| if let Some((name, _, floats)) = declaration(part) { declared.insert(name, floats); });
	if declared.is_empty() {
		return node;
	}
	let functions = definitions(&node);
	let shared_parameters = shared_parameters(&node, &functions, &declared);
	Rewrite { declared, functions: &functions, shared_parameters: &shared_parameters }.node(node, None)
}

/// `shared xs = int[n]`, `shared xs = float[n]`: the name, the count n and whether it holds floats
fn declaration(node: &Node) -> Option<(String, Node, Floats)> {
	let Node::List(items, _, _) = node.drop_meta() else { return None };
	let [word, assignment] = items.as_slice() else { return None };
	if !matches!(word.drop_meta(), Node::Symbol(w) if w == SHARED_WORD) {
		return None;
	}
	let Node::Key(target, Op::Assign, value) = assignment.drop_meta() else { return None };
	let Node::Symbol(name) = target.drop_meta() else { return None };
	let Node::Key(element, Op::Hash, index) = value.drop_meta() else { return None };
	Some((name.clone(), written_count(index), FLOAT_WORDS.contains(&element.name().as_str())))
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

/// `f(xs)` or `go f(xs)` (lower_tasks' `task·go(f, xs)`) with a shared xs: f's parameter is shared, of xs's element
/// type, until nothing changes
fn shared_parameters(node: &Node, functions: &HashMap<String, Vec<String>>, declared: &HashMap<String, Floats>) -> HashMap<String, HashMap<usize, Floats>> {
	let mut shared: HashMap<String, HashMap<usize, Floats>> = HashMap::new();
	loop {
		let mut changed = false;
		node.visit(&mut |part| {
			let Some((callee, arguments)) = call(part) else { return };
			if !functions.contains_key(&callee) {
				return;
			}
			for (index, argument) in arguments.iter().enumerate() {
				let Node::Symbol(name) = argument.drop_meta() else { continue };
				let floats = declared.get(name).copied().or_else(|| functions.iter().find_map(|(function, parameters)| {
					shared.get(function).and_then(|indexes| indexes.iter().find(|(index, _)| parameters.get(**index) == Some(name)).map(|(_, floats)| *floats))
				}));
				if let Some(floats) = floats {
					if shared.entry(callee.clone()).or_default().insert(index, floats).is_none() {
						changed = true;
					}
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
	declared: HashMap<String, Floats>,
	functions: &'a HashMap<String, Vec<String>>,
	shared_parameters: &'a HashMap<String, HashMap<usize, Floats>>,
}

impl Rewrite<'_> {
	/// The shared names in a function's body and their element types: its shared parameters and the declared arrays
	/// it does not shadow
	fn names(&self, function: Option<&str>) -> HashMap<String, Floats> {
		let mut names = self.declared.clone();
		if let Some(function) = function {
			let parameters = &self.functions[function];
			names.retain(|name, _| !parameters.contains(name));
			let indexes = self.shared_parameters.get(function).cloned().unwrap_or_default();
			names.extend(indexes.iter().filter_map(|(index, floats)| parameters.get(*index).map(|name| (name.clone(), *floats))));
		}
		names
	}

	fn node(&self, node: Node, function: Option<&str>) -> Node {
		if let Some((name, count, _)) = declaration(&node) {
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
					(Node::Key(array, Op::Hash, index), Op::Assign) if let Some(floats) = shared(array, &names) => {
						let [_, set, _] = element_words(floats);
						builtin(set, vec![array.as_ref().clone(), self.node(index.as_ref().clone(), function), self.node(*value, function)])
					}
					(Node::Key(array, Op::Hash, index), Op::AddAssign | Op::SubAssign) if let Some(floats) = shared(array, &names) => {
						let [_, _, add] = element_words(floats);
						let value = self.node(*value, function);
						let added = if op == Op::SubAssign { Node::Key(Box::new(Node::Empty), Op::Neg, Box::new(value)) } else { value };
						builtin(add, vec![array.as_ref().clone(), self.node(index.as_ref().clone(), function), added])
					}
					(array, Op::Hash) if !matches!(value.drop_meta(), Node::Empty) && let Some(floats) = shared(array, &names) => {
						let [get, _, _] = element_words(floats);
						builtin(get, vec![array.clone(), self.node(*value, function)])
					}
					// `#xs`, `xs.count`
					(Node::Empty, Op::Hash) if shared(&value, &names).is_some() => builtin(SHARED_COUNT, vec![*value]),
					(array, Op::Dot) if shared(array, &names).is_some() && COUNTING_WORDS.contains(&value.name().as_str()) => builtin(SHARED_COUNT, vec![array.clone()]),
					_ => Node::Key(Box::new(self.node(*target, function)), op, Box::new(self.node(*value, function))),
				}
			}
			Node::List(items, bracket, separator) => {
				let names = self.names(function);
				match items.as_slice() {
					[word, array] if COUNTING_WORDS.contains(&word.name().as_str()) && shared(array, &names).is_some() => builtin(SHARED_COUNT, vec![array.clone()]),
					_ => Node::List(items.into_iter().map(|item| self.node(item, function)).collect(), bracket, separator),
				}
			}
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.node(*node, function)), data },
			other => other,
		}
	}
}

/// The element type of a shared array named by `node`, if it is one
fn shared(node: &Node, names: &HashMap<String, Floats>) -> Option<Floats> {
	match node.drop_meta() {
		Node::Symbol(name) => names.get(name).copied(),
		_ => None,
	}
}
