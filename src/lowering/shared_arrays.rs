//! Shared arrays (user decision P33 step 6, P44): `shared xs = int[n]` makes n Ints every task of the run shares (held by
//! the host: src/shared.rs natively, host.js in the browser), `shared xs = float[n]` n floats; `go f(xs)` passes that
//! same array, the one exception to copying. The array is an Int naming it; its uses become host words, atomic in the
//! host: `xs#i` → `shared_get(xs, i)`, `xs#i = v` → `shared_set(xs, i, v)`, `xs#i += v` → `shared_add(xs, i, v)`
//! (`shared_getf` … for floats), `#xs` / `count(xs)` → `shared_count(xs)`, `for x in xs` loops over its cells and xs as a
//! whole is the list of its cells. A parameter given a shared array (by a call or a `go`) is shared too, of the same
//! element type.
//! Linear arrays (notes/linear_arrays.md): `linear xs = int[n]`, `linear xs = float[n]` are rewritten the same way into
//! the module's own words over a block of linear memory (`linear_get`, `linear_set` … in wasm_emitter/linear_arrays.rs);
//! they stay in their task's memory, so a `go` cannot take one. Declaring one is discouraged: the compiler picks where
//! number lists live by itself.
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
const MAP_WORD: &str = "map";
const DOT_WORD: &str = "dot";
const SUM_WORD: &str = "sum";

/// `linear xs = int[n]`: an array in linear memory
const LINEAR_WORD: &str = "linear";
const LINEAR_TOPIC: &str = "linear-array";

/// The cell of a shared value
const VALUE_CELL: i64 = 1;

/// What a shared name holds: an array of Ints or floats, or one value (P106)
#[derive(Clone, Copy, PartialEq)]
struct Shared {
	element: Element,
	value: bool,
	storage: Storage,
}

/// Where an array's cells are: in the host (shared with every task) or in the module's linear memory
#[derive(Clone, Copy, PartialEq)]
enum Storage {
	Host,
	Linear,
}

impl Storage {
	/// The words that create and count an array
	fn new_and_count(self) -> (&'static str, &'static str) {
		match self {
			Storage::Host => (SHARED_NEW, SHARED_COUNT),
			Storage::Linear => (crate::wasm_emitter::linear_arrays::LINEAR_NEW, crate::wasm_emitter::linear_arrays::LINEAR_COUNT),
		}
	}
}

#[derive(Clone, Copy, PartialEq)]
enum Element {
	Int,
	Float,
	/// a shared value of true or false, 1 or 0 in its cell
	Bool,
}

/// The words of reading, writing and adding an element of a shared or linear array of Ints or of floats
fn element_words(kind: Shared) -> [&'static str; 3] {
	use crate::wasm_emitter::linear_arrays::{LINEAR_FLOAT_WORDS, LINEAR_INT_WORDS};
	let [_, get, set, add, _] = crate::host::SHARED_WORDS;
	match (kind.storage, kind.element == Element::Float) {
		(Storage::Host, true) => crate::host::SHARED_FLOAT_WORDS,
		(Storage::Host, false) => [get, set, add],
		(Storage::Linear, true) => LINEAR_FLOAT_WORDS,
		(Storage::Linear, false) => LINEAR_INT_WORDS,
	}
}

