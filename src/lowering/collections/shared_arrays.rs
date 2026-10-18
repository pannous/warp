//! Shared arrays (user decision P33 step 6, P44): `shared xs = int[n]` makes n Ints every task of the run shares (held by
//! the host: src/host/shared.rs natively, host.js in the browser), `shared xs = float[n]` n floats; `go f(xs)` passes that
//! same array, the one exception to copying. The array is an Int naming it; its uses become host words, atomic in the
//! host: `xs#i` → `shared_get(xs, i)`, `xs#i = v` → `shared_set(xs, i, v)`, `xs#i += v` → `shared_add(xs, i, v)`
//! (`shared_getf` … for floats), `#xs` / `count(xs)` → `shared_count(xs)`, `for x in xs` loops over its cells and xs as a
//! whole is the list of its cells. A parameter given a shared array (by a call or a `go`) is shared too, of the same
//! element type.
//! Linear arrays (notes/linear_arrays.md): `linear xs = int[n]`, `linear xs = float[n]` are rewritten the same way into
//! the module's own words over a block of linear memory (`linear_get`, `linear_set` … in wasm_emitter/lists/linear_arrays.rs);
//! they stay in their task's memory, so a `go` cannot take one. Declaring one is discouraged: the compiler picks where
//! number lists live by itself; plain `xs = float[n]` arrays paired by dot or `.*`, or mapped by a numeric kernel,
//! become linear ones (picked_linear).
//! Shared values (P106): `shared done = false`, `shared n = 0`, `shared x = 0.5` are one-cell arrays: a read of `n` is
//! `shared_get(n, 1)`, `n = v` is `shared_set(n, 1, v)`, `n += v` is `shared_add(n, 1, v)`; a boolean is the Int 1 or 0.
//! Kept as built (user decision cleanup-cuts): `shared` for any atomic variable, not only Int and float arrays and
//! one-cell values, is the general form for later.

use super::nodes::{any_part, assign, block, call, key};
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::{DOT_WORD, MAP_WORD, SUM_WORD};
use std::collections::{HashMap, HashSet};

/// `shared xs = int[n]` and its synonym `atomic` (P44)
const SHARED_WORDS: [&str; 2] = ["shared", "atomic"];
/// The host words (src/host.rs SHARED_WORDS, SHARED_FLOAT_WORDS)
const SHARED_NEW: &str = crate::host::SHARED_WORDS[0];
const SHARED_COUNT: &str = crate::host::SHARED_WORDS[4];
const FLOAT_WORDS: [&str; 3] = ["float", "real", "double"];

/// `linear xs = int[n]`: an array in linear memory
const LINEAR_WORD: &str = "linear";
const LINEAR_TOPIC: &str = "linear-array";

/// The cell of a shared value
const VALUE_CELL: i64 = 1;
/// The fewest cells a `go` map of a float kernel splits into slices on threads: an instance per slice costs about half a
/// millisecond, the kernel about a nanosecond a cell
const SPLIT_FEWEST: i64 = 1 << 18;

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
			Storage::Linear => (
				crate::wasm_emitter::lists::linear_arrays::LINEAR_NEW,
				crate::wasm_emitter::lists::linear_arrays::LINEAR_COUNT,
			),
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
	use crate::wasm_emitter::lists::linear_arrays::{LINEAR_FLOAT_WORDS, LINEAR_INT_WORDS};
	let [_, get, set, add, _] = crate::host::SHARED_WORDS;
	match (kind.storage, kind.element == Element::Float) {
		(Storage::Host, true) => crate::host::SHARED_FLOAT_WORDS,
		(Storage::Host, false) => [get, set, add],
		(Storage::Linear, true) => LINEAR_FLOAT_WORDS,
		(Storage::Linear, false) => LINEAR_INT_WORDS,
	}
}

pub fn lower(node: Node) -> Node {
	// a kernel may call the program's functions (card gpu-functions)
	let program = node.clone();
	crate::gpu_maps::with_program_functions(&program, || {
		lower_knowing_functions(crate::gpu_maps::painted_functions(node))
	})
}

