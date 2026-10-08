//! One dispatch layer for list operations: each operation the emitter compiles on a list (count, element at an index,
//! index assignment, storing a list in a variable, reading it as a value) asks `list_backend` which implementation runs.
//!
//! Backends are interchangeable, the result is the same Node either way:
//! - `NodeCells`: the generic cons-cell list (data = first, value = rest) of any elements; index walks i links
//! - `TypedArray`: a list variable proven to hold only ints (floats) is an `$IntList` (`$FloatList`), a length and a
//!   wasm GC `(array (mut i64))` (`f64`) with spare capacity, see `find_typed_lists`: O(1) index and count, amortised
//!   O(1) append (`out = out + [x]`, which `map` lowers to, or `out.add(x)`), no box per element. Wherever the program needs it as a value (a
//!   result, an argument, a print) it becomes the same Node list the literal would have built (`int_list_as_node`).
//! - `Host`: reserved for a host-native or GPU implementation behind a host import, taking over above
//!   `HOST_BACKEND_MIN_LENGTH` at run time; none exists yet (no wasm SIMD, user decision). notes/typed_lists.md
//!
//! A list is a reference (P200b, card shared-lists): `ys = xs` makes both variables hold the one typed list, an item
//! added or set through either is in both; `ys = ys + [v]` makes a new list, `ys += [v]` (what `ys.add(v)` lowers to)
//! grows it in place. A typed list never meets a Node holder of itself, whose copy would part from it: a list another
//! variable holds as Nodes, or one used whole (an argument, an item, a field) stays the Node list every holder shares.

use super::map_backend::is_update;
use super::WasmGcEmitter;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use crate::type_kinds::Kind;
use std::collections::{HashMap, HashSet};
use wasm_encoder::*;
use Instruction as I;
use ValType::Ref;

/// The field of a typed list struct holding its array (type_manager emit_typed_list_types: length, items)
const TYPED_LIST_ITEMS: u32 = 1;

/// Capacity of the first items array a push into an empty list allocates; it doubles when full
const FIRST_CAPACITY: i32 = 4;
const NODE_COUNT: &str = "node_count";
const SQUARE_BRACKET_INFO: i64 = 1;

/// The list operations that go through the dispatch layer
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ListOp {
	/// `xs#i` as a number or as a Node
	Element,
	/// `xs#i = v`, `xs#i += v`
	SetElement,
	/// `sum xs`: `list_sum(xs, loop)`
	Sum,
}

/// The representation an emission context wants a number in
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Wanted {
	Int,
	Float,
	Node,
}

/// What the fast int sum returns when an element or the sum leaves the fixnum range: the exact loop takes over. It is a
/// handle (big_int.rs), never a fixnum the fast path could have produced.
const SUM_NOT_FIXNUM: i64 = i64::MIN;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ElementType {
	Int,
	Float,
	/// any element, as its Node: `$NodeList`, for list variables that are indexed, counted or iterated
	Node,
}

/// The runtime functions of one element type
struct ElementRuntime {
	at: &'static str,
	set: &'static str,
	copy: &'static str,
	as_node: &'static str,
	/// as_node_list(list) -> $NodeList: the element nodes in an array, what a list parameter of the array convention takes
	as_node_list: &'static str,
	filled: &'static str,
	push: &'static str,
	sum: &'static str,
	/// the constructor of one element's Node
	new_node: &'static str,
}

const INT_RUNTIME: ElementRuntime = ElementRuntime {
	at: "int_list_at", set: "int_list_set", copy: "int_list_copy", as_node: "int_list_as_node", as_node_list: "int_list_as_node_list", filled: "int_list_filled",
	push: "int_list_push", sum: "int_list_sum", new_node: "new_int",
};
const FLOAT_RUNTIME: ElementRuntime = ElementRuntime {
	at: "float_list_at", set: "float_list_set", copy: "float_list_copy", as_node: "float_list_as_node", as_node_list: "float_list_as_node_list",
	filled: "float_list_filled", push: "float_list_push", sum: "float_list_sum", new_node: "new_float",
};
const NODE_RUNTIME: ElementRuntime = ElementRuntime {
	at: "node_list_at", set: "node_list_set", copy: "node_list_copy", as_node: "node_list_as_node", as_node_list: "",
	filled: "node_list_filled", push: "node_list_push", sum: "", new_node: "",
};
/// node_list_of(node) -> $NodeList: the items of a Node list in an array, what a Node list variable starts from
const NODE_LIST_OF: &str = "node_list_of";

impl ElementType {
	const ALL: [ElementType; 2] = [ElementType::Int, ElementType::Float];

	fn runtime(self) -> &'static ElementRuntime {
		match self {
			ElementType::Int => &INT_RUNTIME,
			ElementType::Float => &FLOAT_RUNTIME,
			ElementType::Node => &NODE_RUNTIME,
		}
	}

	fn kind(self) -> Kind {
		match self {
			ElementType::Int => Kind::Int,
			ElementType::Float => Kind::Float,
			ElementType::Node => Kind::Data,
		}
	}

	fn val_type(self) -> ValType {
		match self {
			ElementType::Int => ValType::I64,
			ElementType::Float => ValType::F64,
			ElementType::Node => unreachable!("Node elements are references (emit_node_list_runtime)"),
		}
	}
}

/// An implementation of the list operations
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Backend {
	NodeCells,
	TypedArray(ElementType),
}

/// A list variable held as a typed array
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TypedList {
	pub element: ElementType,
	/// changed in place somewhere (an item set, an append) through it or a variable holding the same list
	pub updated: bool,
	/// held by another variable too (`ys = xs`): `xs = xs + [v]` makes a new list instead of growing the shared one
	pub aliased: bool,
}

/// An append to a list variable or an assignment of one list variable to another, numbered in the order it runs,
/// with the loops around it
struct ListEvent {
	append: bool,
	order: usize,
	loops: Vec<usize>,
}

/// Where a typed list variable lives: a local of the body, or a global main builds and functions read
#[derive(Clone, Copy, Debug)]
pub(super) enum Slot {
	Local(u32),
	Global(u32),
}

impl Slot {
	fn get(self) -> Instruction<'static> {
		match self {
			Slot::Local(index) => I::LocalGet(index),
			Slot::Global(index) => I::GlobalGet(index),
		}
	}

	/// Store the list on the stack and leave it there
	fn tee(self, func: &mut Function) {
		match self {
			Slot::Local(index) => { func.instruction(&I::LocalTee(index)); }
			Slot::Global(index) => { func.instruction(&I::GlobalSet(index)); func.instruction(&I::GlobalGet(index)); }
		}
	}
}

/// Where the value a list variable is assigned comes from
enum Source {
	/// `[1 2 3]`, `int[100]`
	Literal(ElementType),
	/// `[]`, `ø`: fits a list of any element type
	Empty,
	/// `xs = xs + [4 5]`, `xs += [4]` (what `xs.add(4)` lowers to)
	Append(ElementType),
	Variable(String),
	/// a list held as Nodes (a call's result, a parameter, a field): converted once into an array of its items. Fresh:
	/// a call's result, which no one else holds
	Converted { fresh: bool },
	Other,
}

/// The list and the loop of `list_sum(list, loop)`, what `sum xs` lowers to (library_words.rs)
pub(super) fn list_sum_parts(node: &Node) -> Option<(&Node, &Node)> {
	match node.drop_meta() {
		Node::List(items, _, _) => list_sum_call(items),
		_ => None,
	}
}

pub(super) fn list_sum_call(items: &[Node]) -> Option<(&Node, &Node)> {
	match items {
		[call, list, sum_loop] if matches!(call.drop_meta(), Node::Symbol(name) if name == crate::library_words::LIST_SUM) => Some((list, sum_loop)),
		_ => None,
	}
}

/// The count and zero of `zero_fill(count, zero)`, what `x : 100 int` and `int[100]` lower to
fn zero_fill_parts(value: &Node) -> Option<(&Node, &Node)> {
	match value.drop_meta() {
		Node::List(items, _, _) => match items.as_slice() {
			[call, count, zero] if matches!(call.drop_meta(), Node::Symbol(name) if name == crate::analyzer::ZERO_FILL_CALL) => Some((count, zero)),
			_ => None,
		},
		_ => None,
	}
}

/// `x = (t = x; …; t)`, an inlined call (inlining.rs) handing x back: the temporary t, which may share x's array; only
/// the inliner's temporaries, which nothing else reads, and only when the block hands back the very variable it got
fn moved_through(owner: &str, statements: &[Node], last: &Node) -> Option<String> {
	let Node::Symbol(temporary) = last.drop_meta() else { return None };
	if !crate::inlining::is_temporary(temporary) {
		return None;
	}
	let given = statements.iter().any(|statement| matches!(statement.drop_meta(), Node::Key(target, Op::Assign, value)
		if matches!(target.drop_meta(), Node::Symbol(target) if target == temporary) && matches!(value.drop_meta(), Node::Symbol(source) if source == owner)));
	given.then(|| temporary.clone())
}