pub fn lower(node: Node) -> Node {
	let mut node = crate::gpu_maps::kept_on_gpu(node);
	let mut declared = HashMap::new();
	let mut first_linear: Option<Node> = None;
	node.visit(&mut |part| if let Some((name, _, shared)) = declaration(part) {
		if shared.storage == Storage::Linear && first_linear.is_none() {
			first_linear = Some(part.clone());
		}
		declared.insert(name, shared);
	});
	// a map of a map's result is one too: until no name is added
	let (defines_dot, mut pairs) = (definitions(&node).contains_key(DOT_WORD), 0);
	loop {
		node = paired_with_linear(node, &declared, defines_dot, &mut pairs);
		let results = float_map_results(&node, &declared);
		if results.is_empty() {
			break;
		}
		declared.extend(results);
	}
	let node = crate::gpu_maps::warn_unapplied(node);
	if (declared.is_empty() && !reduces_on_gpu(&node)) || matches!(node, Node::Error(_)) {
		return node;
	}
	if let Some(linear) = first_linear {
		crate::normalize::set_position_of(&linear);
		crate::diagnostic::educate_once(LINEAR_TOPIC, "linear xs = int[n]", "xs = int[n]",
			"the compiler picks where a list of numbers lives by itself, linear memory included; `linear` only forces it");
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
	let storage = match word.drop_meta() {
		Node::Symbol(w) if SHARED_WORDS.contains(&w.as_str()) => Storage::Host,
		Node::Symbol(w) if w == LINEAR_WORD => Storage::Linear,
		_ => return None,
	};
	let Node::Key(target, Op::Assign, value) = assignment.drop_meta() else { return None };
	let Node::Symbol(name) = target.drop_meta() else { return None };
	let Node::Key(element, Op::Hash, index) = value.drop_meta() else {
		// a linear value would be a variable: only arrays are linear
		return (storage == Storage::Host).then(|| (name.clone(), value.as_ref().clone(), Shared { element: value_element(value), value: true, storage }));
	};
	let element = if FLOAT_WORDS.contains(&element.name().as_str()) { Element::Float } else { Element::Int };
	Some((name.clone(), written_count(index), Shared { element, value: false, storage }))
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
	match (crate::warp_parser::subscript_key(index), index.drop_meta()) {
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
		// `"a \(xs#1)"`: the holes' expressions as code now, so their shared names are rewritten like any others
		let mentioned = |hole: &Node| mentions_any(hole, &self.names(function));
		if let Some(text) = matches!(node, Node::Meta { .. }).then(|| crate::interpolation::interpolated_mentioning(&node, mentioned)).flatten() {
			return self.node(text, function);
		}
		if let Some((name, first, kind)) = declaration(&node) {
			let count = if kind.value { crate::node::int(VALUE_CELL) } else { self.node(first.clone(), function) };
			let created = Node::Key(Box::new(Node::Symbol(name.clone())), Op::Assign, Box::new(builtin(kind.storage.new_and_count().0, vec![count])));
			if !kind.value {
				return created;
			}
			// `n = shared_new(1); shared_set(n, 1, v)`: the value is set once the name holds its cell
			let [_, set, _] = element_words(kind);
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
					// `s = sum(xs.map(x => …) @gpu)`, min, max: reduced on the GPU, only a partial result per workgroup read back
					(Node::Symbol(_), Op::Assign) if let Some((reduction, map)) = crate::gpu_maps::gpu_reduction(&value)
						&& let Some((array, _, kernel)) = gpu_kernel_map(&map) => {
						let on_cpu = self.node(value.as_ref().clone(), function);
						gpu_reduced(&target, reduction, &array, &kernel, on_cpu, &names, crate::gpu_maps::keeping(&value))
					}
					// `ys = xs.map(x => …) @gpu` of a linear float array: the lambda as a WGSL kernel, the CPU's map without an adapter
					(Node::Symbol(_), Op::Assign) if shared(&target, &names).is_some() && let Some((array, lambda, kernel)) = gpu_kernel_map(&value) => {
						let keeping = crate::gpu_maps::keeping(&value);
						if is_linear_floats(&array, &names) {
							gpu_mapped(&target, &array, &lambda, kernel, keeping)
						} else {
							let (copy, block) = copied_into_block(&target, &array);
							Node::List(vec![copy, gpu_mapped(&target, &block, &lambda, kernel, keeping)], Bracket::None, Separator::Semicolon)
						}
					}
					// `ys = xs.map(x => x * 0.5 + 1)` of a linear float array: its float kernel makes ys, a new linear array
					(Node::Symbol(_), Op::Assign) if shared(&target, &names).is_some() && let Some((array, kernel)) = float_map(&value, &names) => {
						Node::Key(target, op, Box::new(builtin(&kernel, vec![array])))
					}
					(Node::Symbol(_), Op::Assign) if shared(&target, &names).is_some() && let Some((array, lambda)) = numeric_map(&value, &names) => {
						block_mapped(&target, &array, &lambda, "map_loop", &[])
					}
					// `ys = gpu_compute(shader, xs, w)` of a linear float array: ys names the same block
					(Node::Symbol(_), Op::Assign) if shared(&target, &names).is_some() && gpu_compute_of_linear_floats(&value, &names) => {
						Node::Key(target, op, Box::new(self.node(*value, function)))
					}
					// `n = n + v`, `n = n - v`: the atomic add, as `n += v`, so no task's update is lost
					(name, Op::Assign) if let Some(kind) = shared_value(name, &names) && let Some((op, added)) = self_update(name, &value) => {
						let [_, _, add] = element_words(kind);
						builtin(add, vec![name.clone(), crate::node::int(VALUE_CELL), signed(self.node(added, function), op)])
					}
					// `n = v`, `n += v` of a shared value
					(name, Op::Assign) if let Some(kind) = shared_value(name, &names) => {
						let [_, set, _] = element_words(kind);
						builtin(set, vec![name.clone(), crate::node::int(VALUE_CELL), cell_value(self.node(*value, function), kind.element)])
					}
					(name, Op::AddAssign | Op::SubAssign) if let Some(kind) = shared_value(name, &names) => {
						let [_, _, add] = element_words(kind);
						builtin(add, vec![name.clone(), crate::node::int(VALUE_CELL), signed(self.node(*value, function), op)])
					}
					(Node::Key(array, Op::Hash, index), Op::Assign) if let Some(kind) = shared(array, &names) => {
						let [_, set, _] = element_words(kind);
						builtin(set, vec![array.as_ref().clone(), self.node(index.as_ref().clone(), function), self.node(*value, function)])
					}
					(Node::Key(array, Op::Hash, index), Op::AddAssign | Op::SubAssign) if let Some(kind) = shared(array, &names) => {
						let [_, _, add] = element_words(kind);
						builtin(add, vec![array.as_ref().clone(), self.node(index.as_ref().clone(), function), signed(self.node(*value, function), op)])
					}
					(array, Op::Hash) if !matches!(value.drop_meta(), Node::Empty) && let Some(kind) = shared(array, &names) => {
						let [get, _, _] = element_words(kind);
						builtin(get, vec![array.clone(), self.node(*value, function)])
					}
					// `#xs`, `xs.count`
					(Node::Empty, Op::Hash) if let Some(kind) = shared(&value, &names) => builtin(kind.storage.new_and_count().1, vec![*value]),
					(array, Op::Dot) if let Some(kind) = shared(array, &names) && COUNTING_WORDS.contains(&value.name().as_str()) => builtin(kind.storage.new_and_count().1, vec![array.clone()]),
					_ => Node::Key(Box::new(self.node(*target, function)), op, Box::new(self.node(*value, function))),
				}
			}
			// `shared_writes(n)` (signal_values::poll_shared) takes the array, not its value
			Node::List(ref items, _, _) if items.first().is_some_and(|head| head.name() == crate::host::SHARED_WRITES) => node,
			Node::List(items, bracket, separator) => {
				let names = self.names(function);
				// a shared value given to a function that shares it goes as its cell, not as its value
				let passed = call(&Node::List(items.clone(), bracket.clone(), separator.clone())).and_then(|(callee, _)| self.shared_parameters.get(&callee).cloned()).unwrap_or_default();
				let starts_task = items.first().is_some_and(|head| head.name() == crate::declarations::TASK_GO);
				let offset = if starts_task { 2 } else { 1 };
				let linear_argument = items.iter().skip(offset).find(|item| shared(item, &names).is_some_and(|kind| kind.storage == Storage::Linear));
				if let (true, Some(array)) = (starts_task, linear_argument) {
					return crate::diagnostic::Diagnostic::at(array, format!("a task cannot take the linear array {}: it lives in this task's memory; `shared {} = int[n]` shares one", array.name(), array.name())).into_error();
				}
				match items.as_slice() {
					[word, array] if COUNTING_WORDS.contains(&word.name().as_str()) && let Some(kind) = shared(array, &names) => builtin(kind.storage.new_and_count().1, vec![array.clone()]),
					[word, left, right] if word.name() == crate::wasm_emitter::linear_arrays::LINEAR_DOT && is_linear_floats(left, &names) && is_linear_floats(right, &names) => builtin(&word.name(), vec![left.clone(), right.clone()]),
					[word, shader, array, workgroups] if word.name() == crate::host::GPU_COMPUTE && is_linear_floats(array, &names) => {
						builtin(crate::host::GPU_COMPUTE_LINEAR, vec![self.node(shader.clone(), function), array.clone(), self.node(workgroups.clone(), function)])
					}
					// `for x in xs {…}` over an array: over its indexes, x read from each cell
					[word, item, in_word, array, body] if word.name() == "for" && in_word.name() == "in" && let Some(kind) = shared(array, &names) => {
						let index = Node::Symbol(format!("{}{}index", array.name(), crate::analyzer::TEMPORARY_SEPARATOR));
						let read = Node::Key(Box::new(item.clone()), Op::Assign, Box::new(builtin(element_words(kind)[0], vec![array.clone(), index.clone()])));
						let statements = match self.node(body.clone(), function).drop_meta().clone() {
							Node::List(statements, Bracket::Curly, Separator::Semicolon | Separator::Newline) => statements,
							Node::List(statement, Bracket::Curly, separator) => vec![Node::List(statement, Bracket::None, separator)],
							other => vec![other],
						};
						let indexes = Node::Key(Box::new(crate::node::int(1)), Op::To, Box::new(builtin(kind.storage.new_and_count().1, vec![array.clone()])));
						let body = Node::List([vec![read], statements].concat(), Bracket::Curly, Separator::Semicolon);
						Node::List(vec![word.clone(), index, in_word.clone(), indexes, body], bracket, separator)
					}
					_ => Node::List(items.into_iter().enumerate().map(|(index, item)| match index.checked_sub(offset) {
						Some(argument) if passed.contains_key(&argument) && (shared_value(&item, &names).is_some() || shared(&item, &names).is_some()) => item,
						_ => self.node(item, function),
					}).collect(), bracket, separator),
				}
			}
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.node(*node, function)), data },
			// an array as a whole: the list of its cells
			Node::Symbol(_) if let Some(kind) = shared(&node, &self.names(function)) => collected(&node, kind),
			// a read of a shared value
			Node::Symbol(_) if let Some(kind) = shared_value(&node, &self.names(function)) => {
				let [get, _, _] = element_words(kind);
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

/// `xs.map(x => body)` or `map(xs, x => body)` of a linear float array whose body a float kernel computes: the array and
/// the kernel's name (wasm_emitter/linear_arrays.rs, f64x2 lanes)
fn float_map(value: &Node, names: &HashMap<String, Shared>) -> Option<(Node, String)> {
	let (array, lambda) = map_parts(value)?;
	let Node::Key(parameter, Op::FatArrow, body) = lambda.drop_meta() else { return None };
	let Node::Symbol(parameter) = parameter.drop_meta() else { return None };
	(is_linear_floats(array, names) && crate::wasm_emitter::linear_arrays::is_float_kernel(parameter, body))
		.then(|| (array.clone(), crate::wasm_emitter::linear_arrays::kernel_name(parameter, body)))
}

/// `xs.map(f)` or `map(xs, f)`: xs and f
fn map_parts(value: &Node) -> Option<(&Node, &Node)> {
	match value.drop_meta() {
		Node::Key(array, Op::Dot, call) => match call.drop_meta() {
			Node::List(items, _, _) if items.len() == 2 && items[0].name() == MAP_WORD => Some((array.as_ref(), &items[1])),
			_ => None,
		},
		Node::List(items, _, _) if items.len() == 3 && items[0].name() == MAP_WORD => Some((&items[1], &items[2])),
		_ => None,
	}
}

/// `xs.map(x => …)` of a linear float array whose lambda is pure arithmetic (any the f64x2 kernel cannot compute,
/// `sin(x)`, `max(x, 2)`): xs and the lambda, mapped in one loop into a new block (card linear-map); a chain
/// `xs.map(f).map(g)` is one loop of g after f
fn numeric_map(value: &Node, names: &HashMap<String, Shared>) -> Option<(Node, Node)> {
	let (array, lambda) = map_parts(value)?;
	let (array, lambda) = crate::gpu_maps::fused(array.clone(), lambda.clone());
	(is_linear_floats(&array, names) && crate::gpu_maps::kernel(&lambda).is_some()).then_some((array, lambda))
}

fn is_linear_floats(array: &Node, names: &HashMap<String, Shared>) -> bool {
	shared(array, names).is_some_and(is_linear_float_array)
}

fn is_linear_float_array(kind: Shared) -> bool {
	kind.storage == Storage::Linear && kind.element == Element::Float && !kind.value
}

/// `xs.map(x => …) @gpu` whose lambda WGSL computes: xs, the lambda and its kernel
fn gpu_kernel_map(value: &Node) -> Option<(Node, Node, crate::gpu_maps::Kernel)> {
	let (array, lambda) = crate::gpu_maps::gpu_map(value)?;
	let kernel = crate::gpu_maps::gpu_kernel(&lambda)?;
	Some((array, lambda, kernel))
}

/// `xs .op ys` with ys a linear array (or an element-wise expression of one): the items paired by index as
/// broadcasting.rs pairs two lists, where the map would read ys as one number (its block's address); `dot(xs, ys)` of
/// one is `sum(xs .* ys)`. `pairs` counts the pairings, naming each one's lists
fn paired_with_linear(node: Node, names: &HashMap<String, Shared>, defines_dot: bool, pairs: &mut usize) -> Node {
	use crate::broadcasting::{PAIRED_SUM_TEMPLATE, PAIRED_TEMPLATE};
	let node = match node.drop_meta() {
		Node::List(items, Bracket::Round, _) if !defines_dot && items.len() == 3 && items[0].name() == DOT_WORD && items[1..].iter().any(|list| is_linear_list(list, names)) => {
			let product = crate::analyzer::element_wise(items[1].clone(), Op::Mul, items[2].clone());
			Node::List(vec![Node::Symbol(SUM_WORD.into()), product], Bracket::Round, Separator::None)
		}
		_ => node,
	};
	let summed = match node.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 2 && items[0].name() == SUM_WORD => Some(items[1].clone()),
		_ => None,
	};
	if let Some(paired) = summed.and_then(|product| paired_linear(&product, names, defines_dot, pairs, PAIRED_SUM_TEMPLATE)) {
		return paired;
	}
	let node = node.map_children(|child| paired_with_linear(child, names, defines_dot, pairs));
	paired_linear(&node, names, defines_dot, pairs, PAIRED_TEMPLATE).unwrap_or(node)
}