fn lower_knowing_functions(node: Node) -> Node {
	let node = crate::gpu_maps::kept_on_gpu(node);
	// the first array the program itself declares linear, before the compiler picks any
	let mut first_linear: Option<(Node, String, Element)> = None;
	// the compiler's own (a name with TEMPORARY_SEPARATOR, as music_words' blocks) are not the program's
	node.visit(&mut |part| {
		if let Some((name, _, shared)) = declaration(part).filter(|(name, _, shared)| {
			shared.storage == Storage::Linear && !name.contains(crate::analyzer::TEMPORARY_SEPARATOR)
		}) {
			first_linear.get_or_insert((part.clone(), name, shared.element));
		}
	});
	let (mut node, picked) = picked_linear(node);
	let mut declared = HashMap::new();
	node.visit(&mut |part| {
		if let Some((name, _, shared)) = declaration(part) {
			declared.insert(name, shared);
		}
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
	// heavy maps of linear float arrays go to the GPU by themselves above a count (card gpu-auto), kept there as @gpu maps
	let node =
		crate::gpu_maps::kept_on_gpu(crate::gpu_maps::automatic(node, &|list| is_linear_floats(list, &declared)));
	let node = crate::gpu_maps::warn_unapplied(node);
	if (declared.is_empty() && !reduces_on_gpu(&node)) || matches!(node, Node::Error(_)) {
		return node;
	}
	// hinted unless paired in a way the compiler would not pick linear memory for
	if let Some((linear, _, element)) = first_linear.filter(|(_, name, _)| pairs == 0 || picked.contains(name)) {
		crate::normalize::set_position_of(&linear);
		let array = if element == Element::Float { FLOAT_WORDS[0] } else { "int" };
		crate::errors::diagnostic::educate_once(LINEAR_TOPIC, &format!("linear xs = {array}[n]"), &format!("xs = {array}[n]"),
			"the compiler picks where a list of numbers lives by itself, linear memory included; `linear` only forces it");
	}
	let functions = definitions(&node);
	let shared_parameters = shared_parameters(&node, &functions, &declared);
	Rewrite { declared, functions: &functions, shared_parameters: &shared_parameters }.node(node, None)
}

/// Plain `xs = float[n]` arrays paired by dot or `.*` with another float array, or mapped by a numeric kernel, every
/// other use one linear arrays support (cells, count, `for x in xs`): declared `linear`, so dot runs as linear_dotf over
/// the blocks (10^6: 3 ms instead of 13 ms over GC lists) and the map as one loop into a new block. The names picked,
/// those declared linear already included
fn picked_linear(node: Node) -> (Node, HashSet<String>) {
	let mut assignments: HashMap<String, usize> = HashMap::new();
	node.visit(&mut |part| {
		if let Some(name) = float_array_assignment(part) {
			*assignments.entry(name).or_default() += 1;
		}
	});
	let mut picked: HashSet<String> =
		assignments.into_iter().filter(|(_, count)| *count == 1).map(|(name, _)| name).collect();
	// dropping one array may leave its partner unpaired: until none is dropped
	loop {
		let kept: HashSet<String> =
			picked.iter().filter(|name| only_linear_uses(&node, name, &picked)).cloned().collect();
		if kept.len() == picked.len() {
			break;
		}
		picked = kept;
	}
	(declared_linear(node, &picked), picked)
}

/// `xs = float[n]`, plain or declared linear: xs
fn float_array_assignment(node: &Node) -> Option<String> {
	let Node::Key(target, Op::Assign, value) = node.drop_meta() else {
		return None;
	};
	let Node::Key(element, Op::Hash, _) = value.drop_meta() else {
		return None;
	};
	let name = target.symbol_name()?;
	FLOAT_WORDS.contains(&element.name().as_str()).then(|| name.to_string())
}

/// Whether every mention of name is one a linear array supports, and one of them pairs it with another of `picked` or
/// maps it by a kernel
fn only_linear_uses(node: &Node, name: &str, picked: &HashSet<String>) -> bool {
	let (mut mentions, mut supported, mut pairings) = (0, 0, 0);
	node.visit(&mut |part| {
		mentions += usize::from(part.is_symbol(name));
		let (uses, paired) = linear_uses(part, name, picked);
		supported += uses;
		pairings += paired;
	});
	pairings > 0 && supported == mentions
}

/// The mentions of name that part makes as a linear array supports them, and how many of those pair it with another
/// array of `picked`
fn linear_uses(part: &Node, name: &str, picked: &HashSet<String>) -> (usize, usize) {
	let is_name = |node: &Node| node.is_symbol(name);
	let is_picked = |node: &Node| matches!(node.drop_meta(), Node::Symbol(symbol) if picked.contains(symbol));
	let paired = |left: &Node, right: &Node| {
		let uses = usize::from(is_name(left)) + usize::from(is_name(right));
		if is_picked(left) && is_picked(right) {
			(uses, uses)
		} else {
			(0, 0)
		}
	};
	if let Some((receiver, _, operand)) = crate::broadcasting::element_wise_parts(part) {
		return paired(&receiver, &operand);
	}
	// a numeric kernel maps a linear array as one loop into a new block (f64x2 lanes where it can)
	if let Some((_, lambda)) = map_parts(part).filter(|(array, _)| is_name(array)) {
		let kernel = usize::from(crate::gpu_maps::linear_kernel(lambda).is_some());
		return (kernel, kernel);
	}
	let single = match part.drop_meta() {
		Node::Key(..) if float_array_assignment(part).as_deref() == Some(name) => true,
		Node::Key(array, Op::Hash, index) if is_name(array) => !index.is_empty_node(),
		Node::Key(empty, Op::Hash, array) => empty.is_empty_node() && is_name(array),
		Node::Key(array, Op::Dot, word) => is_name(array) && crate::analyzer::is_counting_word(&word.name()),
		Node::List(items, Bracket::Round, _) if items.len() == 3 && items[0].name() == DOT_WORD => {
			return paired(&items[1], &items[2])
		}
		Node::List(items, _, _) => match items.as_slice() {
			[word, array] => crate::analyzer::is_counting_word(&word.name()) && is_name(array),
			[word, _, in_word, array, _] => word.name() == "for" && in_word.name() == "in" && is_name(array),
			_ => false,
		},
		_ => false,
	};
	(usize::from(single), 0)
}

/// The assignments of `picked` arrays declared `linear`
fn declared_linear(node: Node, picked: &HashSet<String>) -> Node {
	if declaration(&node).is_some() {
		return node;
	}
	if float_array_assignment(&node).is_some_and(|name| picked.contains(&name)) {
		return Node::List(vec![symbol(LINEAR_WORD), node], Bracket::None, Separator::Space);
	}
	node.map_children(|child| declared_linear(child, picked))
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
	let Node::List(items, _, _) = node.drop_meta() else {
		return None;
	};
	let [word, assignment] = items.as_slice() else {
		return None;
	};
	let storage = match word.drop_meta() {
		Node::Symbol(w) if SHARED_WORDS.contains(&w.as_str()) => Storage::Host,
		Node::Symbol(w) if w == LINEAR_WORD => Storage::Linear,
		_ => return None,
	};
	let Node::Key(target, Op::Assign, value) = assignment.drop_meta() else {
		return None;
	};
	let name = target.symbol_name()?;
	let Node::Key(element, Op::Hash, index) = value.drop_meta() else {
		// a linear value would be a variable: only arrays are linear
		return (storage == Storage::Host).then(|| {
			(name.to_string(), value.as_ref().clone(), Shared { element: value_element(value), value: true, storage })
		});
	};
	let element = if FLOAT_WORDS.contains(&element.name().as_str()) { Element::Float } else { Element::Int };
	Some((name.to_string(), written_count(index), Shared { element, value: false, storage }))
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
		(None, other) => key(other.clone(), Op::Sub, crate::node::int(1)),
	}
}

/// The parameters of each function
fn definitions(node: &Node) -> HashMap<String, Vec<String>> {
	let mut functions = HashMap::new();
	node.visit(&mut |part| {
		if let Node::Key(head, Op::Define | Op::Assign, _) = part {
			if let Node::List(items, Bracket::Round, _) = head.drop_meta() {
				if let Some(Node::Symbol(name)) = items.first().map(Node::drop_meta) {
					functions.insert(
						name.clone(),
						items[1..]
							.iter()
							.map(|parameter| match parameter.drop_meta() {
								Node::Key(name, _, _) => name.name(),
								other => other.name(),
							})
							.collect(),
					);
				}
			}
		}
	});
	functions
}

/// `f(xs)` or `go f(xs)` (lower_tasks' `task·go(f, xs)`) with a shared xs: f's parameter is shared, of xs's element
/// type, until nothing changes
fn shared_parameters(
	node: &Node,
	functions: &HashMap<String, Vec<String>>,
	declared: &HashMap<String, Shared>,
) -> HashMap<String, HashMap<usize, Shared>> {
	let mut shared: HashMap<String, HashMap<usize, Shared>> = HashMap::new();
	loop {
		let mut changed = false;
		node.visit(&mut |part| {
			let Some((callee, arguments)) = called_function(part) else {
				return;
			};
			if !functions.contains_key(&callee) {
				return;
			}
			for (index, argument) in arguments.iter().enumerate() {
				let Node::Symbol(name) = argument.drop_meta() else {
					continue;
				};
				let kind = declared.get(name).copied().or_else(|| {
					functions.iter().find_map(|(function, parameters)| {
						shared.get(function).and_then(|indexes| {
							indexes
								.iter()
								.find(|(index, _)| parameters.get(**index) == Some(name))
								.map(|(_, kind)| *kind)
						})
					})
				});
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
fn called_function(node: &Node) -> Option<(String, Vec<Node>)> {
	let Node::List(items, _, _) = node.drop_meta() else {
		return None;
	};
	let Node::Symbol(head) = items.first()?.drop_meta() else {
		return None;
	};
	if head == crate::declarations::TASK_GO {
		return Some((items.get(1)?.name(), items[2..].to_vec()));
	}
	Some((head.clone(), items[1..].to_vec()))
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
			names.extend(
				indexes.iter().filter_map(|(index, kind)| parameters.get(*index).map(|name| (name.clone(), *kind))),
			);
		}
		names
	}

	fn node(&self, node: Node, function: Option<&str>) -> Node {
		// `"a \(xs#1)"`: the holes' expressions as code now, so their shared names are rewritten like any others
		let mentioned = |hole: &Node| mentions_any(hole, &self.names(function));
		if let Some(text) = matches!(node, Node::Meta { .. })
			.then(|| crate::text::interpolation::interpolated_mentioning(&node, mentioned))
			.flatten()
		{
			return self.node(text, function);
		}
		if let Some((name, first, kind)) = declaration(&node) {
			return self.declared(name, first, kind, function);
		}
		match node {
			// a definition: its body sees its own shared parameters
			Node::Key(head, op @ (Op::Define | Op::Assign), body) if matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if items.first().is_some_and(|name| self.functions.contains_key(&name.name()))) =>
			{
				let name = match head.drop_meta() {
					Node::List(items, _, _) => items[0].name(),
					_ => unreachable!("guarded"),
				};
				Node::Key(head, op, Box::new(self.node(*body, Some(&name))))
			}
			Node::Key(target, op, value) => {
				let names = self.names(function);
				let rewritten = match (target.drop_meta(), op) {
					(Node::Symbol(_), Op::Assign) => self.array_assigned(&target, &value, &names, function),
					_ => None,
				};
				rewritten
					.or_else(|| self.value_written(target.drop_meta(), op, &value, &names, function))
					.or_else(|| self.cell_accessed(target.drop_meta(), op, &value, &names, function))
					.unwrap_or_else(|| key(self.node(*target, function), op, self.node(*value, function)))
			}
			// `shared_writes(n)` (signal_values::poll_shared) takes the array, not its value
			Node::List(ref items, _, _)
				if items.first().is_some_and(|head| head.name() == crate::host::SHARED_WRITES) =>
			{
				node
			}
			Node::List(items, bracket, separator) => self.listed(items, bracket, separator, function),
			meta @ Node::Meta { .. } => meta.under_meta(|node| self.node(node, function)),
			// an array as a whole: the list of its cells
			Node::Symbol(_) if let Some(kind) = shared(&node, &self.names(function)) => collected(&node, kind),
			// a read of a shared value
			Node::Symbol(_) if let Some(kind) = shared_value(&node, &self.names(function)) => {
				let [get, _, _] = element_words(kind);
				let read = call(get, vec![node, crate::node::int(VALUE_CELL)]);
				match kind.element {
					Element::Bool => key(read, Op::Ne, crate::node::int(0)),
					_ => read,
				}
			}
			other => other,
		}
	}

	/// `shared xs = int[n]`: the array of n cells; a shared value one cell, set once the name holds it
	fn declared(&self, name: String, first: Node, kind: Shared, function: Option<&str>) -> Node {
		let count = if kind.value { crate::node::int(VALUE_CELL) } else { self.node(first.clone(), function) };
		let created = assign(Node::Symbol(name.clone()), call(kind.storage.new_and_count().0, vec![count]));
		if !kind.value {
			return created;
		}
		// `n = shared_new(1); shared_set(n, 1, v)`: the value is set once the name holds its cell
		let [_, set, _] = element_words(kind);
		let first = call(
			set,
			vec![
				Node::Symbol(name),
				crate::node::int(VALUE_CELL),
				cell_value(self.node(first, function), kind.element),
			],
		);
		Node::List(vec![created, first], Bracket::None, Separator::Semicolon)
	}

	/// `ys = …` of an array mapped or computed: on the GPU, by a float kernel or a loop over the block's cells
	fn array_assigned(
		&self,
		target: &Node,
		value: &Node,
		names: &HashMap<String, Shared>,
		function: Option<&str>,
	) -> Option<Node> {
		// `s = sum(xs.map(x => …) @gpu)`, min, max: reduced on the GPU, only a partial result per workgroup read back
		let reduced = crate::gpu_maps::gpu_reduction(value)
			.and_then(|(reduction, map)| gpu_kernel_map(&map).map(|(array, _, kernel)| (reduction, array, kernel)));
		if let Some((reduction, array, kernel)) = reduced {
			let on_cpu = self.node(value.clone(), function);
			return Some(gpu_reduced(
				target,
				reduction,
				&array,
				&kernel,
				on_cpu,
				names,
				crate::gpu_maps::keeping(value),
			));
		}
		shared(target, names)?;
		// `ys = xs.map(x => …) @gpu` of a linear float array: the lambda as a WGSL kernel, the CPU's map without an adapter
		if let Some((array, lambda, kernel)) = gpu_kernel_map(value) {
			let keeping = crate::gpu_maps::keeping(value);
			if is_linear_floats(&array, names) {
				return Some(gpu_mapped(target, &array, &lambda, kernel, keeping));
			}
			let (copy, block) = copied_into_block(target, &array);
			return Some(Node::List(
				vec![copy, gpu_mapped(target, &block, &lambda, kernel, keeping)],
				Bracket::None,
				Separator::Semicolon,
			));
		}
		// `ys = xs.map(x => x * 0.5 + 1)` of a linear float array: its float kernel makes ys, a new linear array
		if let Some((array, kernel)) = float_map(value, names) {
			let mapped = call(&kernel, vec![array.clone()]);
			return Some(match value.attribute(crate::parallel::SPLIT_ATTRIBUTE) {
				Some(_) => split_mapped(target, &array, &kernel, mapped),
				None => assign(target.clone(), mapped),
			});
		}
		if let Some((array, lambda)) = numeric_map(value, names) {
			return Some(block_mapped(target, &array, &lambda, "map_loop", &[]));
		}
		// `ys = gpu_compute(shader, xs, w)` of a linear float array: ys names the same block
		gpu_compute_of_linear_floats(value, names).then(|| assign(target.clone(), self.node(value.clone(), function)))
	}

	/// `n = v`, `n += v` of a shared value; `n = n + v`, `n = n - v` as the atomic add, as `n += v`, so no task's update
	/// is lost
	fn value_written(
		&self,
		name: &Node,
		op: Op,
		value: &Node,
		names: &HashMap<String, Shared>,
		function: Option<&str>,
	) -> Option<Node> {
		let kind = shared_value(name, names)?;
		let [_, set, add] = element_words(kind);
		let cell = |word: &str, value: Node| call(word, vec![name.clone(), crate::node::int(VALUE_CELL), value]);
		match op {
			Op::Assign if let Some((op, added)) = self_update(name, value) => {
				Some(cell(add, signed(self.node(added, function), op)))
			}
			Op::Assign => Some(cell(set, cell_value(self.node(value.clone(), function), kind.element))),
			Op::AddAssign | Op::SubAssign => Some(cell(add, signed(self.node(value.clone(), function), op))),
			_ => None,
		}
	}

	/// `xs#i = v`, `xs#i += v`, `xs#i`, `#xs` and `xs.count` of a shared array
	fn cell_accessed(
		&self,
		target: &Node,
		op: Op,
		value: &Node,
		names: &HashMap<String, Shared>,
		function: Option<&str>,
	) -> Option<Node> {
		let rewritten = |node: &Node| self.node(node.clone(), function);
		match (target, op) {
			(Node::Key(array, Op::Hash, index), Op::Assign) => {
				let [_, set, _] = element_words(shared(array, names)?);
				Some(call(set, vec![array.as_ref().clone(), rewritten(index), rewritten(value)]))
			}
			(Node::Key(array, Op::Hash, index), Op::AddAssign | Op::SubAssign) => {
				let [_, _, add] = element_words(shared(array, names)?);
				Some(call(add, vec![array.as_ref().clone(), rewritten(index), signed(rewritten(value), op)]))
			}
			(array, Op::Hash)
				if !value.is_empty_node()
					&& let Some(kind) = shared(array, names) =>
			{
				let [get, _, _] = element_words(kind);
				Some(call(get, vec![array.clone(), rewritten(value)]))
			}
			// `#xs`, `xs.count`
			(Node::Empty, Op::Hash) => Some(call(shared(value, names)?.storage.new_and_count().1, vec![value.clone()])),
			(array, Op::Dot) if crate::analyzer::is_counting_word(&value.name()) => {
				Some(call(shared(array, names)?.storage.new_and_count().1, vec![array.clone()]))
			}
			_ => None,
		}
	}

	/// A call or list: words of arrays rewritten, a loop over an array's cells, a shared value passed as its cell to a
	/// function that shares it; a task cannot take a linear array
	fn listed(&self, items: Vec<Node>, bracket: Bracket, separator: Separator, function: Option<&str>) -> Node {
		let names = self.names(function);
		// a shared value given to a function that shares it goes as its cell, not as its value
		let passed = called_function(&Node::List(items.clone(), bracket.clone(), separator.clone()))
			.and_then(|(callee, _)| self.shared_parameters.get(&callee).cloned())
			.unwrap_or_default();
		let starts_task = items.first().is_some_and(|head| head.name() == crate::declarations::TASK_GO);
		let offset = if starts_task { 2 } else { 1 };
		let linear_argument = items
			.iter()
			.skip(offset)
			.find(|item| shared(item, &names).is_some_and(|kind| kind.storage == Storage::Linear));
		if let (true, Some(array)) = (starts_task, linear_argument) {
			return crate::errors::diagnostic::Diagnostic::at(array, format!("a task cannot take the linear array {}: it lives in this task's memory; `shared {} = int[n]` shares one", array.name(), array.name())).into_error();
		}
		if let Some(word) = self.array_word(&items, &names, function) {
			return word;
		}
		// `for x in xs {…}` over an array: over its indexes, x read from each cell
		let over_cells = match items.as_slice() {
			[word, item, in_word, array, body] if word.name() == "for" && in_word.name() == "in" => {
				shared(array, &names).map(|kind| (word, item, in_word, array, body, kind))
			}
			_ => None,
		};
		if let Some((word, item, in_word, array, body, kind)) = over_cells {
			let indexes = key(crate::node::int(1), Op::To, call(kind.storage.new_and_count().1, vec![array.clone()]));
			let index = Node::Symbol(format!("{}{}index", array.name(), crate::analyzer::TEMPORARY_SEPARATOR));
			let body = self.cells_looped(item, array, &index, kind, body, function);
			return Node::List(vec![word.clone(), index, in_word.clone(), indexes, body], bracket, separator);
		}
		let arguments = items.into_iter().enumerate().map(|(index, item)| match index.checked_sub(offset) {
			Some(argument)
				if passed.contains_key(&argument)
					&& (shared_value(&item, &names).is_some() || shared(&item, &names).is_some()) =>
			{
				item
			}
			_ => self.node(item, function),
		});
		Node::List(arguments.collect(), bracket, separator)
	}

	/// The words of linear arrays: its address, count, dot product and gpu_compute over its block
	fn array_word(&self, items: &[Node], names: &HashMap<String, Shared>, function: Option<&str>) -> Option<Node> {
		use crate::wasm_emitter::lists::linear_arrays::{LINEAR_ADDRESS, LINEAR_DOT};
		match items {
			[word, array]
				if word.name() == LINEAR_ADDRESS
					&& shared(array, names).is_some_and(|kind| kind.storage == Storage::Linear) =>
			{
				Some(array.clone())
			}
			[word, array] if crate::analyzer::is_counting_word(&word.name()) => {
				Some(call(shared(array, names)?.storage.new_and_count().1, vec![array.clone()]))
			}
			[word, left, right]
				if word.name() == LINEAR_DOT && is_linear_floats(left, names) && is_linear_floats(right, names) =>
			{
				Some(call(&word.name(), vec![left.clone(), right.clone()]))
			}
			[word, shader, array, workgroups]
				if word.name() == crate::host::GPU_COMPUTE && is_linear_floats(array, names) =>
			{
				Some(call(
					crate::host::GPU_COMPUTE_LINEAR,
					vec![self.node(shader.clone(), function), array.clone(), self.node(workgroups.clone(), function)],
				))
			}
			_ => None,
		}
	}

	/// The body of `for item in array`, looping over the array's indexes: item read from the cell at index first
	fn cells_looped(
		&self,
		item: &Node,
		array: &Node,
		index: &Node,
		kind: Shared,
		body: &Node,
		function: Option<&str>,
	) -> Node {
		let read = assign(item.clone(), call(element_words(kind)[0], vec![array.clone(), index.clone()]));
		let statements = match self.node(body.clone(), function).drop_meta().clone() {
			Node::List(statements, Bracket::Curly, Separator::Semicolon | Separator::Newline) => statements,
			Node::List(statement, Bracket::Curly, separator) => vec![Node::List(statement, Bracket::None, separator)],
			other => vec![other],
		};
		block([vec![read], statements].concat())
	}
}

/// `xs.map(x => body)` or `map(xs, x => body)` of a linear float array whose body a float kernel computes: the array and
/// the kernel's name (wasm_emitter/lists/linear_arrays.rs, f64x2 lanes)
fn float_map(value: &Node, names: &HashMap<String, Shared>) -> Option<(Node, String)> {
	let (array, lambda) = map_parts(value)?;
	let Node::Key(parameter, Op::Arrow, body) = lambda.drop_meta() else {
		return None;
	};
	let parameter = parameter.symbol_name()?;
	(is_linear_floats(array, names) && crate::wasm_emitter::lists::linear_arrays::is_float_kernel(parameter, body))
		.then(|| (array.clone(), crate::wasm_emitter::lists::linear_arrays::kernel_name(parameter, body)))
}

/// `xs.map(f)` or `map(xs, f)`: xs and f
fn map_parts(value: &Node) -> Option<(&Node, &Node)> {
	match value.drop_meta() {
		Node::Key(array, Op::Dot, call) => match call.drop_meta() {
			Node::List(items, _, _) if items.len() == 2 && items[0].name() == MAP_WORD => {
				Some((array.as_ref(), &items[1]))
			}
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
	(is_linear_floats(&array, names) && crate::gpu_maps::linear_kernel(&lambda).is_some()).then_some((array, lambda))
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
		Node::List(items, Bracket::Round, _)
			if !defines_dot
				&& items.len() == 3
				&& items[0].name() == DOT_WORD
				&& items[1..].iter().any(|list| is_linear_list(list, names)) =>
		{
			let product = crate::broadcasting::element_wise(items[1].clone(), Op::Mul, items[2].clone());
			call(SUM_WORD, vec![product])
		}
		_ => node,
	};
	let summed = match node.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 2 && items[0].name() == SUM_WORD => {
			Some(items[1].clone())
		}
		_ => None,
	};
	if let Some(paired) =
		summed.and_then(|product| paired_linear(&product, names, defines_dot, pairs, PAIRED_SUM_TEMPLATE))
	{
		return paired;
	}
	let node = node.map_children(|child| paired_with_linear(child, names, defines_dot, pairs));
	paired_linear(&node, names, defines_dot, pairs, PAIRED_TEMPLATE).unwrap_or(node)
}

/// `receiver .op operand` of a linear operand, its items paired by index in `template`: the list, or its fused sum
fn paired_linear(
	node: &Node,
	names: &HashMap<String, Shared>,
	defines_dot: bool,
	pairs: &mut usize,
	template: &str,
) -> Option<Node> {
	use crate::broadcasting::{element_wise_parts, named_by, paired_by};
	let (receiver, op, operand) = element_wise_parts(node).filter(|(_, _, operand)| is_linear_list(operand, names))?;
	*pairs += 1;
	if template == crate::broadcasting::PAIRED_SUM_TEMPLATE
		&& op == Op::Mul
		&& is_linear_floats(&receiver, names)
		&& is_linear_floats(&operand, names)
	{
		return Some(linear_dot(receiver, operand));
	}
	let (receiver, operand) = (
		paired_with_linear(receiver, names, defines_dot, pairs),
		paired_with_linear(operand, names, defines_dot, pairs),
	);
	Some(paired_by(receiver, op, operand, template, named_by(format!("linear_{pairs}"))))
}

/// `sum(xs .* ys)` of two linear float arrays: their lengths checked, then linear_dotf over both blocks
fn linear_dot(left: Node, right: Node) -> Node {
	use crate::wasm_emitter::lists::linear_arrays::LINEAR_DOT;
	let template = format!("({}; {LINEAR_DOT}(LEFT, RIGHT))", crate::broadcasting::PAIRED_COUNT)
		.replace("LEFT", "dot_left")
		.replace("RIGHT", "dot_right");
	super::nodes::filled(&template, [("dot_left", left), ("dot_right", right)])
}

/// A linear array, or an element-wise expression of one: `ys`, `(ys .* 2)`
fn is_linear_list(node: &Node, names: &HashMap<String, Shared>) -> bool {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 1 => is_linear_list(&items[0], names),
		_ => {
			shared(node, names).is_some_and(|kind| kind.storage == Storage::Linear)
				|| crate::broadcasting::element_wise_parts(node)
					.is_some_and(|(receiver, _, _)| is_linear_list(&receiver, names))
		}
	}
}

/// Whether a name of `names` occurs in node
fn mentions_any(node: &Node, names: &HashMap<String, Shared>) -> bool {
	node.mentions_where(|name| names.contains_key(name))
}

/// Whether the program has a `s = sum(xs.map(f) @gpu)` that may run on the GPU, of any list of floats
fn reduces_on_gpu(node: &Node) -> bool {
	any_part(node, |part| {
		matches!(part.drop_meta(), Node::Key(target, Op::Assign, value) if matches!(target.drop_meta(), Node::Symbol(_))
		&& crate::gpu_maps::gpu_reduction(value).is_some_and(|(_, map)| gpu_kernel_map(&map).is_some()))
	})
}

/// The program's numbers a kernel reads, as the list its host word takes
fn outer_values(kernel: &crate::gpu_maps::Kernel) -> Node {
	Node::List(kernel.outer.iter().cloned().map(Node::Symbol).collect(), Bracket::Square, Separator::Space)
}

/// `s = sum(xs.map(f) @gpu)`, min, max: the kernel leaves each workgroup's partial result in a new block, which the CPU
/// combines; `on_cpu`, the reduction as written, in f64 for few items or without an adapter
fn gpu_reduced(
	target: &Node,
	reduction: crate::gpu_maps::Reduction,
	array: &Node,
	kernel: &crate::gpu_maps::Kernel,
	on_cpu: Node,
	names: &HashMap<String, Shared>,
	keeping: i64,
) -> Node {
	use crate::wasm_emitter::lists::linear_arrays::{LINEAR_COUNT, LINEAR_FLOAT_WORDS, LINEAR_NEW};
	let [get, _, _] = LINEAR_FLOAT_WORDS;
	let (size, fewest) = (crate::gpu_maps::WORKGROUP_SIZE, crate::gpu_maps::fewest_items(keeping));
	let combined = reduction.combined("reduce_value", &format!("{get}(reduce_partials, reduce_index)"));
	let template = crate::warp_parser::parse_generated(&format!("reduce_partials = {LINEAR_NEW}(({LINEAR_COUNT}(reduce_source) + {size} - 1) /! {size}); \
		reduce_target = if {LINEAR_COUNT}(reduce_source) < {fewest} or {}(gpu_shader, reduce_source, gpu_values, reduce_partials, {LINEAR_COUNT}(reduce_partials), {keeping}) == 0 then reduce_on_cpu \
		else (reduce_value = {get}(reduce_partials, 1); for reduce_index in 2 to {LINEAR_COUNT}(reduce_partials) {{ reduce_value = {combined} }}; reduce_value)", crate::host::GPU_REDUCE_LINEAR));
	let (copy, source) = if is_linear_floats(array, names) {
		(None, array.clone())
	} else {
		let (copy, block) = copied_into_block(target, array);
		(Some(copy), block)
	};
	let separator = crate::analyzer::TEMPORARY_SEPARATOR;
	let temporary = |part: &str| Node::Symbol(format!("{}{separator}reduce{separator}{part}", target.name()));
	let reduced = [
		("reduce_partials", temporary("partials")),
		("reduce_value", temporary("value")),
		("reduce_index", temporary("index")),
		("reduce_target", target.clone()),
		("reduce_source", source),
		("reduce_on_cpu", on_cpu),
		("gpu_shader", Node::Text(kernel.reduce_shader(reduction))),
		("gpu_values", outer_values(kernel)),
	]
	.into_iter()
	.fold(template, |node, (placeholder, value)| crate::library_words::substitute(node, placeholder, &value));
	match copy {
		Some(copy) => Node::List(vec![copy, reduced], Bracket::None, Separator::Semicolon),
		None => reduced,
	}
}

/// The items of the list `array` (not in linear memory) as floats in a new block, once: the statements and the name
/// holding the block's address
fn copied_into_block(target: &Node, array: &Node) -> (Node, Node) {
	use crate::wasm_emitter::lists::linear_arrays::{LINEAR_FLOAT_WORDS, LINEAR_NEW};
	let [_, set, _] = LINEAR_FLOAT_WORDS;
	let copy = crate::warp_parser::parse_generated(&format!("copy_list = copy_items; copy_block = {LINEAR_NEW}(count(copy_list)); copy_index = 0; for copy_item in copy_list {{ copy_index = copy_index + 1; {set}(copy_block, copy_index, {}(copy_item)) }}", FLOAT_WORDS[0]));
	let separator = crate::analyzer::TEMPORARY_SEPARATOR;
	let temporary = |part: &str| Node::Symbol(format!("{}{separator}copy{separator}{part}", target.name()));
	let block = temporary("block");
	let copy = [
		("copy_list", temporary("list")),
		("copy_block", block.clone()),
		("copy_index", temporary("index")),
		("copy_item", temporary("item")),
		("copy_items", array.clone()),
	]
	.into_iter()
	.fold(copy, |node, (placeholder, value)| crate::library_words::substitute(node, placeholder, &value));
	(copy, block)
}

/// `ys = linear_new(count(xs))`, then the kernel maps xs's cells into ys's, given the values of the program's numbers it
/// reads, or the CPU does: for few items or without an adapter
fn gpu_mapped(target: &Node, array: &Node, lambda: &Node, kernel: crate::gpu_maps::Kernel, keeping: i64) -> Node {
	use crate::wasm_emitter::lists::linear_arrays::LINEAR_COUNT;
	let size = crate::gpu_maps::WORKGROUP_SIZE;
	let fewest = crate::gpu_maps::fewest_items(keeping);
	let mapped = format!("if {LINEAR_COUNT}(map_source) < {fewest} or {}(gpu_shader, map_source, gpu_values, map_target, ({LINEAR_COUNT}(map_source) + {size} - 1) /! {size}, {keeping}) == 0 {{ map_loop }}", crate::host::GPU_MAP_LINEAR);
	block_mapped(
		target,
		array,
		lambda,
		&mapped,
		&[("gpu_shader", Node::Text(kernel.map_shader())), ("gpu_values", outer_values(&kernel))],
	)
}

/// `ys = go xs.map(x => …)` of a float kernel: from SPLIT_FEWEST cells on the host maps slices of xs on threads
/// (task_split_mapf), else, or where it gives 0, the kernel here
fn split_mapped(target: &Node, array: &Node, kernel: &str, mapped: Node) -> Node {
	use crate::wasm_emitter::lists::linear_arrays::LINEAR_COUNT;
	let template = crate::warp_parser::parse_generated(&format!(
		"map_target = (map_split = (if {LINEAR_COUNT}(map_source) >= {SPLIT_FEWEST} then {}(map_kernel, map_source) else 0); if map_split == 0 then map_kernel_call else map_split)",
		crate::host::TASK_SPLIT_MAPF
	));
	let separator = crate::analyzer::TEMPORARY_SEPARATOR;
	[
		("map_target", target.clone()),
		("map_source", array.clone()),
		("map_split", Node::Symbol(format!("{}{separator}split", target.name()))),
		("map_kernel", Node::Text(kernel.to_string())),
		("map_kernel_call", mapped),
	]
	.into_iter()
	.fold(template, |node, (placeholder, value)| crate::library_words::substitute(node, placeholder, &value))
}

/// `ys = linear_new(count(xs))`, then `mapped` (`map_loop` in it the CPU's loop writing each cell of ys from xs's)
fn block_mapped(target: &Node, array: &Node, lambda: &Node, mapped: &str, fills: &[(&str, Node)]) -> Node {
	use crate::library_words::substitute;
	use crate::wasm_emitter::lists::linear_arrays::{LINEAR_COUNT, LINEAR_FLOAT_WORDS, LINEAR_NEW};
	let Node::Key(parameter, _, body) = lambda.drop_meta() else {
		unreachable!("a numeric map's function is a lambda")
	};
	let [get, set, _] = LINEAR_FLOAT_WORDS;
	let each_cell = format!(
		"for map_index in 1 to {LINEAR_COUNT}(map_source) {{ {set}(map_target, map_index, {}(map_item)) }}",
		FLOAT_WORDS[0]
	);
	let template = crate::warp_parser::parse_generated(&format!(
		"map_target = {LINEAR_NEW}({LINEAR_COUNT}(map_source)); {}",
		mapped.replace("map_loop", &each_cell)
	));
	let separator = crate::analyzer::TEMPORARY_SEPARATOR;
	let index = Node::Symbol(format!("{}{separator}map{separator}index", target.name()));
	let item = substitute(body.as_ref().clone(), &parameter.name(), &call(get, vec![array.clone(), index.clone()]));
	[("map_target", target.clone()), ("map_source", array.clone()), ("map_index", index), ("map_item", item)]
		.into_iter()
		.chain(fills.iter().cloned())
		.fold(template, |node, (placeholder, value)| substitute(node, placeholder, &value))
}

/// `gpu_compute(shader, xs, workgroups)` of a linear float array, which the shader updates in place
fn gpu_compute_of_linear_floats(value: &Node, names: &HashMap<String, Shared>) -> bool {
	matches!(value.drop_meta(), Node::List(items, _, _) if items.len() == 4 && items[0].name() == crate::host::GPU_COMPUTE && is_linear_floats(&items[2], names))
}

/// The names assigned once, by a float map or a gpu_compute of a linear float array: linear float arrays too
fn float_map_results(node: &Node, declared: &HashMap<String, Shared>) -> HashMap<String, Shared> {
	let mut assignments: HashMap<String, (usize, bool)> = HashMap::new();
	node.visit(&mut |part| {
		if let Node::Key(target, Op::Assign | Op::AddAssign | Op::SubAssign | Op::MulAssign | Op::DivAssign, value) =
			part
		{
			if let Node::Symbol(name) = target.drop_meta() {
				let entry = assignments.entry(name.clone()).or_insert((0, false));
				entry.0 += 1;
				entry.1 = float_map(value, declared).is_some()
					|| gpu_compute_of_linear_floats(value, declared)
					|| gpu_kernel_map(value).is_some()
					|| numeric_map(value, declared).is_some();
			}
		}
	});
	let linear_floats = Shared { element: Element::Float, value: false, storage: Storage::Linear };
	assignments
		.into_iter()
		.filter(|(name, (count, maps))| *count == 1 && *maps && !declared.contains_key(name))
		.map(|(name, _)| (name, linear_floats))
		.collect()
}

/// What a shared array named by `node` holds, if it is one
fn shared(node: &Node, names: &HashMap<String, Shared>) -> Option<Shared> {
	node.symbol_name().and_then(|name| names.get(name).copied().filter(|kind| !kind.value))
}

/// `(xs·list = float[count(xs)]; for xs·index in 1 to count(xs) { xs·list#xs·index = get(xs, xs·index) }; xs·list)`
fn collected(array: &Node, kind: Shared) -> Node {
	let name = array.name();
	let separator = crate::analyzer::TEMPORARY_SEPARATOR;
	let [get, _, _] = element_words(kind);
	let count = kind.storage.new_and_count().1;
	let element = if kind.element == Element::Float { FLOAT_WORDS[0] } else { "int" };
	use crate::library_words::substitute;
	let template = crate::warp_parser::parse_generated(&format!(
		"(linear_list = {element}[{count}(linear_array)]; for linear_index in 1 to {count}(linear_array) {{ linear_list#linear_index = {get}(linear_array, linear_index) }}; linear_list)"
	));
	let named = |suffix: &str| Node::Symbol(format!("{name}{separator}{suffix}"));
	substitute(
		substitute(substitute(template, "linear_array", array), "linear_list", &named("list")),
		"linear_index",
		&named("index"),
	)
}

/// The type of a shared value named by `node`, if it is one
fn shared_value(node: &Node, names: &HashMap<String, Shared>) -> Option<Shared> {
	node.symbol_name().and_then(|name| names.get(name).copied().filter(|kind| kind.value))
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
		Op::SubAssign => key(Node::Empty, Op::Neg, value),
		_ => value,
	}
}
