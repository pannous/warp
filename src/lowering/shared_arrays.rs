//! Shared arrays (user decision P33 step 6, P44): `shared xs = int[n]` makes n Ints every task of the run shares (held by
//! the host: src/shared.rs natively, host.js in the browser), `shared xs = float[n]` n floats; `go f(xs)` passes that
//! same array, the one exception to copying. The array is an Int naming it; its uses become host words, atomic in the
//! host: `xs#i` → `shared_get(xs, i)`, `xs#i = v` → `shared_set(xs, i, v)`, `xs#i += v` → `shared_add(xs, i, v)`
//! (`shared_getf` … for floats), `#xs` / `count(xs)` → `shared_count(xs)`. A parameter given a shared array (by a call
//! or a `go`) is shared too, of the same element type.
//! Shared values (P106): `shared done = false`, `shared n = 0`, `shared x = 0.5` are one-cell arrays: a read of `n` is
//! `shared_get(n, 1)`, `n = v` is `shared_set(n, 1, v)`, `n += v` is `shared_add(n, 1, v)`; a boolean is the Int 1 or 0.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashMap;

/// `shared xs = int[n]` and its synonym `atomic` (P44)
const SHARED_WORDS: [&str; 2] = ["shared", "atomic"];
/// The host words (src/host.rs SHARED_WORDS, SHARED_FLOAT_WORDS)
const SHARED_NEW: &str = crate::host::SHARED_WORDS[0];
const SHARED_COUNT: &str = crate::host::SHARED_WORDS[4];
const COUNTING_WORDS: [&str; 3] = ["count", "length", "size"];
const FLOAT_WORDS: [&str; 3] = ["float", "real", "double"];

/// The cell of a shared value
const VALUE_CELL: i64 = 1;