/// `receiver .op operand` of a linear operand, its items paired by index in `template`: the list, or its fused sum
fn paired_linear(node: &Node, names: &HashMap<String, Shared>, defines_dot: bool, pairs: &mut usize, template: &str) -> Option<Node> {
	use crate::broadcasting::{element_wise_parts, named_by, paired_by};
	let (receiver, op, operand) = element_wise_parts(node).filter(|(_, _, operand)| is_linear_list(operand, names))?;
	if template == crate::broadcasting::PAIRED_SUM_TEMPLATE && op == Op::Mul && is_linear_floats(&receiver, names) && is_linear_floats(&operand, names) {
		return Some(linear_dot(receiver, operand));
	}
	let (receiver, operand) = (paired_with_linear(receiver, names, defines_dot, pairs), paired_with_linear(operand, names, defines_dot, pairs));
	*pairs += 1;
	Some(paired_by(receiver, op, operand, template, named_by(format!("linear_{pairs}"))))
}

/// `sum(xs .* ys)` of two linear float arrays: their lengths checked, then linear_dotf over both blocks
fn linear_dot(left: Node, right: Node) -> Node {
	use crate::wasm_emitter::linear_arrays::LINEAR_DOT;
	let template = format!("({}; {LINEAR_DOT}(LEFT, RIGHT))", crate::broadcasting::PAIRED_COUNT).replace("LEFT", "dot_left").replace("RIGHT", "dot_right");
	let bindings = HashMap::from([("dot_left".to_string(), left), ("dot_right".to_string(), right)]);
	crate::law::substitute(&crate::warp_parser::parse(&template), &bindings)
}

