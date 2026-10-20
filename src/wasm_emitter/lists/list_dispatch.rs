//! One dispatch layer for list operations: each operation the emitter compiles on a list (count, element at an index,
//! index assignment, storing a list in a variable, reading it as a value) asks `list_backend` which implementation runs.
//!
//! Backends are interchangeable, the result is the same Node either way:
//! - `NodeCells`: the generic cons-cell list (data = first, value = rest) of any elements; index walks i links, or on
//!   from the list cursor (list_ops.rs emit_list_cursor_globals), so a loop's `items#(i+1)` and `#items` are O(1)
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

use super::super::maps::map_backend::is_update;
use super::super::WasmGcEmitter;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use crate::type_kinds::Kind;
use std::collections::{HashMap, HashSet};
use wasm_encoder::*;
use Instruction as I;
use ValType::Ref;

mod typed_list_finding;
pub(in crate::wasm_emitter) use typed_list_finding::*;
mod list_runtime;

/// The locals of an as_node(list) runtime function after its parameter: the item index and the cells built so far
const AS_NODE_POSITION: u32 = 1;
const AS_NODE_REST: u32 = 2;

/// The field of a typed list struct holding its array (type_manager emit_growable_list_types: length, items)
const TYPED_LIST_ITEMS: u32 = 1;

/// Capacity of the first items array a push into an empty list allocates; it doubles when full
const FIRST_CAPACITY: i32 = 4;
const NODE_COUNT: &str = "node_count";
const NODE_BYTES: &str = "node_bytes";
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