/// `{a:1}`, `(1, 2)`, `(1 2)`, not the call `(f x)`
fn is_tuple_or_object(value: &Node) -> bool {
	use crate::node::Separator;
	match value.drop_meta() {
		Node::List(_, Bracket::Curly, _) | Node::List(_, Bracket::Round, Separator::Colon) => true,
		Node::List(items, Bracket::Round, Separator::Space | Separator::None) => !matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))),
		_ => false,
	}
}

/// The list variables a program indexes (`xs#i`, `xs#i = v`) or counts (`#xs`), as a for loop over them does
fn indexed_lists(program: &Node) -> HashSet<String> {
	let key_variables = crate::analyzer::key_variables(program);
	let mut indexed = HashSet::new();
	program.visit(&mut |part| {
		let Node::Key(list, Op::Hash, index) = part else { return };
		let counted = if matches!(list.drop_meta(), Node::Empty) { index } else { list };
		let by_key = crate::analyzer::looks_up_a_key(index, &key_variables);
		if let (Node::Symbol(name), false) = (counted.drop_meta(), by_key) {
			indexed.insert(name.clone());
		}
	});
	indexed
}

/// The items of `xs += [a b]`
fn extended_items(value: &Node) -> Option<&[Node]> {
	match value.drop_meta() {
		Node::List(items, Bracket::Square, _) if !items.is_empty() => Some(items),
		_ => None,
	}
}

/// Each list variable with every variable holding the same list: those assigned one another (`ys = xs`), transitively
fn alias_groups(sources: &HashMap<String, Vec<Source>>) -> HashMap<String, Vec<String>> {
	let mut neighbours: HashMap<&str, Vec<&str>> = HashMap::new();
	for (name, sources) in sources {
		neighbours.entry(name).or_default();
		for source in sources {
			if let Source::Variable(other) = source {
				neighbours.entry(name).or_default().push(other);
				neighbours.entry(other).or_default().push(name);
			}
		}
	}
	let mut groups = HashMap::new();
	for start in neighbours.keys() {
		if groups.contains_key(*start) {
			continue;
		}
		let (mut group, mut pending) = (vec![start.to_string()], vec![*start]);
		while let Some(name) = pending.pop() {
			for next in &neighbours[name] {
				if !group.iter().any(|member| member == next) {
					group.push(next.to_string());
					pending.push(next);
				}
			}
		}
		for member in &group {
			groups.insert(member.clone(), group.clone());
		}
	}
	groups
}

/// The element type every source agrees on: `Err` when a source is no typed list or they disagree, `Ok(None)` when only
/// variables of yet unknown type feed it
fn agreed_element(sources: &[Source], typed: &HashMap<String, Option<ElementType>>, globals: &HashMap<String, (u32, TypedList)>) -> Result<Option<ElementType>, ()> {
	let mut agreed = None;
	for source in sources {
		let element = match source {
			Source::Literal(element) | Source::Append(element) => Some(*element),
			Source::Empty => None,
			// an untyped list variable converts like any Node list
			Source::Variable(name) => typed.get(name).copied().or_else(|| globals.get(name).map(|(_, list)| Some(list.element))).unwrap_or(Some(ElementType::Node)),
			Source::Converted { .. } => Some(ElementType::Node),
			Source::Other => return Err(()),
		};
		match (agreed, element) {
			// numbers and Nodes mixed: Nodes
			(Some(a), Some(b)) if a != b && (a == ElementType::Node || b == ElementType::Node) => agreed = Some(ElementType::Node),
			(Some(a), Some(b)) if a != b => return Err(()),
			(None, element) => agreed = element,
			_ => {}
		}
	}
	Ok(agreed)
}

impl Source {
	fn is_literal(&self) -> bool {
		matches!(self, Source::Literal(_))
	}
}

impl WasmGcEmitter {
	/// An element provably of the element type: a number literal, a variable, a call of a function returning one, or
	/// arithmetic of numbers. Not by the type alone: it defaults to Int for anything unknown (ø, a key)
	fn is_element(&self, item: &Node, element: ElementType) -> bool {
		element == ElementType::Node || (self.get_type(item) == element.kind() && self.is_number_expression(item))
	}