/// A linear array, or an element-wise expression of one: `ys`, `(ys .* 2)`
fn is_linear_list(node: &Node, names: &HashMap<String, Shared>) -> bool {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 1 => is_linear_list(&items[0], names),
		_ => shared(node, names).is_some_and(|kind| kind.storage == Storage::Linear)
			|| crate::broadcasting::element_wise_parts(node).is_some_and(|(receiver, _, _)| is_linear_list(&receiver, names)),
	}
}

/// Whether a name of `names` occurs in node
fn mentions_any(node: &Node, names: &HashMap<String, Shared>) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= matches!(part.drop_meta(), Node::Symbol(name) if names.contains_key(name)));
	found
}

/// Whether the program has a `s = sum(xs.map(f) @gpu)` that may run on the GPU, of any list of floats
fn reduces_on_gpu(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= matches!(part.drop_meta(), Node::Key(target, Op::Assign, value) if matches!(target.drop_meta(), Node::Symbol(_))
		&& crate::gpu_maps::gpu_reduction(value).is_some_and(|(_, map)| gpu_kernel_map(&map).is_some())));
	found
}

/// The program's numbers a kernel reads, as the list its host word takes
fn outer_values(kernel: &crate::gpu_maps::Kernel) -> Node {
	Node::List(kernel.outer.iter().cloned().map(Node::Symbol).collect(), Bracket::Square, Separator::Space)
}