/// push(list, value) body for a list struct (length, items, …): value appended in place, the items doubled when full;
/// for ø a new list from (0, items) and `new_list_tail`
fn push_in_place(list: u32, array: u32, new_list_tail: &[I<'static>]) -> Vec<I<'static>> {
	let grown = 2;
	let length = || I::StructGet { struct_type_index: list, field_index: 0 };
	let items = || I::StructGet { struct_type_index: list, field_index: 1 };
	#[rustfmt::skip]
	let mut body = vec![
		I::LocalGet(0), I::RefIsNull, I::If(BlockType::Empty),
			I::I32Const(0), I::I32Const(FIRST_CAPACITY), I::ArrayNewDefault(array),
	];
	body.extend_from_slice(new_list_tail);
	body.extend([
		I::LocalSet(0),
		I::End,
		I::LocalGet(0),
		length(),
		I::LocalGet(0),
		items(),
		I::ArrayLen,
		I::I32Eq,
		I::If(BlockType::Empty),
		I::LocalGet(0),
		length(),
		I::I32Const(1),
		I::I32Shl,
		I::I32Const(FIRST_CAPACITY),
		I::I32Add,
		I::ArrayNewDefault(array),
		I::LocalSet(grown),
		I::LocalGet(grown),
		I::I32Const(0),
		I::LocalGet(0),
		items(),
		I::I32Const(0),
		I::LocalGet(0),
		length(),
		I::ArrayCopy { array_type_index_dst: array, array_type_index_src: array },
		I::LocalGet(0),
		I::LocalGet(grown),
		I::RefAsNonNull,
		I::StructSet { struct_type_index: list, field_index: 1 },
		I::End,
		I::LocalGet(0),
		items(),
		I::LocalGet(0),
		length(),
		I::LocalGet(1),
		I::ArraySet(array),
		I::LocalGet(0),
		I::LocalGet(0),
		length(),
		I::I32Const(1),
		I::I32Add,
		I::StructSet { struct_type_index: list, field_index: 0 },
		I::LocalGet(0),
	]);
	body
}

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
	/// concat(a, b) -> list: a's items then b's, copied in bulk (array.copy); ø counts as empty
	concat: &'static str,
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
	at: "int_list_at",
	set: "int_list_set",
	copy: "int_list_copy",
	concat: "int_list_concat",
	as_node: "int_list_as_node",
	as_node_list: "int_list_as_node_list",
	filled: "int_list_filled",
	push: "int_list_push",
	sum: "int_list_sum",
	new_node: "new_int",
};
const FLOAT_RUNTIME: ElementRuntime = ElementRuntime {
	at: "float_list_at",
	set: "float_list_set",
	copy: "float_list_copy",
	concat: "float_list_concat",
	as_node: "float_list_as_node",
	as_node_list: "float_list_as_node_list",
	filled: "float_list_filled",
	push: "float_list_push",
	sum: "float_list_sum",
	new_node: "new_float",
};
const NODE_RUNTIME: ElementRuntime = ElementRuntime {
	at: "node_list_at",
	set: "node_list_set",
	copy: "node_list_copy",
	concat: "",
	as_node: "node_list_as_node",
	as_node_list: "",
	filled: "node_list_filled",
	push: "node_list_push",
	sum: "",
	new_node: "",
};
/// list_items(node) -> $NodeList: the items of a Node list in a fresh array (emit_list_items), what a Node list variable
/// starts from
pub(in crate::wasm_emitter) const LIST_ITEMS: &str = "list_items";
/// list_item_count(node) -> i64: how many items list_items finds, without making their array
pub(in crate::wasm_emitter) const LIST_ITEM_COUNT: &str = "list_item_count";

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

/// An append to a list variable, an assignment of one list variable to another or of a fresh list to it, numbered in
/// the order it runs, with the loops around it
struct ListEvent {
	kind: ListEventKind,
	order: usize,
	loops: Vec<usize>,
	/// under an `if`, `and`, `or` or `?`: it may not run
	conditional: bool,
}

#[derive(PartialEq)]
enum ListEventKind {
	Append,
	Alias,
	/// `out = ø`, `out = [1 2]`: the variable holds a list no one else does
	Fresh,
}

/// The walk list_events makes: the loops around the node, the events numbered so far, the conditions it is under
#[derive(Default)]
struct EventWalk {
	loops: Vec<usize>,
	count: usize,
	conditions: usize,
	events: Vec<ListEvent>,
}

const CONDITIONAL_OPS: [Op; 6] = [Op::If, Op::Then, Op::Else, Op::And, Op::Or, Op::Question];

/// Where a typed list variable lives: a local of the body, or a global main builds and functions read
#[derive(Clone, Copy, Debug)]
pub(in crate::wasm_emitter) enum Slot {
	Local(u32),
	Global(u32),
}

impl Slot {
	pub(in crate::wasm_emitter) fn get(self) -> Instruction<'static> {
		match self {
			Slot::Local(index) => I::LocalGet(index),
			Slot::Global(index) => I::GlobalGet(index),
		}
	}

	/// Store the list on the stack and leave it there
	pub(in crate::wasm_emitter) fn tee(self, func: &mut Function) {
		match self {
			Slot::Local(index) => {
				func.instruction(&I::LocalTee(index));
			}
			Slot::Global(index) => {
				WasmGcEmitter::emit_list(func, &[I::GlobalSet(index), I::GlobalGet(index)]);
			}
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
	Converted {
		fresh: bool,
	},
	/// `xs + ys + [4]`: a new list of list variables and literals, an array of their element type when they agree
	Concat(Vec<Source>),
	Other,
}

impl WasmGcEmitter {
	/// The slot and element type of `target` when it is a list variable held as a typed array: a local of the body, else
	/// a typed global
	pub(in crate::wasm_emitter) fn typed_list(&self, target: &Node) -> Option<(Slot, TypedList)> {
		let Node::Symbol(name) = target.drop_meta() else {
			return None;
		};
		match self.scope.lookup(name) {
			Some(local) => Some((Slot::Local(local.position), *self.typed_lists.get(name)?)),
			None => self.typed_globals.get(name).map(|&(global, list)| (Slot::Global(global), list)),
		}
	}

	/// The backend that runs `op` on `target`. A host backend would be picked here for its element types, with the
	/// length test against HOST_BACKEND_MIN_LENGTH emitted around the call.
	pub(in crate::wasm_emitter) fn list_backend(&self, _op: ListOp, target: &Node) -> Backend {
		match self.typed_list(target) {
			Some((_, list)) => Backend::TypedArray(list.element),
			None => Backend::NodeCells,
		}
	}

	/// The (array, list) type indices of an element type
	pub(in crate::wasm_emitter) fn typed_list_types(&self, element: ElementType) -> (u32, u32) {
		let types = &self.type_manager;
		match element {
			ElementType::Int => (types.int_array_type, types.int_list_type),
			ElementType::Float => (types.float_array_type, types.float_list_type),
			ElementType::Node => (types.node_array_type, types.node_list_type),
		}
	}

	pub(in crate::wasm_emitter) fn typed_list_ref(&self, element: ElementType) -> RefType {
		RefType { nullable: true, heap_type: HeapType::Concrete(self.typed_list_types(element).1) }
	}

	/// How the local of a variable is stored: a typed list as its list struct, anything else by its kind
	pub(in crate::wasm_emitter) fn local_storage_type(&self, name: &str, kind: Kind) -> ValType {
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

	/// `t.bytes` without a call when t holds a text, its length field; anything else through node_bytes (card
	/// loop-calls: a byte loop asked node_bytes each round)
	fn emit_byte_count_inline(&mut self, func: &mut Function, target: &Node) {
		let held = self.node_scratch();
		self.emit_node_instructions(func, target);
		func.instruction(&I::LocalTee(held));
		func.instruction(&I::StructGet { struct_type_index: self.type_manager.node_type, field_index: 0 });
		Self::emit_kind_test(func, Kind::Text as i64, I::I64Eq);
		func.instruction(&I::If(BlockType::Result(ValType::I64)));
		self.emit_text_field(func, held, 1);
		Self::emit_list(func, &[I::I64ExtendI32U, I::Else, I::LocalGet(held), I::RefAsNonNull]);
		self.emit_call(func, NODE_BYTES);
		func.instruction(&I::End);
	}

	/// Push the element count (i64) of `target`; `counter` is the runtime counter of the generic backend (node_count, node_bytes …)
	pub(in crate::wasm_emitter) fn emit_list_count(
		&mut self,
		func: &mut Function,
		target: &Node,
		counter: &'static str,
	) {
		// `(statements; xs)` (a lowered filter): the statements run, a typed list xs is counted without making its nodes
		if let Node::List(items, Bracket::Round, crate::node::Separator::Semicolon | crate::node::Separator::Newline) =
			target.drop_meta()
		{
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
				Self::emit_list(
					func,
					&[slot.get(), I::StructGet { struct_type_index: list_type, field_index: 0 }, I::I64ExtendI32U],
				);
			}
			(_, NODE_BYTES) => self.emit_byte_count_inline(func, target),
			_ => {
				self.emit_node_instructions(func, target);
				self.emit_call(func, counter);
			}
		}
	}

	/// Push the number (i64) at the 1-based `index` of `target`; a float element takes the generic way, which fails as
	/// it always did
	pub(in crate::wasm_emitter) fn emit_list_element_number(
		&mut self,
		func: &mut Function,
		target: &Node,
		index: &Node,
	) {
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
	pub(in crate::wasm_emitter) fn holds_only_numbers(&self, target: &Node) -> bool {
		self.typed_list(target).is_some_and(|(_, list)| list.element != ElementType::Node)
	}

	pub(in crate::wasm_emitter) fn is_typed_list(&self, target: &Node) -> bool {
		self.typed_list(target).is_some()
	}

	/// Push the f64 at the 1-based `index` of a typed list `target`
	pub(in crate::wasm_emitter) fn emit_typed_element_float(
		&mut self,
		func: &mut Function,
		target: &Node,
		index: &Node,
	) {
		let (_, list) = self.typed_list(target).expect("typed backend");
		self.emit_integral_index_check(func, index);
		self.emit_typed_element(func, target, index);
		match list.element {
			ElementType::Int => self.emit_int_to_f64(func, None),
			ElementType::Node => self.emit_call(func, super::super::lists::list_ops::TEXT_AS_FLOAT),
			ElementType::Float => {}
		}
	}

	/// Push the Node at the 1-based `index` of `target` (a list element, or a character of a text)
	pub(in crate::wasm_emitter) fn emit_list_element_node(&mut self, func: &mut Function, target: &Node, index: &Node) {
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
			#[rustfmt::skip]
			Self::emit_list(func, &[
				I::StructGet { struct_type_index: list_type, field_index: TYPED_LIST_ITEMS }, I::LocalGet(counter),
				I::I32WrapI64, I::ArrayGet(array),
			]);
			return;
		}
		self.emit_numeric_value(func, index);
		self.emit_call(func, list.element.runtime().at);
	}

	/// The local of the counter when `index` is `x·index + 1` and `target` is `x·items`, the list a for loop walks
	/// (for_loop.rs): the loop runs while the counter is below the count, so the element exists
	fn walking_counter(&self, target: &Node, index: &Node) -> Option<u32> {
		let Node::Key(counter, Op::Add, one) = index.drop_meta() else {
			return None;
		};
		let (Node::Symbol(counter), Node::Symbol(list)) = (counter.drop_meta(), target.drop_meta()) else {
			return None;
		};
		let walked = counter.strip_suffix(crate::for_loop::INDEX_SUFFIX)?;
		let is_walk = matches!(one.drop_meta(), Node::Number(crate::extensions::numbers::Number::Int(1)))
			&& *list == format!("{walked}{}", crate::for_loop::ITEMS_SUFFIX);
		(is_walk && self.bounded_counters.contains(counter) && !self.typed_lists[list].updated)
			.then(|| self.scope.lookup(counter).map(|local| local.position))
			.flatten()
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
	pub(in crate::wasm_emitter) fn emit_typed_element_assignment(
		&mut self,
		func: &mut Function,
		target: &Node,
		index: &Node,
		value: &Node,
	) -> bool {
		let Backend::TypedArray(element) = self.list_backend(ListOp::SetElement, target) else {
			return false;
		};
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
	pub(in crate::wasm_emitter) fn emit_typed_list_store(
		&mut self,
		func: &mut Function,
		name: &str,
		value: &Node,
	) -> bool {
		if let Some(slot) = self.typed_map(&Node::Symbol(name.to_string())) {
			self.emit_typed_map_store(func, name, slot, value);
			return true;
		}
		if let Some((slot, _)) = self.typed_struct(&Node::Symbol(name.to_string())) {
			let class = self.typed_structs[name].clone();
			self.emit_typed_struct_store(func, name, slot, &class, value);
			return true;
		}
		let Some((slot, list)) = self.typed_list(&Node::Symbol(name.to_string())) else {
			return false;
		};
		// `(statements; last)`: the statements run, the last value is stored
		if let Node::List(items, Bracket::Round, crate::node::Separator::Semicolon | crate::node::Separator::Newline) =
			value.drop_meta()
		{
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
				let Node::List(items, _, _) = appended.drop_meta() else {
					unreachable!("find_typed_lists admits only appended lists")
				};
				func.instruction(&slot.get());
				if list.aliased {
					self.emit_call(func, runtime.copy);
				}
				self.emit_typed_pushes(func, items, list.element);
			}
			// `xs + ys + [4]` of arrays of this element type: copied in bulk, no Node made of an item
			_ if list.element != ElementType::Node && self.concatenated_arrays(value, list.element).is_some() => {
				for (index, part) in
					self.concatenated_arrays(value, list.element).expect("guarded").into_iter().enumerate()
				{
					match part.drop_meta() {
						Node::List(items, _, _) => self.emit_typed_list_of(func, items, list.element),
						_ => {
							func.instruction(&self.typed_list(part).expect("a typed list part").0.get());
						}
					}
					if index > 0 {
						self.emit_call(func, runtime.concat);
					}
				}
			}
			Node::Empty => self.emit_typed_list_of(func, &[], list.element),
			Node::List(items, Bracket::Square, _)
				if self.number_source_of(name, value).is_literal() || items.is_empty() =>
			{
				self.emit_typed_list_of(func, items, list.element)
			}
			// a call of a function giving back its array: that array, no conversion (list_abi.rs)
			_ if list.element == ElementType::Node
				&& super::super::lists::list_abi::called_function(value)
					.is_some_and(|callee| self.returns_list_abi(callee)) =>
			{
				self.emit_list_abi_value(func, value);
			}
			// a Node list (a call's result, a list parameter): its items once into the array
			_ if list.element == ElementType::Node => {
				self.emit_node_instructions(func, value);
				self.emit_call(func, LIST_ITEMS);
			}
			other => unreachable!("find_typed_lists admits no {other:?}"),
		}
		slot.tee(func);
		true
	}

	/// `xs += [a b]` of a typed list: the items pushed onto its array in place, leaving the list. False for any other
	pub(in crate::wasm_emitter) fn emit_typed_list_extend(
		&mut self,
		func: &mut Function,
		name: &str,
		items: &[Node],
	) -> bool {
		let Some((slot, list)) = self.typed_list(&Node::Symbol(name.to_string())) else {
			return false;
		};
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

	/// The parts of `xs + ys + [4]` when each is an array of `element` (a typed list variable or a literal of such numbers)
	fn concatenated_arrays<'n>(&self, value: &'n Node, element: ElementType) -> Option<Vec<&'n Node>> {
		let parts = self.concatenated_lists(value)?;
		let is_array = |part: &Node| match part.drop_meta() {
			Node::List(items, _, _) => self.items_element(items) == Some(element),
			_ => self.typed_list(part).is_some_and(|(_, list)| list.element == element),
		};
		parts.iter().all(|part| is_array(part)).then_some(parts)
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
	pub(in crate::wasm_emitter) fn typed_list_store(&self, statement: &Node) -> Option<(String, Node)> {
		let Node::Key(target, Op::Assign | Op::Define, value) = statement.drop_meta() else {
			return None;
		};
		let Node::Symbol(name) = target.drop_meta() else {
			return None;
		};
		(self.typed_list(target).is_some() || self.typed_maps.contains(name) || self.typed_structs.contains_key(name))
			.then(|| (name.clone(), value.as_ref().clone()))
	}

	/// The variable and items of `xs += [a b]` on a typed list variable
	pub(in crate::wasm_emitter) fn typed_list_extension(&self, statement: &Node) -> Option<(String, Vec<Node>)> {
		let Node::Key(target, Op::AddAssign, value) = statement.drop_meta() else {
			return None;
		};
		let Node::Symbol(name) = target.drop_meta() else {
			return None;
		};
		self.typed_list(target).and(extended_items(value)).map(|items| (name.clone(), items.to_vec()))
	}

	/// Push a typed list variable as the Node list it stands for; false for any other variable
	pub(in crate::wasm_emitter) fn emit_typed_list_as_node(&mut self, func: &mut Function, name: &str) -> bool {
		if let Some(slot) = self.typed_map(&Node::Symbol(name.to_string())) {
			self.emit_typed_map_as_node(func, slot);
			return true;
		}
		if self.typed_structs.contains_key(name) {
			self.emit_typed_struct_as_node(func, name);
			return true;
		}
		let Some((slot, list)) = self.typed_list(&Node::Symbol(name.to_string())) else {
			return false;
		};
		func.instruction(&slot.get());
		self.emit_call(func, list.element.runtime().as_node);
		true
	}

	/// The $IntList itself of a typed int list variable, as a host word reads it (list_to_ints); false for any other value
	pub(in crate::wasm_emitter) fn emit_typed_int_list(&mut self, func: &mut Function, value: &Node) -> bool {
		let Some((slot, _)) = self.typed_list(value).filter(|(_, list)| list.element == ElementType::Int) else {
			return false;
		};
		func.instruction(&slot.get());
		true
	}

	/// `list_sum(list, loop)` as `wanted`: a typed list sums its items in one runtime call (an int sum that leaves the
	/// fixnum range runs the exact loop instead), any other list runs the loop
	pub(in crate::wasm_emitter) fn emit_list_sum(
		&mut self,
		func: &mut Function,
		list: &Node,
		sum_loop: &Node,
		wanted: Wanted,
	) {
		match (self.list_backend(ListOp::Sum, list), wanted) {
			(Backend::TypedArray(ElementType::Int), _) => {
				let (slot, _) = self.typed_list(list).expect("typed backend");
				let sum = self.scratch(0);
				func.instruction(&slot.get());
				self.emit_call(func, INT_RUNTIME.sum);
				Self::emit_list(
					func,
					&[I::LocalTee(sum), I::I64Const(SUM_NOT_FIXNUM), I::I64Eq, I::If(BlockType::Result(ValType::I64))],
				);
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
}

/// `xs: floats`, `xs: [float]`: an assignment's target declared a list of floats (the type its Meta)
fn declares_float_items(target: &Node) -> bool {
	matches!(target, Node::Meta { data, .. } if crate::analyzer::declared_element_type(data).is_some_and(crate::type_tests::is_float_type_word))
}