	fn is_number_expression(&self, item: &Node) -> bool {
		let is_number = |operand: &Node| matches!(self.get_type(operand), Kind::Int | Kind::Float) && self.is_number_expression(operand);
		match item.drop_meta() {
			Node::Number(_) | Node::Symbol(_) => true,
			// `1.5f` is `1.5 as float`
			Node::Key(value, Op::As, _) => is_number(value),
			Node::Key(left, op, right) if op.is_arithmetic() || op.is_prefix() => {
				(matches!(left.drop_meta(), Node::Empty) || is_number(left)) && is_number(right)
			}
			Node::List(items, _, _) => matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if self.ctx.user_functions.contains_key(name)),
			_ => false,
		}
	}

	/// The element type all `items` share
	fn items_element(&self, items: &[Node]) -> Option<ElementType> {
		let first = items.first()?;
		ElementType::ALL.into_iter().find(|element| self.is_element(first, *element) && items.iter().all(|item| self.is_element(item, *element)))
	}

	/// The source of `target = value`
	/// Whether an append to `name` (`xs = xs + [v]`) can run while another variable holds its list: after an assignment
	/// between it and another (`ys = xs`, `xs = ys`) or in a loop around one. A list aliased only once its appends are
	/// done (`d = (out = ø; for … { out = out + [x] }; out)`, every lowered map and filter) grows in place (card map-typed)
	fn appends_while_aliased(&self, program: &Node, name: &str) -> bool {
		let mut events = vec![];
		self.list_events(program, name, &mut vec![], &mut 0, &mut events);
		let runs_after = |append: &ListEvent, alias: &ListEvent| append.order > alias.order || append.loops.iter().any(|ring| alias.loops.contains(ring));
		events.iter().filter(|alias| !alias.append).any(|alias| events.iter().any(|append| append.append && runs_after(append, alias)))
	}

	/// The appends to `name` and the assignments between it and another variable, in the order they run (an assignment
	/// after its value), each with the loops around it
	fn list_events(&self, node: &Node, name: &str, loops: &mut Vec<usize>, count: &mut usize, events: &mut Vec<ListEvent>) {
		let node = node.drop_meta();
		let is_loop = matches!(node, Node::Key(condition, Op::Do, _) if matches!(condition.drop_meta(), Node::Key(_, Op::While, _)));
		if is_loop {
			loops.push(*count);
			*count += 1;
		}
		match node {
			Node::Key(left, _, right) => [left, right].into_iter().for_each(|part| self.list_events(part, name, loops, count, events)),
			Node::List(items, _, _) => items.iter().for_each(|item| self.list_events(item, name, loops, count, events)),
			_ => {}
		}
		let append = match node {
			Node::Key(target, Op::Assign | Op::Define, value) => match target.drop_meta() {
				Node::Symbol(target) => match self.number_source_of(target, value) {
					Source::Append(_) if target == name => Some(true),
					Source::Variable(other) if target == name || other == name => Some(false),
					_ => None,
				},
				_ => None,
			},
			_ => None,
		};
		if let Some(append) = append {
			events.push(ListEvent { append, order: *count, loops: loops.clone() });
			*count += 1;
		}
		if is_loop {
			loops.pop();
		}
	}

	fn source_of(&self, target: &str, value: &Node) -> Source {
		match self.number_source_of(target, value) {
			// not a tuple `(1, 2)` or an object `{a:1}`, whose brackets a typed list would lose
			Source::Other if self.get_type(value) == Kind::List && !is_tuple_or_object(value) => Source::Converted { fresh: super::list_abi::called_function(value).is_some() },
			source => source,
		}
	}

	fn number_source_of(&self, target: &str, value: &Node) -> Source {
		let literal = |items: &[Node]| self.items_element(items).map_or(Source::Other, Source::Literal);
		match value.drop_meta() {
			// `(a = …; b)`, an inlined call: worth its last value
			Node::List(items, Bracket::Round, crate::node::Separator::Semicolon | crate::node::Separator::Newline) if !items.is_empty() => {
				self.source_of(target, &items[items.len() - 1])
			}
			Node::Empty => Source::Empty,
			Node::List(items, Bracket::Square, _) if items.is_empty() => Source::Empty,
			Node::Key(list, Op::Add, appended) if matches!(list.drop_meta(), Node::Symbol(name) if name == target) => match appended.drop_meta() {
				Node::List(items, Bracket::Square, _) if !items.is_empty() => Source::Append(self.items_element(items).unwrap_or(ElementType::Node)),
				_ => Source::Other,
			},
			Node::Symbol(name) => Source::Variable(name.clone()),
			_ if zero_fill_parts(value).is_some() => literal(std::slice::from_ref(zero_fill_parts(value).expect("guarded").1)),
			Node::List(items, Bracket::Square, _) => literal(items),
			_ => Source::Other,
		}
	}

	/// The list variables of the current body that can be typed arrays: locals of kind List that are only ever assigned
	/// int (float) literal lists, the empty list, numbers appended to themselves or other such variables, never updated
	/// by another operator (`xs -= …`), not captured by a function, no global, not assigned inside a `try`; a float list
	/// only ever gets floats by index. Lists are shared (P200b): a list some holder keeps as Nodes stays Nodes once it
	/// changes (list_sharing.rs). Any other use reads them as Nodes, so it stays correct.
	pub(super) fn find_typed_lists(&self, program: &Node) -> HashMap<String, TypedList> {
		let mut sources: HashMap<String, Vec<Source>> = HashMap::new();
		let mut excluded: HashSet<String> = HashSet::new();
		let mut updated: HashSet<String> = HashSet::new();
		let mut assigned_kinds: HashMap<String, Vec<Kind>> = HashMap::new();
		// `xs = float[n]`: declared floats, so any number written into it is stored as one
		let mut declared_floats: HashSet<String> = HashSet::new();
		// `row = grid#i`, `ys = p.xs`: a typed list read from a field or item, an array copy of a list others hold
		let mut place_sourced: HashSet<String> = HashSet::new();
		program.visit(&mut |part| {
			let Node::Key(target, op, value) = part else { return };
			match target.drop_meta() {
				Node::Symbol(name) if matches!(op, Op::Assign | Op::Define) => {
					if matches!(value.drop_meta(), Node::Key(_, Op::Hash | Op::Dot, _)) {
						place_sourced.insert(name.clone());
					}
					if zero_fill_parts(value).is_some_and(|(_, zero)| self.items_element(std::slice::from_ref(zero)) == Some(ElementType::Float)) {
						declared_floats.insert(name.clone());
					}
					sources.entry(name.clone()).or_default().push(self.source_of(name, value));
					if super::list_sharing::changes_in_place(value) {
						updated.insert(name.clone());
					}
				}
				Node::Symbol(name) if *op == Op::AddAssign && extended_items(value).is_some() => {
					let items = extended_items(value).expect("guarded");
					sources.entry(name.clone()).or_default().push(Source::Append(self.items_element(items).unwrap_or(ElementType::Node)));
				}
				Node::Symbol(name) if is_update(op) => {
					excluded.insert(name.clone());
					updated.insert(name.clone());
				}
				Node::Key(list, Op::Hash, index) if matches!(op, Op::Assign | Op::Define) || is_update(op) => {
					let Node::Symbol(name) = list.drop_meta() else { return };
					if self.map_key(index).is_some() || self.dynamic_key(index).is_some() {
						excluded.insert(name.clone());
						return;
					}
					updated.insert(name.clone());
					let assigned = if is_update(op) { Node::Key(target.clone(), op.base_op(), value.clone()) } else { value.as_ref().clone() };
					assigned_kinds.entry(name.clone()).or_default().push(self.get_type(&assigned));
				}
				_ => {}
			}
		});
		program.visit(&mut |part| {
			if matches!(part, Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if word == super::RAN_WITHOUT_ERROR)) {
				part.visit(&mut |inner| if let Node::Key(target, _, _) = inner {
					if let Node::Symbol(name) = target.drop_meta() { excluded.insert(name.clone()); }
				});
			}
		});
		excluded.extend(self.ctx.user_globals.keys().cloned());
		excluded.extend(self.ctx.captures.values().flatten().map(|(name, _)| name.clone()));
		for (name, sources) in &sources {
			if sources.iter().any(|source| matches!(source, Source::Append(_))) {
				updated.insert(name.clone());
			}
		}
		let groups = alias_groups(&sources);
		let group_updated = |name: &String| groups.get(name).is_some_and(|group| group.iter().any(|member| updated.contains(member)));
		let held = super::list_sharing::held_elsewhere(program, &self.ctx.user_functions);
		let written = super::list_sharing::written_roots(program, &self.ctx.user_functions);
		loop {
			let typed = self.typed_list_elements(&sources, &excluded, &declared_floats, &assigned_kinds);
			// such a copy is no snapshot only while no list but the typed arrays is changed in place (shared-lists-typed):
			// `ys = p.xs; p.xs#1 = 7` shares the list instead
			let writes_shared_lists = written.iter().any(|root| !typed.contains_key(root));
			let snapshots: Vec<String> = typed.keys().filter(|name| writes_shared_lists && place_sourced.contains(*name)).cloned().collect();
			if !snapshots.is_empty() {
				excluded.extend(snapshots);
				continue;
			}
			// a changing list is no typed array where a holder keeps it as Nodes: an untyped variable or one of another
			// element type holding the same list, a field or a call's argument it came from or goes to
			let element_of = |name: &String| typed.get(name).copied().flatten().or_else(|| self.typed_globals.get(name).map(|(_, list)| list.element));
			let shared_as_nodes = |name: &String| held.contains(name) || (group_updated(name) && (
				groups[name].iter().any(|member| element_of(member).is_none() || element_of(member) != element_of(name))
				|| sources[name].iter().any(|source| matches!(source, Source::Converted { fresh: false }))));
			let parted: Vec<String> = typed.keys().filter(|name| shared_as_nodes(name)).cloned().collect();
			if parted.is_empty() {
				// a Node list pays a conversion wherever it is used as a whole: worth it for one that is indexed or counted, and
				// for one appended to (map, filter), which as cons cells walks the whole list per append
				let indexed = indexed_lists(program);
				let appended = |name: &String| sources[name].iter().any(|source| matches!(source, Source::Append(_)));
				return typed.into_iter()
					.filter(|(name, element)| *element != Some(ElementType::Node) || indexed.contains(name) || appended(name))
					.filter_map(|(name, element)| {
						let aliased = groups[&name].len() > 1 && self.appends_while_aliased(program, &name);
						let list = TypedList { element: element?, updated: group_updated(&name), aliased };
						Some((name, list))
					})
					.collect();
			}
			excluded.extend(parted);
		}
	}

	/// The element type of each list variable that can be a typed array, `None` while only variables of yet unknown
	/// type feed it
	fn typed_list_elements(&self, sources: &HashMap<String, Vec<Source>>, excluded: &HashSet<String>, declared_floats: &HashSet<String>,
		assigned_kinds: &HashMap<String, Vec<Kind>>) -> HashMap<String, Option<ElementType>> {
		// a variable first assigned ø and appended to only in a branch (`out = ø; … if c {out = out + [x]}`) has the kind ø
		let appended = |name: &String| sources[name].iter().any(|source| matches!(source, Source::Append(_)));
		let is_list_local = |name: &String| self.scope.lookup(name).is_some_and(|local| !local.is_param && (local.kind == Kind::List || (local.kind == Kind::Empty && appended(name))));
		let has_start = |name: &String| sources[name].iter().any(|source| !matches!(source, Source::Append(_)));
		// an index assignment stores what node_with_node_at would: any exact number into an int list, only floats into a
		// float list, any number into a declared float array (converted)
		let takes_assignments = |name: &String, element: ElementType| {
			let takes = |kind: &Kind| match element {
				ElementType::Float => *kind == Kind::Float || (*kind == Kind::Int && declared_floats.contains(name)),
				ElementType::Int => *kind == Kind::Int,
				ElementType::Node => true,
			};
			assigned_kinds.get(name).is_none_or(|kinds| kinds.iter().all(takes))
		};
		let mut typed: HashMap<String, Option<ElementType>> = sources.keys()
			.filter(|name| is_list_local(name) && has_start(name) && !excluded.contains(*name))
			.map(|name| (name.clone(), None))
			.collect();
		// greatest fixpoint: drop a variable fed by something untyped, until all that remain agree
		loop {
			let mut changed = false;
			for name in typed.keys().cloned().collect::<Vec<_>>() {
				match agreed_element(&sources[&name], &typed, &self.typed_globals) {
					Ok(Some(element)) if !takes_assignments(&name, element) => { typed.remove(&name); changed = true; }
					Err(()) => { typed.remove(&name); changed = true; }
					// a source that turned into Nodes (its variable left the typed lists) widens an int list to Nodes
					Ok(Some(element)) if typed[&name] != Some(element) => { typed.insert(name, Some(element)); changed = true; }
					Ok(_) => {}
				}
			}
			if !changed {
				return typed;
			}
		}
	}

	/// Push what the typed capture global of main's typed list `name` takes: the array, a copy when main changes it later,
	/// so the function keeps what it captured. False for any other capture global.
	pub(super) fn emit_typed_capture(&mut self, func: &mut Function, name: &str, global: u32) -> bool {
		let Some(&captured) = self.typed_capture_globals.get(&global) else { return false };
		let (slot, list) = self.typed_list(&Node::Symbol(name.to_string())).expect("a typed capture global takes main's typed list");
		func.instruction(&slot.get());
		if list.updated {
			self.emit_call(func, captured.element.runtime().copy);
		}
		true
	}

	/// A null-initialized mutable global holding a typed list
	pub(super) fn declare_typed_list_global(&mut self, element: ElementType) -> u32 {
		let list_type = self.typed_list_types(element).1;
		self.declare_global_of(Ref(self.typed_list_ref(element)), ConstExpr::ref_null(HeapType::Concrete(list_type)))
	}

	/// The globals (main's variables that functions read, `global` ones) held as typed arrays: what find_typed_lists finds
	/// for them as main's locals, when no function replaces them, so all their sources are main's. A function may write
	/// their items (`s#i = v`) with values of the element type: `global s = int[n]; def f() { s#i = 1 }` keeps the array
	pub(super) fn find_typed_globals(&mut self, program: &Node, main: &crate::analyzer::Scope) -> HashMap<String, TypedList> {
		if main.globals.is_empty() {
			return HashMap::new();
		}
		let mut as_locals = main.clone();
		for (name, global) in &main.globals {
			as_locals.locals.entry(name.clone()).or_insert_with(|| crate::local::Local { is_param: false, ..global.clone() });
		}
		let saved = std::mem::replace(&mut self.scope, as_locals);
		let main_code = self.without_definitions(program); // function bodies are checked below, each in its own scope
		let mut found: HashMap<String, TypedList> = self.find_typed_lists(&main_code).into_iter().filter(|(name, _)| main.globals.contains_key(name)).collect();
		self.scope = saved;
		let function_kinds = self.user_function_kinds();
		let functions: Vec<_> = self.ctx.user_functions.values().map(|function| (function.params.clone(), function.body.clone())).collect();
		for (params, body) in functions {
			let scope = crate::analyzer::function_body_scope(&params, &body, &function_kinds, &main.globals, &self.ctx.closure_variable_targets);
			let saved = std::mem::replace(&mut self.scope, scope);
			found.retain(|name, list| !self.replaces(&body, name, list.element) && self.item_writes_fit(&body, name, list));
			self.scope = saved;
		}
		found
	}

	/// The program's statements without its function definitions and imports
	fn without_definitions(&self, program: &Node) -> Node {
		match program.drop_meta() {
			Node::List(items, bracket, separator) => Node::List(items.iter().filter(|item| !self.is_definition(item)).cloned().collect(), bracket.clone(), separator.clone()),
			other => other.clone(),
		}
	}

	/// Whether `body` assigns or updates the typed list `name` itself or a field of it (`name.x = v`), other than with a
	/// new array of its element type (`canvas_pixels = int[w * h]`); its items are item_writes_fit's
	fn replaces(&self, body: &Node, name: &str, element: ElementType) -> bool {
		let mut found = false;
		body.visit(&mut |part| if let Node::Key(target, op, value) = part {
			let target = match target.drop_meta() {
				Node::Key(owner, Op::Dot, _) => owner.drop_meta(),
				other => other,
			};
			let new_array = *op == Op::Assign && zero_fill_parts(value).is_some_and(|(_, zero)| self.items_element(std::slice::from_ref(zero)) == Some(element));
			found |= (matches!(op, Op::Assign | Op::Define) || is_update(op)) && !new_array && matches!(target, Node::Symbol(assigned) if assigned == name);
		});
		found
	}

	/// Whether every item write of `body` to the typed list `name` stores a value of its element type at an index (not a
	/// map key); marks the list updated when there is one
	fn item_writes_fit(&self, body: &Node, name: &str, list: &mut TypedList) -> bool {
		let mut fits = true;
		body.visit(&mut |part| {
			let Node::Key(target, op, value) = part else { return };
			let Node::Key(owner, Op::Hash, index) = target.drop_meta() else { return };
			if !(matches!(op, Op::Assign | Op::Define) || is_update(op)) || !matches!(owner.drop_meta(), Node::Symbol(written) if written == name) {
				return;
			}
			list.updated = true;
			let assigned = if is_update(op) { Node::Key(target.clone(), op.base_op(), value.clone()) } else { value.as_ref().clone() };
			let kind = self.get_type(&assigned);
			let stored = match list.element {
				ElementType::Float => kind == Kind::Float,
				ElementType::Int => kind == Kind::Int,
				ElementType::Node => true,
			};
			fits &= stored && self.map_key(index).is_none() && self.dynamic_key(index).is_none();
		});
		fits
	}

	/// The slot and element type of `target` when it is a list variable held as a typed array: a local of the body, else
	/// a typed global
	fn typed_list(&self, target: &Node) -> Option<(Slot, TypedList)> {
		let Node::Symbol(name) = target.drop_meta() else { return None };
		match self.scope.lookup(name) {
			Some(local) => Some((Slot::Local(local.position), *self.typed_lists.get(name)?)),
			None => self.typed_globals.get(name).map(|&(global, list)| (Slot::Global(global), list)),
		}
	}

	/// The backend that runs `op` on `target`. A host backend would be picked here for its element types, with the
	/// length test against HOST_BACKEND_MIN_LENGTH emitted around the call.
	pub(super) fn list_backend(&self, _op: ListOp, target: &Node) -> Backend {
		match self.typed_list(target) {
			Some((_, list)) => Backend::TypedArray(list.element),
			None => Backend::NodeCells,
		}
	}

	/// The (array, list) type indices of an element type
	pub(super) fn typed_list_types(&self, element: ElementType) -> (u32, u32) {
		let types = &self.type_manager;
		match element {
			ElementType::Int => (types.int_array_type, types.int_list_type),
			ElementType::Float => (types.float_array_type, types.float_list_type),
			ElementType::Node => (types.node_array_type, types.node_list_type),
		}
	}

	pub(super) fn typed_list_ref(&self, element: ElementType) -> RefType {
		RefType { nullable: true, heap_type: HeapType::Concrete(self.typed_list_types(element).1) }
	}

	/// How the local of a variable is stored: a typed list as its list struct, anything else by its kind
	pub(super) fn local_storage_type(&self, name: &str, kind: Kind) -> ValType {
		if self.typed_maps.contains(name) {
			return Ref(self.node_map_ref());
		}
		if let Some(class) = self.typed_structs.get(name) {
			return Ref(self.instance_ref(class));
		}
		match self.typed_lists.get(name) {
			Some(list) => Ref(self.typed_list_ref(list.element)),
			None => self.storage_type(kind),
		}
	}

	/// Push the element count (i64) of `target`; `counter` is the runtime counter of the generic backend (node_count, node_bytes …)
	pub(super) fn emit_list_count(&mut self, func: &mut Function, target: &Node, counter: &'static str) {
		// `(statements; xs)` (a lowered filter): the statements run, a typed list xs is counted without making its nodes
		if let Node::List(items, Bracket::Round, crate::node::Separator::Semicolon | crate::node::Separator::Newline) = target.drop_meta() {
			if let Some((last, statements)) = items.split_last().filter(|(last, _)| self.typed_list(last).is_some()) {
				self.emit_discarded_statements(func, statements);
				return self.emit_list_count(func, last, counter);
			}
		}
		if let (Some(slot), NODE_COUNT) = (self.typed_map(target), counter) {
			return self.emit_typed_map_count(func, slot);
		}
		match (self.typed_list(target), counter) {
			(Some((slot, list)), NODE_COUNT) => {
				let (_, list_type) = self.typed_list_types(list.element);
				Self::emit_list(func, &[slot.get(), I::StructGet { struct_type_index: list_type, field_index: 0 }, I::I64ExtendI32U]);
			}
			_ => {
				self.emit_node_instructions(func, target);
				self.emit_call(func, counter);
			}
		}
	}

	/// Push the number (i64) at the 1-based `index` of `target`; a float element takes the generic way, which fails as
	/// it always did
	pub(super) fn emit_list_element_number(&mut self, func: &mut Function, target: &Node, index: &Node) {
		self.emit_integral_index_check(func, index);
		match self.list_backend(ListOp::Element, target) {
			Backend::TypedArray(ElementType::Int) => self.emit_typed_element(func, target, index),
			Backend::TypedArray(ElementType::Node) => {
				self.emit_typed_element(func, target, index);
				self.emit_call(func, "get_int_value");
			}
			_ => {
				self.emit_node_instructions(func, target);
				self.emit_numeric_value(func, index);
				self.emit_call(func, "list_at");
			}
		}
	}

	/// A typed array of Ints or Floats: its elements are numbers, never characters (P65)
	pub(super) fn holds_only_numbers(&self, target: &Node) -> bool {
		self.typed_list(target).is_some_and(|(_, list)| list.element != ElementType::Node)
	}

	pub(super) fn is_typed_list(&self, target: &Node) -> bool {
		self.typed_list(target).is_some()
	}

	/// Push the f64 at the 1-based `index` of a typed list `target`
	pub(super) fn emit_typed_element_float(&mut self, func: &mut Function, target: &Node, index: &Node) {
		let (_, list) = self.typed_list(target).expect("typed backend");
		self.emit_integral_index_check(func, index);
		self.emit_typed_element(func, target, index);
		match list.element {
			ElementType::Int => self.emit_int_to_f64(func, None),
			ElementType::Node => self.emit_call(func, super::list_ops::TEXT_AS_FLOAT),
			ElementType::Float => {}
		}
	}

	/// Push the Node at the 1-based `index` of `target` (a list element, or a character of a text)
	pub(super) fn emit_list_element_node(&mut self, func: &mut Function, target: &Node, index: &Node) {
		self.emit_integral_index_check(func, index);
		match self.list_backend(ListOp::Element, target) {
			Backend::TypedArray(element) => {
				self.emit_typed_element(func, target, index);
				if element != ElementType::Node {
					self.emit_call(func, element.runtime().new_node);
				}
			}
			_ => {
				self.emit_node_instructions(func, target);
				self.emit_numeric_value(func, index);
				self.emit_call(func, "node_index_at");
			}
		}
	}

	fn emit_typed_element(&mut self, func: &mut Function, target: &Node, index: &Node) {
		let (slot, list) = self.typed_list(target).expect("typed backend");
		func.instruction(&slot.get());
		if let Some(counter) = self.walking_counter(target, index).filter(|_| list.element != ElementType::Node) {
			// `x·items#(x·index + 1)` of a for loop: the counter is below the count, read the array directly
			let (array, list_type) = self.typed_list_types(list.element);
			Self::emit_list(func, &[I::StructGet { struct_type_index: list_type, field_index: TYPED_LIST_ITEMS }, I::LocalGet(counter), I::I32WrapI64, I::ArrayGet(array)]);
			return;
		}
		self.emit_numeric_value(func, index);
		self.emit_call(func, list.element.runtime().at);
	}

	/// The local of the counter when `index` is `x·index + 1` and `target` is `x·items`, the list a for loop walks
	/// (for_loop.rs): the loop runs while the counter is below the count, so the element exists
	fn walking_counter(&self, target: &Node, index: &Node) -> Option<u32> {
		let Node::Key(counter, Op::Add, one) = index.drop_meta() else { return None };
		let (Node::Symbol(counter), Node::Symbol(list)) = (counter.drop_meta(), target.drop_meta()) else { return None };
		let walked = counter.strip_suffix(crate::for_loop::INDEX_SUFFIX)?;
		let is_walk = matches!(one.drop_meta(), Node::Number(crate::extensions::numbers::Number::Int(1))) && *list == format!("{walked}{}", crate::for_loop::ITEMS_SUFFIX);
		(is_walk && self.bounded_counters.contains(counter) && !self.typed_lists[list].updated).then(|| self.scope.lookup(counter).map(|local| local.position)).flatten()
	}

	/// Emit `value` as an element of a typed list
	fn emit_element_value(&mut self, func: &mut Function, value: &Node, element: ElementType) {
		match element {
			ElementType::Int => self.emit_numeric_value(func, value),
			ElementType::Float => self.emit_float_value(func, value),
			ElementType::Node => self.emit_node_instructions(func, value),
		}
	}

	/// `target#index = value` on a typed list, in place: leaves the assigned value (i64; 0 for a float, which is no exact
	/// Int, as an entry assignment of a non-number leaves). False for any other list.
	pub(super) fn emit_typed_element_assignment(&mut self, func: &mut Function, target: &Node, index: &Node, value: &Node) -> bool {
		let Backend::TypedArray(element) = self.list_backend(ListOp::SetElement, target) else { return false };
		let (slot, _) = self.typed_list(target).expect("typed backend");
		func.instruction(&slot.get());
		self.emit_numeric_value(func, index);
		self.emit_element_value(func, value, element);
		if element != ElementType::Int {
			self.emit_call(func, element.runtime().set);
			func.instruction(&I::I64Const(0));
			return true;
		}
		let assigned = self.scratch(2);
		func.instruction(&I::LocalTee(assigned));
		self.emit_call(func, element.runtime().set);
		func.instruction(&I::LocalGet(assigned));
		true
	}

	/// `name = value` for a typed list variable: stores the list and leaves it on the stack. False for any other variable.
	pub(super) fn emit_typed_list_store(&mut self, func: &mut Function, name: &str, value: &Node) -> bool {
		if let Some(slot) = self.typed_map(&Node::Symbol(name.to_string())) {
			self.emit_typed_map_store(func, name, slot, value);
			return true;
		}
		if let Some((slot, _)) = self.typed_struct(&Node::Symbol(name.to_string())) {
			let class = self.typed_structs[name].clone();
			self.emit_typed_struct_store(func, name, slot, &class, value);
			return true;
		}
		let Some((slot, list)) = self.typed_list(&Node::Symbol(name.to_string())) else { return false };
		// `(statements; last)`: the statements run, the last value is stored
		if let Node::List(items, Bracket::Round, crate::node::Separator::Semicolon | crate::node::Separator::Newline) = value.drop_meta() {
			if let Some((last, statements)) = items.split_last() {
				let moved = moved_through(name, statements, last);
				if let Some(temporary) = &moved {
					self.moved_lists.push((name.to_string(), temporary.clone()));
				}
				self.emit_discarded_statements(func, statements);
				let stored = self.emit_typed_list_store(func, name, last);
				if moved.is_some() {
					self.moved_lists.pop();
				}
				return stored;
			}
		}
		let runtime = list.element.runtime();
		match value.drop_meta() {
			// the one list both variables hold (P200b)
			Node::Symbol(_) if self.typed_list(value).is_some_and(|(_, source)| source.element == list.element) => {
				let (source_slot, _) = self.typed_list(value).expect("guarded");
				func.instruction(&source_slot.get());
			}
			_ if zero_fill_parts(value).is_some() => {
				let (count, zero) = zero_fill_parts(value).expect("guarded");
				self.emit_numeric_value(func, count);
				self.emit_element_value(func, zero, list.element);
				self.emit_call(func, runtime.filled);
			}
			// `xs = xs + [v]`: a new list, unless no other variable holds this one
			Node::Key(_, Op::Add, appended) if matches!(self.number_source_of(name, value), Source::Append(_)) => {
				let Node::List(items, _, _) = appended.drop_meta() else { unreachable!("find_typed_lists admits only appended lists") };
				func.instruction(&slot.get());
				if list.aliased {
					self.emit_call(func, runtime.copy);
				}
				self.emit_typed_pushes(func, items, list.element);
			}
			Node::Empty => self.emit_typed_list_of(func, &[], list.element),
			Node::List(items, Bracket::Square, _) if self.number_source_of(name, value).is_literal() || items.is_empty() => {
				self.emit_typed_list_of(func, items, list.element)
			}
			// a call of a function giving back its array: that array, no conversion (list_abi.rs)
			_ if list.element == ElementType::Node && super::list_abi::called_function(value).is_some_and(|callee| self.returns_list_abi(callee)) => {
				self.emit_list_abi_value(func, value);
			}
			// a Node list (a call's result, a list parameter): its items once into the array
			_ if list.element == ElementType::Node => {
				self.emit_node_instructions(func, value);
				self.emit_call(func, NODE_LIST_OF);
			}
			other => unreachable!("find_typed_lists admits no {other:?}"),
		}
		slot.tee(func);
		true
	}

	/// `xs += [a b]` of a typed list: the items pushed onto its array in place, leaving the list. False for any other
	pub(super) fn emit_typed_list_extend(&mut self, func: &mut Function, name: &str, items: &[Node]) -> bool {
		let Some((slot, list)) = self.typed_list(&Node::Symbol(name.to_string())) else { return false };
		func.instruction(&slot.get());
		self.emit_typed_pushes(func, items, list.element);
		slot.tee(func);
		true
	}

	/// Push each item onto the typed list on the stack, leaving the list
	fn emit_typed_pushes(&mut self, func: &mut Function, items: &[Node], element: ElementType) {
		for item in items {
			self.emit_element_value(func, item, element);
			self.emit_call(func, element.runtime().push);
		}
	}

	/// A new typed list of `items`
	fn emit_typed_list_of(&mut self, func: &mut Function, items: &[Node], element: ElementType) {
		let (array, list) = self.typed_list_types(element);
		func.instruction(&I::I32Const(items.len() as i32));
		for item in items {
			self.emit_element_value(func, item, element);
		}
		func.instruction(&I::ArrayNewFixed { array_type_index: array, array_size: items.len() as u32 });
		if element == ElementType::Node {
			func.instruction(&I::I64Const(crate::type_kinds::SQUARE_LIST_KIND));
		}
		func.instruction(&I::StructNew(list));
	}

	/// The variable and value of an assignment to a typed list variable
	pub(super) fn typed_list_store(&self, statement: &Node) -> Option<(String, Node)> {
		let Node::Key(target, Op::Assign | Op::Define, value) = statement.drop_meta() else { return None };
		let Node::Symbol(name) = target.drop_meta() else { return None };
		(self.typed_list(target).is_some() || self.typed_maps.contains(name) || self.typed_structs.contains_key(name)).then(|| (name.clone(), value.as_ref().clone()))
	}

	/// The variable and items of `xs += [a b]` on a typed list variable
	pub(super) fn typed_list_extension(&self, statement: &Node) -> Option<(String, Vec<Node>)> {
		let Node::Key(target, Op::AddAssign, value) = statement.drop_meta() else { return None };
		let Node::Symbol(name) = target.drop_meta() else { return None };
		self.typed_list(target).and(extended_items(value)).map(|items| (name.clone(), items.to_vec()))
	}

	/// Push a typed list variable as the Node list it stands for; false for any other variable
	pub(super) fn emit_typed_list_as_node(&mut self, func: &mut Function, name: &str) -> bool {
		if let Some(slot) = self.typed_map(&Node::Symbol(name.to_string())) {
			self.emit_typed_map_as_node(func, slot);
			return true;
		}
		if self.typed_structs.contains_key(name) {
			self.emit_typed_struct_as_node(func, name);
			return true;
		}
		let Some((slot, list)) = self.typed_list(&Node::Symbol(name.to_string())) else { return false };
		func.instruction(&slot.get());
		self.emit_call(func, list.element.runtime().as_node);
		true
	}

	/// `list_sum(list, loop)` as `wanted`: a typed list sums its items in one runtime call (an int sum that leaves the
	/// fixnum range runs the exact loop instead), any other list runs the loop
	pub(super) fn emit_list_sum(&mut self, func: &mut Function, list: &Node, sum_loop: &Node, wanted: Wanted) {
		match (self.list_backend(ListOp::Sum, list), wanted) {
			(Backend::TypedArray(ElementType::Int), _) => {
				let (slot, _) = self.typed_list(list).expect("typed backend");
				let sum = self.scratch(0);
				func.instruction(&slot.get());
				self.emit_call(func, INT_RUNTIME.sum);
				Self::emit_list(func, &[I::LocalTee(sum), I::I64Const(SUM_NOT_FIXNUM), I::I64Eq, I::If(BlockType::Result(ValType::I64))]);
				self.emit_numeric_value(func, sum_loop);
				Self::emit_list(func, &[I::Else, I::LocalGet(sum), I::End]);
				match wanted {
					Wanted::Int => {}
					Wanted::Float => self.emit_int_to_f64(func, None),
					Wanted::Node => self.emit_call(func, INT_RUNTIME.new_node),
				}
			}
			(Backend::TypedArray(ElementType::Float), Wanted::Float | Wanted::Node) => {
				let (slot, _) = self.typed_list(list).expect("typed backend");
				func.instruction(&slot.get());
				self.emit_call(func, FLOAT_RUNTIME.sum);
				if wanted == Wanted::Node {
					self.emit_call(func, FLOAT_RUNTIME.new_node);
				}
			}
			(_, Wanted::Int) => self.emit_numeric_value(func, sum_loop),
			(_, Wanted::Float) => self.emit_float_value(func, sum_loop),
			(_, Wanted::Node) => self.emit_node_instructions(func, sum_loop),
		}
	}

	/// The runtime functions of the typed-array backend, each only when the program calls it
	pub(super) fn emit_typed_list_runtime(&mut self) {
		for element in ElementType::ALL {
			self.emit_element_runtime(element);
		}
		self.emit_node_list_runtime();
	}

	/// The runtime functions of `$NodeList`, each only when the program calls it: as the number lists, plus the kind of the
	/// list node it stands for, and node_list_of to start from a Node list
	fn emit_node_list_runtime(&mut self) {
		let (array, list) = self.typed_list_types(ElementType::Node);
		let list_ref = Ref(self.typed_list_ref(ElementType::Node));
		let array_ref = Ref(RefType { nullable: true, heap_type: HeapType::Concrete(array) });
		let node = self.type_manager.node_type;
		let node_ref = Ref(self.node_ref(false));
		let nullable_node = Ref(self.node_ref(true));
		let length = I::StructGet { struct_type_index: list, field_index: 0 };
		let items = I::StructGet { struct_type_index: list, field_index: 1 };
		let kind = I::StructGet { struct_type_index: list, field_index: 2 };
		let new_list = I::StructNew(list);
		if self.should_emit_function(NODE_RUNTIME.at) || self.should_emit_function(NODE_RUNTIME.set) {
			self.ensure_index_out_of_range_of();
		}
		if self.should_emit_function(NODE_RUNTIME.at) {
			self.runtime_function(NODE_RUNTIME.at, vec![list_ref, ValType::I64], vec![node_ref], vec![], |s, f| {
				s.emit_list_position(f, list);
				Self::emit_list(f, &[I::ArrayGet(array), I::RefAsNonNull]);
			});
		}
		if self.should_emit_function(NODE_RUNTIME.set) {
			self.runtime_function(NODE_RUNTIME.set, vec![list_ref, ValType::I64, node_ref], vec![], vec![], |s, f| {
				s.emit_list_position(f, list);
				Self::emit_list(f, &[I::LocalGet(2), I::ArraySet(array)]);
			});
		}
		if self.should_emit_function(NODE_RUNTIME.copy) {
			self.runtime_function(NODE_RUNTIME.copy, vec![list_ref], vec![list_ref], vec![array_ref], |_, f| {
				Self::emit_list(f, &[
					I::LocalGet(0), length.clone(), I::ArrayNewDefault(array), I::LocalSet(1),
					I::LocalGet(1), I::I32Const(0), I::LocalGet(0), items.clone(), I::I32Const(0), I::LocalGet(0), length.clone(),
					I::ArrayCopy { array_type_index_dst: array, array_type_index_src: array },
					I::LocalGet(0), length.clone(), I::LocalGet(1), I::RefAsNonNull, I::LocalGet(0), kind.clone(), new_list.clone(),
				]);
			});
		}
		if self.should_emit_function(NODE_RUNTIME.filled) {
			self.runtime_function(NODE_RUNTIME.filled, vec![ValType::I64, node_ref], vec![list_ref], vec![ValType::I32], |s, f| {
				Self::emit_list(f, &[I::LocalGet(0), I::I64Const(i32::MAX as i64), I::I64GtS]);
				s.emit_fail_if(f, "out_of_memory");
				Self::emit_list(f, &[
					I::LocalGet(0), I::I64Const(0), I::LocalGet(0), I::I64Const(0), I::I64GtS, I::Select, I::I32WrapI64, I::LocalTee(2),
					I::LocalGet(1), I::LocalGet(2), I::ArrayNew(array), I::I64Const(crate::type_kinds::SQUARE_LIST_KIND), new_list.clone(),
				]);
			});
		}
		if self.should_emit_function(NODE_RUNTIME.push) {
			self.runtime_function(NODE_RUNTIME.push, vec![list_ref, node_ref], vec![list_ref], vec![array_ref], |_, f| {
				let grown = 2;
				Self::emit_list(f, &[
					I::LocalGet(0), I::RefIsNull, I::If(BlockType::Empty),
					I::I32Const(0), I::I32Const(FIRST_CAPACITY), I::ArrayNewDefault(array), I::I64Const(crate::type_kinds::SQUARE_LIST_KIND), new_list.clone(), I::LocalSet(0),
					I::End,
					I::LocalGet(0), length.clone(), I::LocalGet(0), items.clone(), I::ArrayLen, I::I32Eq, I::If(BlockType::Empty),
					I::LocalGet(0), length.clone(), I::I32Const(1), I::I32Shl, I::I32Const(FIRST_CAPACITY), I::I32Add, I::ArrayNewDefault(array), I::LocalSet(grown),
					I::LocalGet(grown), I::I32Const(0), I::LocalGet(0), items.clone(), I::I32Const(0), I::LocalGet(0), length.clone(),
					I::ArrayCopy { array_type_index_dst: array, array_type_index_src: array },
					I::LocalGet(0), I::LocalGet(grown), I::RefAsNonNull, I::StructSet { struct_type_index: list, field_index: 1 },
					I::End,
					I::LocalGet(0), items.clone(), I::LocalGet(0), length.clone(), I::LocalGet(1), I::ArraySet(array),
					I::LocalGet(0), I::LocalGet(0), length.clone(), I::I32Const(1), I::I32Add, I::StructSet { struct_type_index: list, field_index: 0 },
					I::LocalGet(0),
				]);
			});
		}
		// as_node(list): the cons cells of the items under the list's own kind (brackets); ø when empty or unassigned
		if self.should_emit_function(NODE_RUNTIME.as_node) {
			self.runtime_function(NODE_RUNTIME.as_node, vec![list_ref], vec![node_ref], vec![ValType::I32, nullable_node], |s, f| {
				let (position, rest) = (1, 2);
				Self::emit_list(f, &[I::LocalGet(0), I::RefIsNull, I::If(BlockType::Empty)]);
				s.call(f, "new_empty");
				Self::emit_list(f, &[I::Return, I::End]);
				Self::emit_list(f, &[
					I::RefNull(HeapType::Concrete(node)), I::LocalSet(rest),
					I::LocalGet(0), length.clone(), I::LocalSet(position),
					I::Block(BlockType::Empty), I::Loop(BlockType::Empty),
					I::LocalGet(position), I::I32Eqz, I::BrIf(1),
					I::LocalGet(position), I::I32Const(1), I::I32Sub, I::LocalSet(position),
					I::LocalGet(0), kind.clone(),
					I::LocalGet(0), items.clone(), I::LocalGet(position), I::ArrayGet(array),
					I::LocalGet(rest), I::StructNew(node), I::LocalSet(rest),
					I::Br(0), I::End, I::End,
					I::LocalGet(rest), I::RefIsNull, I::If(BlockType::Result(node_ref)),
				]);
				s.call(f, "new_empty");
				Self::emit_list(f, &[I::Else, I::LocalGet(rest), I::RefAsNonNull, I::End]);
			});
		}
		// node_list_of(node): the items of a list's cells (up to a meta entry, as a walk ends there), ø no items, any
		// other node the one item, as node_count counts it
		if self.should_emit_function(NODE_LIST_OF) {
			let (cell, out, tag) = (1, 2, 3);
			self.runtime_function(NODE_LIST_OF, vec![node_ref], vec![list_ref], vec![nullable_node, list_ref, ValType::I64], |s, f| {
				Self::emit_list(f, &[I::RefNull(HeapType::Concrete(list)), I::LocalSet(out)]);
				s.emit_field(f, 0, 0);
				Self::emit_list(f, &[I::I64Const(crate::type_kinds::KIND_MASK), I::I64And, I::LocalTee(tag), I::I64Const(Kind::Empty as i64), I::I64Eq, I::If(BlockType::Empty)]);
				Self::emit_list(f, &[I::I32Const(0), I::I32Const(0), I::ArrayNewDefault(array), I::I64Const(crate::type_kinds::SQUARE_LIST_KIND), new_list.clone(), I::Return, I::End]);
				Self::emit_list(f, &[
					I::LocalGet(tag), I::I64Const(Kind::List as i64), I::I64Ne, I::LocalGet(tag), I::I64Const(Kind::Block as i64), I::I64Ne, I::I32And,
					I::If(BlockType::Empty), I::LocalGet(out), I::LocalGet(0),
				]);
				s.call(f, NODE_RUNTIME.push);
				Self::emit_list(f, &[I::Return, I::End]);
				// a list: walk its cells, the last one's value may be a single item
				Self::emit_list(f, &[I::LocalGet(0), I::LocalSet(cell), I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
				Self::emit_list(f, &[I::LocalGet(cell), I::StructGet { struct_type_index: node, field_index: 1 }]);
				s.call(f, super::equality::IS_META_ENTRY);
				Self::emit_list(f, &[I::BrIf(1), I::LocalGet(out), I::LocalGet(cell), I::StructGet { struct_type_index: node, field_index: 1 }, I::RefCastNonNull(HeapType::Concrete(node))]);
				s.call(f, NODE_RUNTIME.push);
				Self::emit_list(f, &[
					I::LocalSet(out),
					I::LocalGet(cell), I::StructGet { struct_type_index: node, field_index: 2 }, I::LocalTee(cell), I::RefIsNull, I::BrIf(1),
					I::LocalGet(cell), I::StructGet { struct_type_index: node, field_index: 0 }, I::I64Const(crate::type_kinds::KIND_MASK), I::I64And, I::LocalTee(tag),
					I::I64Const(Kind::List as i64), I::I64Eq, I::LocalGet(tag), I::I64Const(Kind::Block as i64), I::I64Eq, I::I32Or, I::BrIf(0),
					I::LocalGet(out), I::LocalGet(cell), I::RefAsNonNull,
				]);
				s.call(f, NODE_RUNTIME.push);
				Self::emit_list(f, &[I::LocalSet(out), I::End, I::End]);
				// the empty list ø and a list whose first item is a meta entry hold no items
				Self::emit_list(f, &[I::LocalGet(out), I::RefIsNull, I::If(BlockType::Empty),
					I::I32Const(0), I::I32Const(0), I::ArrayNewDefault(array), I::I64Const(crate::type_kinds::SQUARE_LIST_KIND), new_list.clone(), I::LocalSet(out), I::End]);
				f.instruction(&I::LocalGet(out));
				s.emit_field(f, 0, 0);
				Self::emit_list(f, &[I::StructSet { struct_type_index: list, field_index: 2 }, I::LocalGet(out)]);
			});
		}
	}

	fn emit_element_runtime(&mut self, element: ElementType) {
		let runtime = element.runtime();
		let value = element.val_type();
		let (array, list) = self.typed_list_types(element);
		let list_ref = Ref(self.typed_list_ref(element));
		let array_ref = Ref(RefType { nullable: true, heap_type: HeapType::Concrete(array) });
		let node_ref = Ref(self.node_ref(false));
		let length = I::StructGet { struct_type_index: list, field_index: 0 };
		let items = I::StructGet { struct_type_index: list, field_index: 1 };
		if self.should_emit_function(runtime.at) || self.should_emit_function(runtime.set) {
			self.ensure_index_out_of_range_of();
		}
		// at(list, index) -> element: the element at the 1-based index
		if self.should_emit_function(runtime.at) {
			self.runtime_function(runtime.at, vec![list_ref, ValType::I64], vec![value], vec![], |s, f| {
				s.emit_list_position(f, list);
				f.instruction(&I::ArrayGet(array));
			});
		}
		// set(list, index, value): the element at the 1-based index becomes value
		if self.should_emit_function(runtime.set) {
			self.runtime_function(runtime.set, vec![list_ref, ValType::I64, value], vec![], vec![], |s, f| {
				s.emit_list_position(f, list);
				Self::emit_list(f, &[I::LocalGet(2), I::ArraySet(array)]);
			});
		}
		// copy(list) -> list: its own items, no spare capacity
		if self.should_emit_function(runtime.copy) {
			self.runtime_function(runtime.copy, vec![list_ref], vec![list_ref], vec![array_ref], |_, f| {
				Self::emit_list(f, &[
					I::LocalGet(0), length.clone(), I::ArrayNewDefault(array), I::LocalSet(1),
					I::LocalGet(1), I::I32Const(0), I::LocalGet(0), items.clone(), I::I32Const(0), I::LocalGet(0), length.clone(),
					I::ArrayCopy { array_type_index_dst: array, array_type_index_src: array },
					I::LocalGet(0), length.clone(), I::LocalGet(1), I::RefAsNonNull, I::StructNew(list),
				]);
			});
		}
		// filled(count, zero) -> list: count times zero, empty when count is not positive
		if self.should_emit_function(runtime.filled) {
			self.runtime_function(runtime.filled, vec![ValType::I64, value], vec![list_ref], vec![ValType::I32], |s, f| {
				// more elements than an array index holds: out of memory, never a count wrapped to i32
				Self::emit_list(f, &[I::LocalGet(0), I::I64Const(i32::MAX as i64), I::I64GtS]);
				s.emit_fail_if(f, "out_of_memory");
				Self::emit_list(f, &[
					I::LocalGet(0), I::I64Const(0), I::LocalGet(0), I::I64Const(0), I::I64GtS, I::Select, I::I32WrapI64, I::LocalTee(2),
					I::LocalGet(1), I::LocalGet(2), I::ArrayNew(array), I::StructNew(list),
				]);
			});
		}
		// push(list, value) -> list: value appended in place, the items doubled when full; a new list for ø
		if self.should_emit_function(runtime.push) {
			self.runtime_function(runtime.push, vec![list_ref, value], vec![list_ref], vec![array_ref], |_, f| {
				let grown = 2;
				Self::emit_list(f, &[
					I::LocalGet(0), I::RefIsNull, I::If(BlockType::Empty),
					I::I32Const(0), I::I32Const(FIRST_CAPACITY), I::ArrayNewDefault(array), I::StructNew(list), I::LocalSet(0),
					I::End,
					I::LocalGet(0), length.clone(), I::LocalGet(0), items.clone(), I::ArrayLen, I::I32Eq, I::If(BlockType::Empty),
					I::LocalGet(0), length.clone(), I::I32Const(1), I::I32Shl, I::I32Const(FIRST_CAPACITY), I::I32Add, I::ArrayNewDefault(array), I::LocalSet(grown),
					I::LocalGet(grown), I::I32Const(0), I::LocalGet(0), items.clone(), I::I32Const(0), I::LocalGet(0), length.clone(),
					I::ArrayCopy { array_type_index_dst: array, array_type_index_src: array },
					I::LocalGet(0), I::LocalGet(grown), I::RefAsNonNull, I::StructSet { struct_type_index: list, field_index: 1 },
					I::End,
					I::LocalGet(0), items.clone(), I::LocalGet(0), length.clone(), I::LocalGet(1), I::ArraySet(array),
					I::LocalGet(0), I::LocalGet(0), length.clone(), I::I32Const(1), I::I32Add, I::StructSet { struct_type_index: list, field_index: 0 },
					I::LocalGet(0),
				]);
			});
		}
		// sum(list) -> element: left to right like the loop; an int sum gives SUM_NOT_FIXNUM when it leaves the fixnum range
		if self.should_emit_function(runtime.sum) {
			let (position, sum, item) = (1, 2, 3);
			self.runtime_function(runtime.sum, vec![list_ref], vec![value], vec![ValType::I32, value, value], |_, f| {
				let not_fixnum = |local| [I::LocalGet(local), I::I64Const(super::big_int::FIXNUM_OFFSET), I::I64Add, I::I64Const(0), I::I64LtS,
					I::If(BlockType::Empty), I::I64Const(SUM_NOT_FIXNUM), I::Return, I::End];
				Self::emit_list(f, &[
					I::Block(BlockType::Empty), I::Loop(BlockType::Empty),
					I::LocalGet(position), I::LocalGet(0), length.clone(), I::I32GeU, I::BrIf(1),
					I::LocalGet(0), items.clone(), I::LocalGet(position), I::ArrayGet(array), I::LocalSet(item),
				]);
				if element == ElementType::Int {
					Self::emit_list(f, &not_fixnum(item));
					Self::emit_list(f, &[I::LocalGet(sum), I::LocalGet(item), I::I64Add, I::LocalSet(sum)]);
					Self::emit_list(f, &not_fixnum(sum));
				} else {
					Self::emit_list(f, &[I::LocalGet(sum), I::LocalGet(item), I::F64Add, I::LocalSet(sum)]);
				}
				Self::emit_list(f, &[
					I::LocalGet(position), I::I32Const(1), I::I32Add, I::LocalSet(position), I::Br(0),
					I::End, I::End, I::LocalGet(sum),
				]);
			});
		}
		// as_node_list(list) -> $NodeList: one node per element, no cons cells; ø the empty list
		if self.should_emit_function(runtime.as_node_list) {
			let (node_array, node_list) = self.typed_list_types(ElementType::Node);
			let node_list_ref = Ref(self.typed_list_ref(ElementType::Node));
			let node_array_ref = Ref(RefType { nullable: true, heap_type: HeapType::Concrete(node_array) });
			self.runtime_function(runtime.as_node_list, vec![list_ref], vec![node_list_ref], vec![ValType::I32, node_array_ref], |s, f| {
				let (position, nodes) = (1, 2);
				Self::emit_list(f, &[I::LocalGet(0), I::RefIsNull, I::If(BlockType::Empty),
					I::I32Const(0), I::I32Const(0), I::ArrayNewDefault(node_array), I::I64Const(crate::type_kinds::SQUARE_LIST_KIND), I::StructNew(node_list), I::Return, I::End]);
				Self::emit_list(f, &[
					I::LocalGet(0), length.clone(), I::ArrayNewDefault(node_array), I::LocalSet(nodes),
					I::Block(BlockType::Empty), I::Loop(BlockType::Empty),
					I::LocalGet(position), I::LocalGet(0), length.clone(), I::I32GeU, I::BrIf(1),
					I::LocalGet(nodes), I::LocalGet(position), I::LocalGet(0), items.clone(), I::LocalGet(position), I::ArrayGet(array),
				]);
				s.call(f, runtime.new_node);
				Self::emit_list(f, &[
					I::ArraySet(node_array),
					I::LocalGet(position), I::I32Const(1), I::I32Add, I::LocalSet(position), I::Br(0), I::End, I::End,
					I::LocalGet(0), length.clone(), I::LocalGet(nodes), I::RefAsNonNull, I::I64Const(crate::type_kinds::SQUARE_LIST_KIND), I::StructNew(node_list),
				]);
			});
		}
		// as_node(list) -> ref $Node: the square list of element nodes, built from the last element; ø when empty
		if self.should_emit_function(runtime.as_node) {
			let node = self.type_manager.node_type;
			let nullable_node = Ref(self.node_ref(true));
			self.runtime_function(runtime.as_node, vec![list_ref], vec![node_ref], vec![ValType::I32, nullable_node], |s, f| {
				let (position, rest) = (1, 2);
				// a typed list variable read before its first assignment is ø, as any unassigned list variable
				Self::emit_list(f, &[I::LocalGet(0), I::RefIsNull, I::If(BlockType::Empty)]);
				s.call(f, "new_empty");
				Self::emit_list(f, &[I::Return, I::End]);
				Self::emit_list(f, &[
					I::RefNull(HeapType::Concrete(node)), I::LocalSet(rest),
					I::LocalGet(0), length.clone(), I::LocalSet(position),
					I::Block(BlockType::Empty), I::Loop(BlockType::Empty),
					I::LocalGet(position), I::I32Eqz, I::BrIf(1),
					I::LocalGet(position), I::I32Const(1), I::I32Sub, I::LocalSet(position),
					I::LocalGet(0), items.clone(), I::LocalGet(position), I::ArrayGet(array),
				]);
				s.call(f, runtime.new_node);
				Self::emit_list(f, &[I::LocalGet(rest), I::I64Const(SQUARE_BRACKET_INFO)]);
				s.call(f, "new_list");
				Self::emit_list(f, &[I::LocalSet(rest), I::Br(0), I::End, I::End, I::LocalGet(rest), I::RefIsNull, I::If(BlockType::Result(node_ref))]);
				s.call(f, "new_empty");
				Self::emit_list(f, &[I::Else, I::LocalGet(rest), I::RefAsNonNull, I::End]);
			});
		}
	}

	/// Push a number list variable as a $NodeList; false for any other variable
	pub(super) fn emit_typed_list_as_node_list(&mut self, func: &mut Function, name: &str) -> bool {
		let Some((slot, list)) = self.typed_list(&Node::Symbol(name.to_string())) else { return false };
		if list.element == ElementType::Node {
			return false;
		}
		func.instruction(&slot.get());
		self.emit_call(func, list.element.runtime().as_node_list);
		true
	}

	/// In a runtime function (list, index, …): fail unless the 1-based index (local 1) is an integer within the list,
	/// then push its items and the 0-based i32 position
	fn emit_list_position(&self, func: &mut Function, list: u32) {
		self.emit_require_integral(func, 1);
		Self::emit_list(func, &[
			I::LocalGet(1), I::I64Const(1), I::I64LtS,
			I::LocalGet(1), I::LocalGet(0), I::StructGet { struct_type_index: list, field_index: 0 }, I::I64ExtendI32U, I::I64GtS, I::I32Or,
			I::If(BlockType::Empty), I::LocalGet(1), I::LocalGet(0), I::StructGet { struct_type_index: list, field_index: 0 }, I::I64ExtendI32U,
		]);
		self.call(func, super::list_ops::INDEX_OUT_OF_RANGE_OF);
		func.instruction(&I::End);
		Self::emit_list(func, &[
			I::LocalGet(0), I::StructGet { struct_type_index: list, field_index: 1 },
			I::LocalGet(1), I::I32WrapI64, I::I32Const(1), I::I32Sub,
		]);
	}
}