/// `s = sum(xs.map(f) @gpu)`, min, max: the kernel leaves each workgroup's partial result in a new block, which the CPU
/// combines; `on_cpu`, the reduction as written, in f64 for few items or without an adapter
fn gpu_reduced(target: &Node, reduction: crate::gpu_maps::Reduction, array: &Node, kernel: &crate::gpu_maps::Kernel, on_cpu: Node, names: &HashMap<String, Shared>, keeping: i64) -> Node {
	use crate::wasm_emitter::linear_arrays::{LINEAR_COUNT, LINEAR_FLOAT_WORDS, LINEAR_NEW};
	let [get, _, _] = LINEAR_FLOAT_WORDS;
	let (size, fewest) = (crate::gpu_maps::WORKGROUP_SIZE, crate::gpu_maps::GPU_MAP_MIN_COUNT);
	let combined = reduction.combined("reduce_value", &format!("{get}(reduce_partials, reduce_index)"));
	let template = crate::warp_parser::parse(&format!("reduce_partials = {LINEAR_NEW}(({LINEAR_COUNT}(reduce_source) + {size} - 1)//{size}); \
		reduce_target = if {LINEAR_COUNT}(reduce_source) < {fewest} or {}(gpu_shader, reduce_source, gpu_values, reduce_partials, {LINEAR_COUNT}(reduce_partials), {keeping}) == 0 then reduce_on_cpu \
		else (reduce_value = {get}(reduce_partials, 1); for reduce_index in 2 to {LINEAR_COUNT}(reduce_partials) {{ reduce_value = {combined} }}; reduce_value)", crate::host::GPU_REDUCE_LINEAR));
	let (copy, source) = if is_linear_floats(array, names) { (None, array.clone()) } else {
		let (copy, block) = copied_into_block(target, array);
		(Some(copy), block)
	};
	let separator = crate::analyzer::TEMPORARY_SEPARATOR;
	let temporary = |part: &str| Node::Symbol(format!("{}{separator}reduce{separator}{part}", target.name()));
	let reduced = [("reduce_partials", temporary("partials")), ("reduce_value", temporary("value")), ("reduce_index", temporary("index")), ("reduce_target", target.clone()),
		("reduce_source", source), ("reduce_on_cpu", on_cpu), ("gpu_shader", Node::Text(kernel.reduce_shader(reduction))), ("gpu_values", outer_values(kernel))]
		.into_iter().fold(template, |node, (placeholder, value)| crate::library_words::substitute(node, placeholder, &value));
	match copy {
		Some(copy) => Node::List(vec![copy, reduced], Bracket::None, Separator::Semicolon),
		None => reduced,
	}
}