/// What a shared name holds: an array of Ints or floats, or one value (P106)
#[derive(Clone, Copy, PartialEq)]
struct Shared {
	element: Element,
	value: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Element {
	Int,
	Float,
	/// a shared value of true or false, 1 or 0 in its cell
	Bool,
}

/// The words of reading, writing and adding an element of a shared array of Ints or of floats
fn element_words(element: Element) -> [&'static str; 3] {
	let [_, get, set, add, _] = crate::host::SHARED_WORDS;
	if element == Element::Float { crate::host::SHARED_FLOAT_WORDS } else { [get, set, add] }
}

pub fn lower(node: Node) -> Node {
	let mut declared = HashMap::new();
	node.visit(&mut |part| declared.extend(declaration(part).map(|(name, _, shared)| (name, shared))));
	if declared.is_empty() {
		return node;
	}
	let functions = definitions(&node);
	let shared_parameters = shared_parameters(&node, &functions, &declared);
	Rewrite { declared, functions: &functions, shared_parameters: &shared_parameters }.node(node, None)
}

/// The names a program declares shared, arrays and values
pub(crate) fn shared_names(node: &Node) -> Vec<String> {
	let mut names = vec![];
	node.visit(&mut |part| names.extend(declaration(part).map(|(name, _, _)| name)));
	names
}

/// `shared xs = int[n]`, `shared xs = float[n]`: the name, the count n and what it holds; `shared n = v`: the name, the
/// value v and its type
fn declaration(node: &Node) -> Option<(String, Node, Shared)> {
	let Node::List(items, _, _) = node.drop_meta() else { return None };
	let [word, assignment] = items.as_slice() else { return None };
	if !matches!(word.drop_meta(), Node::Symbol(w) if SHARED_WORDS.contains(&w.as_str())) {
		return None;
	}
	let Node::Key(target, Op::Assign, value) = assignment.drop_meta() else { return None };
	let Node::Symbol(name) = target.drop_meta() else { return None };
	let Node::Key(element, Op::Hash, index) = value.drop_meta() else {
		return Some((name.clone(), value.as_ref().clone(), Shared { element: value_element(value), value: true }));
	};
	let element = if FLOAT_WORDS.contains(&element.name().as_str()) { Element::Float } else { Element::Int };
	Some((name.clone(), written_count(index), Shared { element, value: false }))
}

/// The type of a shared value from its first value: `false`, `0.5`, `-1.5`, else an Int
fn value_element(value: &Node) -> Element {
	match value.drop_meta() {
		Node::True | Node::False => Element::Bool,
		Node::Number(crate::extensions::numbers::Number::Float(_)) => Element::Float,
		Node::Key(_, Op::Neg, negated) => value_element(negated),
		_ => Element::Int,
	}
}

/// A value as its cell holds it: true and false are 1 and 0
fn cell_value(value: Node, element: Element) -> Node {
	match (value.drop_meta(), element) {
		(Node::True, Element::Bool) => crate::node::int(1),
		(Node::False, Element::Bool) => crate::node::int(0),
		_ => value,
	}
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
fn shared_parameters(node: &Node, functions: &HashMap<String, Vec<String>>, declared: &HashMap<String, Shared>) -> HashMap<String, HashMap<usize, Shared>> {
	let mut shared: HashMap<String, HashMap<usize, Shared>> = HashMap::new();
	loop {
		let mut changed = false;
		node.visit(&mut |part| {
			let Some((callee, arguments)) = call(part) else { return };
			if !functions.contains_key(&callee) {
				return;
			}
			for (index, argument) in arguments.iter().enumerate() {
				let Node::Symbol(name) = argument.drop_meta() else { continue };
				let kind = declared.get(name).copied().or_else(|| functions.iter().find_map(|(function, parameters)| {
					shared.get(function).and_then(|indexes| indexes.iter().find(|(index, _)| parameters.get(**index) == Some(name)).map(|(_, kind)| *kind))
				}));
				if let Some(kind) = kind {
					if shared.entry(callee.clone()).or_default().insert(index, kind).is_none() {
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
	declared: HashMap<String, Shared>,
	functions: &'a HashMap<String, Vec<String>>,
	shared_parameters: &'a HashMap<String, HashMap<usize, Shared>>,
}

impl Rewrite<'_> {
	/// The shared names in a function's body and their element types: its shared parameters and the declared arrays
	/// it does not shadow
	fn names(&self, function: Option<&str>) -> HashMap<String, Shared> {
		let mut names = self.declared.clone();
		if let Some(function) = function {
			let parameters = &self.functions[function];
			names.retain(|name, _| !parameters.contains(name));
			let indexes = self.shared_parameters.get(function).cloned().unwrap_or_default();
			names.extend(indexes.iter().filter_map(|(index, kind)| parameters.get(*index).map(|name| (name.clone(), *kind))));
		}
		names
	}

	fn node(&self, node: Node, function: Option<&str>) -> Node {
		if let Some((name, first, kind)) = declaration(&node) {
			let count = if kind.value { crate::node::int(VALUE_CELL) } else { self.node(first.clone(), function) };
			let created = Node::Key(Box::new(Node::Symbol(name.clone())), Op::Assign, Box::new(builtin(SHARED_NEW, vec![count])));
			if !kind.value {
				return created;
			}
			// `n = shared_new(1); shared_set(n, 1, v)`: the value is set once the name holds its cell
			let [_, set, _] = element_words(kind.element);
			let first = builtin(set, vec![Node::Symbol(name), crate::node::int(VALUE_CELL), cell_value(self.node(first, function), kind.element)]);
			return Node::List(vec![created, first], Bracket::None, Separator::Semicolon);
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
					// `n = n + v`, `n = n - v`: the atomic add, as `n += v`, so no task's update is lost
					(name, Op::Assign) if let Some(kind) = shared_value(name, &names) && let Some((op, added)) = self_update(name, &value) => {
						let [_, _, add] = element_words(kind.element);
						builtin(add, vec![name.clone(), crate::node::int(VALUE_CELL), signed(self.node(added, function), op)])
					}
					// `n = v`, `n += v` of a shared value
					(name, Op::Assign) if let Some(kind) = shared_value(name, &names) => {
						let [_, set, _] = element_words(kind.element);
						builtin(set, vec![name.clone(), crate::node::int(VALUE_CELL), cell_value(self.node(*value, function), kind.element)])
					}
					(name, Op::AddAssign | Op::SubAssign) if let Some(kind) = shared_value(name, &names) => {
						let [_, _, add] = element_words(kind.element);
						builtin(add, vec![name.clone(), crate::node::int(VALUE_CELL), signed(self.node(*value, function), op)])
					}
					(Node::Key(array, Op::Hash, index), Op::Assign) if let Some(kind) = shared(array, &names) => {
						let [_, set, _] = element_words(kind.element);
						builtin(set, vec![array.as_ref().clone(), self.node(index.as_ref().clone(), function), self.node(*value, function)])
					}
					(Node::Key(array, Op::Hash, index), Op::AddAssign | Op::SubAssign) if let Some(kind) = shared(array, &names) => {
						let [_, _, add] = element_words(kind.element);
						builtin(add, vec![array.as_ref().clone(), self.node(index.as_ref().clone(), function), signed(self.node(*value, function), op)])
					}
					(array, Op::Hash) if !matches!(value.drop_meta(), Node::Empty) && let Some(kind) = shared(array, &names) => {
						let [get, _, _] = element_words(kind.element);
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
				// a shared value given to a function that shares it goes as its cell, not as its value
				let passed = call(&Node::List(items.clone(), bracket.clone(), separator.clone())).and_then(|(callee, _)| self.shared_parameters.get(&callee).cloned()).unwrap_or_default();
				let offset = if items.first().is_some_and(|head| head.name() == crate::declarations::TASK_GO) { 2 } else { 1 };
				match items.as_slice() {
					[word, array] if COUNTING_WORDS.contains(&word.name().as_str()) && shared(array, &names).is_some() => builtin(SHARED_COUNT, vec![array.clone()]),
					_ => Node::List(items.into_iter().enumerate().map(|(index, item)| match index.checked_sub(offset) {
						Some(argument) if passed.contains_key(&argument) && shared_value(&item, &names).is_some() => item,
						_ => self.node(item, function),
					}).collect(), bracket, separator),
				}
			}
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.node(*node, function)), data },
			// a read of a shared value
			Node::Symbol(_) if let Some(kind) = shared_value(&node, &self.names(function)) => {
				let [get, _, _] = element_words(kind.element);
				let read = builtin(get, vec![node, crate::node::int(VALUE_CELL)]);
				match kind.element {
					Element::Bool => Node::Key(Box::new(read), Op::Ne, Box::new(crate::node::int(0))),
					_ => read,
				}
			}
			other => other,
		}
	}
}

/// What a shared array named by `node` holds, if it is one
fn shared(node: &Node, names: &HashMap<String, Shared>) -> Option<Shared> {
	match node.drop_meta() {
		Node::Symbol(name) => names.get(name).copied().filter(|kind| !kind.value),
		_ => None,
	}
}

/// The type of a shared value named by `node`, if it is one
fn shared_value(node: &Node, names: &HashMap<String, Shared>) -> Option<Shared> {
	match node.drop_meta() {
		Node::Symbol(name) => names.get(name).copied().filter(|kind| kind.value),
		_ => None,
	}
}

/// `n + v` or `n - v` as the value of `n`: the compound operator and v
fn self_update(name: &Node, value: &Node) -> Option<(Op, Node)> {
	match value.drop_meta() {
		Node::Key(left, op @ (Op::Add | Op::Sub), right) if left.drop_meta() == name.drop_meta() => {
			Some((if *op == Op::Add { Op::AddAssign } else { Op::SubAssign }, right.as_ref().clone()))
		}
		_ => None,
	}
}

/// What `+= v` or `-= v` adds
fn signed(value: Node, op: Op) -> Node {
	match op {
		Op::SubAssign => Node::Key(Box::new(Node::Empty), Op::Neg, Box::new(value)),
		_ => value,
	}
}