/// The items of the list `array` (not in linear memory) as floats in a new block, once: the statements and the name
/// holding the block's address
fn copied_into_block(target: &Node, array: &Node) -> (Node, Node) {
	use crate::wasm_emitter::linear_arrays::{LINEAR_FLOAT_WORDS, LINEAR_NEW};
	let [_, set, _] = LINEAR_FLOAT_WORDS;
	let copy = crate::warp_parser::parse(&format!("copy_list = copy_items; copy_block = {LINEAR_NEW}(count(copy_list)); copy_index = 0; for copy_item in copy_list {{ copy_index = copy_index + 1; {set}(copy_block, copy_index, {}(copy_item)) }}", FLOAT_WORDS[0]));
	let separator = crate::analyzer::TEMPORARY_SEPARATOR;
	let temporary = |part: &str| Node::Symbol(format!("{}{separator}copy{separator}{part}", target.name()));
	let block = temporary("block");
	let copy = [("copy_list", temporary("list")), ("copy_block", block.clone()), ("copy_index", temporary("index")), ("copy_item", temporary("item")), ("copy_items", array.clone())]
		.into_iter().fold(copy, |node, (placeholder, value)| crate::library_words::substitute(node, placeholder, &value));
	(copy, block)
}

/// `ys = linear_new(count(xs))`, then the kernel maps xs's cells into ys's, given the values of the program's numbers it
/// reads, or the CPU does: for few items or without an adapter
fn gpu_mapped(target: &Node, array: &Node, lambda: &Node, kernel: crate::gpu_maps::Kernel, keeping: i64) -> Node {
	use crate::wasm_emitter::linear_arrays::LINEAR_COUNT;
	let size = crate::gpu_maps::WORKGROUP_SIZE;
	let fewest = crate::gpu_maps::GPU_MAP_MIN_COUNT;
	let mapped = format!("if {LINEAR_COUNT}(map_source) < {fewest} or {}(gpu_shader, map_source, gpu_values, map_target, ({LINEAR_COUNT}(map_source) + {size} - 1)//{size}, {keeping}) == 0 {{ map_loop }}", crate::host::GPU_MAP_LINEAR);
	block_mapped(target, array, lambda, &mapped, &[("gpu_shader", Node::Text(kernel.map_shader())), ("gpu_values", outer_values(&kernel))])
}

/// `ys = linear_new(count(xs))`, then `mapped` (`map_loop` in it the CPU's loop writing each cell of ys from xs's)
fn block_mapped(target: &Node, array: &Node, lambda: &Node, mapped: &str, fills: &[(&str, Node)]) -> Node {
	use crate::library_words::substitute;
	use crate::wasm_emitter::linear_arrays::{LINEAR_COUNT, LINEAR_FLOAT_WORDS, LINEAR_NEW};
	let Node::Key(parameter, _, body) = lambda.drop_meta() else { unreachable!("a numeric map's function is a lambda") };
	let [get, set, _] = LINEAR_FLOAT_WORDS;
	let each_cell = format!("for map_index in 1 to {LINEAR_COUNT}(map_source) {{ {set}(map_target, map_index, {}(map_item)) }}", FLOAT_WORDS[0]);
	let template = crate::warp_parser::parse(&format!("map_target = {LINEAR_NEW}({LINEAR_COUNT}(map_source)); {}", mapped.replace("map_loop", &each_cell)));
	let separator = crate::analyzer::TEMPORARY_SEPARATOR;
	let index = Node::Symbol(format!("{}{separator}map{separator}index", target.name()));
	let item = substitute(body.as_ref().clone(), &parameter.name(), &builtin(get, vec![array.clone(), index.clone()]));
	[("map_target", target.clone()), ("map_source", array.clone()), ("map_index", index), ("map_item", item)].into_iter().chain(fills.iter().cloned())
		.fold(template, |node, (placeholder, value)| substitute(node, placeholder, &value))
}

/// `gpu_compute(shader, xs, workgroups)` of a linear float array, which the shader updates in place
fn gpu_compute_of_linear_floats(value: &Node, names: &HashMap<String, Shared>) -> bool {
	matches!(value.drop_meta(), Node::List(items, _, _) if items.len() == 4 && items[0].name() == crate::host::GPU_COMPUTE && is_linear_floats(&items[2], names))
}

/// The names assigned once, by a float map or a gpu_compute of a linear float array: linear float arrays too
fn float_map_results(node: &Node, declared: &HashMap<String, Shared>) -> HashMap<String, Shared> {
	let mut assignments: HashMap<String, (usize, bool)> = HashMap::new();
	node.visit(&mut |part| if let Node::Key(target, Op::Assign | Op::AddAssign | Op::SubAssign | Op::MulAssign | Op::DivAssign, value) = part {
		if let Node::Symbol(name) = target.drop_meta() {
			let entry = assignments.entry(name.clone()).or_insert((0, false));
			entry.0 += 1;
			entry.1 = float_map(value, declared).is_some() || gpu_compute_of_linear_floats(value, declared) || gpu_kernel_map(value).is_some() || numeric_map(value, declared).is_some();
		}
	});
	let linear_floats = Shared { element: Element::Float, value: false, storage: Storage::Linear };
	assignments.into_iter().filter(|(name, (count, maps))| *count == 1 && *maps && !declared.contains_key(name)).map(|(name, _)| (name, linear_floats)).collect()
}

/// What a shared array named by `node` holds, if it is one
fn shared(node: &Node, names: &HashMap<String, Shared>) -> Option<Shared> {
	match node.drop_meta() {
		Node::Symbol(name) => names.get(name).copied().filter(|kind| !kind.value),
		_ => None,
	}
}


/// `(xs·list = float[count(xs)]; for xs·index in 1 to count(xs) { xs·list#xs·index = get(xs, xs·index) }; xs·list)`
fn collected(array: &Node, kind: Shared) -> Node {
	let name = array.name();
	let separator = crate::analyzer::TEMPORARY_SEPARATOR;
	let [get, _, _] = element_words(kind);
	let count = kind.storage.new_and_count().1;
	let element = if kind.element == Element::Float { FLOAT_WORDS[0] } else { "int" };
	use crate::library_words::substitute;
	let template = crate::warp_parser::parse(&format!(
		"(linear_list = {element}[{count}(linear_array)]; for linear_index in 1 to {count}(linear_array) {{ linear_list#linear_index = {get}(linear_array, linear_index) }}; linear_list)"
	));
	let named = |suffix: &str| Node::Symbol(format!("{name}{separator}{suffix}"));
	substitute(substitute(substitute(template, "linear_array", array), "linear_list", &named("list")), "linear_index", &named("index"))
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
