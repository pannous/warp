//! One dispatch layer for list operations: each operation the emitter compiles on a list (count, element at an index,
//! index assignment, storing a list in a variable, reading it as a value) asks `list_backend` which implementation runs.
//!
//! Backends are interchangeable, the result is the same Node either way:
//! - `NodeCells`: the generic cons-cell list (data = first, value = rest) of any elements; index walks i links
//! - `TypedArray`: a list variable proven to hold only ints is an `$IntList`, a length and a wasm GC
//!   `(array (mut i64))` with spare capacity, see `find_typed_lists`: O(1) index and count, amortised O(1) append
//!   (`out.add(x)`, which `map` lowers to), no box per element. Wherever the program needs it as a value (a result, an
//!   argument, a print) it becomes the same Node list the literal would have built (`int_list_as_node`).
//! - `Host`: reserved for a host-native or GPU implementation behind a host import, taking over above
//!   `HOST_BACKEND_MIN_LENGTH` at run time; none exists yet (no wasm SIMD, user decision). notes/typed_lists.md
//!
//! A typed list keeps value semantics: `ys = xs` copies it when either variable is updated (by index or by an append).

use super::WasmGcEmitter;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use crate::type_kinds::Kind;
use std::collections::{HashMap, HashSet};
use wasm_encoder::*;
use Instruction as I;
use ValType::Ref;

/// Element count from which a host/GPU backend would take over a list operation at run time; `None`: no such backend
pub const HOST_BACKEND_MIN_LENGTH: Option<u32> = None;

pub const INT_LIST_AT: &str = "int_list_at";
pub const INT_LIST_SET: &str = "int_list_set";
pub const INT_LIST_COPY: &str = "int_list_copy";
pub const INT_LIST_AS_NODE: &str = "int_list_as_node";
pub const INT_LIST_FILLED: &str = "int_list_filled";
pub const INT_LIST_PUSH: &str = "int_list_push";
/// Capacity of the first items array a push into an empty list allocates; it doubles when full
const FIRST_CAPACITY: i32 = 4;
const NODE_COUNT: &str = "node_count";
const SQUARE_BRACKET_INFO: i64 = 1;

/// The list operations that go through the dispatch layer
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ListOp {
	/// `#xs`, `count xs`, `xs.size`
	Count,
	/// `xs#i` as a number or as a Node
	Element,
	/// `xs#i = v`, `xs#i += v`
	SetElement,
	/// `xs = [1 2 3]`, `ys = xs`
	Store,
	/// `xs` where a Node is needed
	AsNode,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ElementType {
	Int,
}

/// An implementation of the list operations
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Backend {
	NodeCells,
	TypedArray(ElementType),
	/// host-native or GPU, behind a host import (not implemented)
	Host,
}

/// A list variable held as a typed array
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TypedList {
	pub element: ElementType,
	/// assigned by index somewhere, so an alias of it needs its own copy
	pub updated: bool,
}

/// Where the value a list variable is assigned comes from
enum Source {
	/// `[1 2 3]`, `int[100]`
	Literal(ElementType),
	/// `[]`, `ø`: fits a list of any element type
	Empty,
	/// `xs = xs + [4 5]`, what `xs.add(4)` lowers to
	Append(ElementType),
	Variable(String),
	Other,
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

fn is_update(op: &Op) -> bool {
	op.is_compound_assign() || matches!(op, Op::Inc | Op::Dec)
}

/// The element type every source agrees on: `Err` when a source is no typed list or they disagree, `Ok(None)` when only
/// variables of yet unknown type feed it
fn agreed_element(sources: &[Source], typed: &HashMap<String, Option<ElementType>>) -> Result<Option<ElementType>, ()> {
	let mut agreed = None;
	for source in sources {
		let element = match source {
			Source::Literal(element) | Source::Append(element) => Some(*element),
			Source::Empty => None,
			Source::Variable(name) => *typed.get(name).ok_or(())?,
			Source::Other => return Err(()),
		};
		match (agreed, element) {
			(Some(a), Some(b)) if a != b => return Err(()),
			(None, element) => agreed = element,
			_ => {}
		}
	}
	Ok(agreed)
}

impl WasmGcEmitter {
	/// An element provably an exact Int: a number literal, an Int variable, a call of a function returning one, or
	/// arithmetic of those. Not by the type alone: it defaults to Int for anything unknown (ø, a key)
	fn is_int_item(&self, item: &Node) -> bool {
		let is_int_kind = self.get_type(item) == Kind::Int;
		is_int_kind && match item.drop_meta() {
			Node::Number(_) | Node::Symbol(_) => true,
			Node::Key(left, op, right) if op.is_arithmetic() => {
				(matches!(left.drop_meta(), Node::Empty) || self.is_int_item(left)) && self.is_int_item(right)
			}
			Node::List(items, _, _) => matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if self.ctx.user_functions.contains_key(name)),
			_ => false,
		}
	}

	/// The source of `target = value`
	fn source_of(&self, target: &str, value: &Node) -> Source {
		let is_int = |item: &Node| self.is_int_item(item);
		match value.drop_meta() {
			Node::Empty => Source::Empty,
			Node::List(items, Bracket::Square, _) if items.is_empty() => Source::Empty,
			Node::Key(list, Op::Add, appended) if matches!(list.drop_meta(), Node::Symbol(name) if name == target) => match appended.drop_meta() {
				Node::List(items, Bracket::Square, _) if !items.is_empty() && items.iter().all(is_int) => Source::Append(ElementType::Int),
				_ => Source::Other,
			},
			Node::Symbol(name) => Source::Variable(name.clone()),
			_ if zero_fill_parts(value).is_some_and(|(_, zero)| is_int(zero)) => Source::Literal(ElementType::Int),
			Node::List(items, Bracket::Square, _) if !items.is_empty() && items.iter().all(is_int) => Source::Literal(ElementType::Int),
			_ => Source::Other,
		}
	}

	/// The list variables of the current body that can be typed arrays: locals of kind List that are only ever assigned
	/// int literal lists, the empty list, ints appended to themselves or other such variables, never updated by an
	/// operator (`xs += …`), not captured by a function, no global, not assigned inside a `try`. Any other use reads them
	/// as Nodes, so it stays correct.
	pub(super) fn find_typed_lists(&self, program: &Node) -> HashMap<String, TypedList> {
		let mut sources: HashMap<String, Vec<Source>> = HashMap::new();
		let mut excluded: HashSet<String> = HashSet::new();
		let mut updated: HashSet<String> = HashSet::new();
		program.visit(&mut |part| {
			let Node::Key(target, op, value) = part else { return };
			match target.drop_meta() {
				Node::Symbol(name) if matches!(op, Op::Assign | Op::Define) => sources.entry(name.clone()).or_default().push(self.source_of(name, value)),
				Node::Symbol(name) if is_update(op) => { excluded.insert(name.clone()); }
				Node::Key(list, Op::Hash, index) if matches!(op, Op::Assign | Op::Define) || is_update(op) => {
					if let Node::Symbol(name) = list.drop_meta() {
						let by_key = self.map_key(index).is_some() || self.dynamic_key(index).is_some();
						if by_key { excluded.insert(name.clone()) } else { updated.insert(name.clone()) };
					}
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
		let is_list_local = |name: &String| self.scope.lookup(name).is_some_and(|local| !local.is_param && local.kind == Kind::List);
		let has_start = |name: &String| sources[name].iter().any(|source| !matches!(source, Source::Append(_)));
		let mut typed: HashMap<String, Option<ElementType>> = sources.keys()
			.filter(|name| is_list_local(name) && has_start(name) && !excluded.contains(*name))
			.map(|name| (name.clone(), None))
			.collect();
		// greatest fixpoint: drop a variable fed by something untyped, until all that remain agree
		loop {
			let mut changed = false;
			for name in typed.keys().cloned().collect::<Vec<_>>() {
				match agreed_element(&sources[&name], &typed) {
					Err(()) => { typed.remove(&name); changed = true; }
					Ok(Some(element)) if typed[&name].is_none() => { typed.insert(name, Some(element)); changed = true; }
					Ok(_) => {}
				}
			}
			if !changed {
				break;
			}
		}
		typed.into_iter()
			.filter_map(|(name, element)| Some((name.clone(), TypedList { element: element?, updated: updated.contains(&name) })))
			.collect()
	}

	/// The local slot and element type of `target` when it is a list variable held as a typed array
	fn typed_list(&self, target: &Node) -> Option<(u32, TypedList)> {
		let Node::Symbol(name) = target.drop_meta() else { return None };
		let list = *self.typed_lists.get(name)?;
		Some((self.scope.lookup(name)?.position, list))
	}

	/// The backend that runs `op` on `target`. A host backend would be picked here for its element types, with the
	/// length test against HOST_BACKEND_MIN_LENGTH emitted around the call.
	pub(super) fn list_backend(&self, _op: ListOp, target: &Node) -> Backend {
		match self.typed_list(target) {
			Some((_, list)) => Backend::TypedArray(list.element),
			None => Backend::NodeCells,
		}
	}

	/// How the local of a variable is stored: a typed list as its array, anything else by its kind
	pub(super) fn local_storage_type(&self, name: &str, kind: Kind) -> ValType {
		match self.typed_lists.get(name).map(|list| list.element) {
			Some(ElementType::Int) => Ref(self.int_list_ref()),
			None => self.storage_type(kind),
		}
	}

	/// Push the element count (i64) of `target`; `counter` is the runtime counter of the generic backend (node_count, node_bytes …)
	pub(super) fn emit_list_count(&mut self, func: &mut Function, target: &Node, counter: &'static str) {
		match (self.typed_list(target), counter) {
			(Some((slot, _)), NODE_COUNT) => {
				let int_list = self.type_manager.int_list_type;
				Self::emit_list(func, &[I::LocalGet(slot), I::StructGet { struct_type_index: int_list, field_index: 0 }, I::I64ExtendI32U]);
			}
			_ => {
				self.emit_node_instructions(func, target);
				self.emit_call(func, counter);
			}
		}
	}

	/// Push the number (i64) at the 1-based `index` of `target`
	pub(super) fn emit_list_element_number(&mut self, func: &mut Function, target: &Node, index: &Node) {
		self.emit_integral_index_check(func, index);
		match self.list_backend(ListOp::Element, target) {
			Backend::TypedArray(ElementType::Int) => self.emit_typed_element(func, target, index),
			_ => {
				self.emit_node_instructions(func, target);
				self.emit_numeric_value(func, index);
				self.emit_call(func, "list_at");
			}
		}
	}

	/// Push the Node at the 1-based `index` of `target` (a list element, or a character of a text)
	pub(super) fn emit_list_element_node(&mut self, func: &mut Function, target: &Node, index: &Node) {
		self.emit_integral_index_check(func, index);
		match self.list_backend(ListOp::Element, target) {
			Backend::TypedArray(ElementType::Int) => {
				self.emit_typed_element(func, target, index);
				self.emit_call(func, "new_int");
			}
			_ => {
				self.emit_node_instructions(func, target);
				self.emit_numeric_value(func, index);
				self.emit_call(func, "node_index_at");
			}
		}
	}

	fn emit_typed_element(&mut self, func: &mut Function, target: &Node, index: &Node) {
		let (slot, _) = self.typed_list(target).expect("typed backend");
		func.instruction(&I::LocalGet(slot));
		self.emit_numeric_value(func, index);
		self.emit_call(func, INT_LIST_AT);
	}

	/// `target#index = value` on a typed list, in place: leaves the assigned value (i64). False for any other list.
	pub(super) fn emit_typed_element_assignment(&mut self, func: &mut Function, target: &Node, index: &Node, value: &Node) -> bool {
		if self.list_backend(ListOp::SetElement, target) != Backend::TypedArray(ElementType::Int) {
			return false;
		}
		let (slot, _) = self.typed_list(target).expect("typed backend");
		let assigned = self.scratch(2);
		func.instruction(&I::LocalGet(slot));
		self.emit_numeric_value(func, index);
		self.emit_numeric_value(func, value);
		func.instruction(&I::LocalTee(assigned));
		self.emit_call(func, INT_LIST_SET);
		func.instruction(&I::LocalGet(assigned));
		true
	}

	/// `name = value` for a typed list variable: stores the array and leaves it on the stack. False for any other variable.
	pub(super) fn emit_typed_list_store(&mut self, func: &mut Function, name: &str, value: &Node) -> bool {
		let Some((slot, list)) = self.typed_list(&Node::Symbol(name.to_string())) else { return false };
		match value.drop_meta() {
			Node::Symbol(source) => {
				let (source_slot, source_list) = self.typed_list(value).unwrap_or_else(|| panic!("{source} is a typed list"));
				func.instruction(&I::LocalGet(source_slot));
				if list.updated || source_list.updated {
					self.emit_call(func, INT_LIST_COPY);
				}
			}
			_ if zero_fill_parts(value).is_some() => {
				let (count, zero) = zero_fill_parts(value).expect("guarded");
				self.emit_numeric_value(func, count);
				self.emit_numeric_value(func, zero);
				self.emit_call(func, INT_LIST_FILLED);
			}
			Node::Key(_, Op::Add, appended) => {
				let Node::List(items, _, _) = appended.drop_meta() else { unreachable!("find_typed_lists admits only appended lists") };
				func.instruction(&I::LocalGet(slot));
				for item in items {
					self.emit_numeric_value(func, item);
					self.emit_call(func, INT_LIST_PUSH);
				}
			}
			Node::Empty => self.emit_int_list_of(func, &[]),
			Node::List(items, _, _) => self.emit_int_list_of(func, items),
			other => unreachable!("find_typed_lists admits no {other:?}"),
		}
		func.instruction(&I::LocalTee(slot));
		true
	}

	/// A new $IntList of the numbers of `items`
	fn emit_int_list_of(&mut self, func: &mut Function, items: &[Node]) {
		func.instruction(&I::I32Const(items.len() as i32));
		for item in items {
			self.emit_numeric_value(func, item);
		}
		func.instruction(&I::ArrayNewFixed { array_type_index: self.type_manager.int_array_type, array_size: items.len() as u32 });
		func.instruction(&I::StructNew(self.type_manager.int_list_type));
	}

	fn int_list_ref(&self) -> RefType {
		RefType { nullable: true, heap_type: HeapType::Concrete(self.type_manager.int_list_type) }
	}

	/// The variable and value of an assignment to a typed list variable
	pub(super) fn typed_list_store(&self, statement: &Node) -> Option<(String, Node)> {
		let Node::Key(target, Op::Assign | Op::Define, value) = statement.drop_meta() else { return None };
		let Node::Symbol(name) = target.drop_meta() else { return None };
		self.typed_lists.contains_key(name).then(|| (name.clone(), value.as_ref().clone()))
	}

	/// Push a typed list variable as the Node list it stands for; false for any other variable
	pub(super) fn emit_typed_list_as_node(&mut self, func: &mut Function, name: &str) -> bool {
		let Some((slot, _)) = self.typed_list(&Node::Symbol(name.to_string())) else { return false };
		func.instruction(&I::LocalGet(slot));
		self.emit_call(func, INT_LIST_AS_NODE);
		true
	}

	/// The runtime functions of the typed-array backend, each only when the program calls it
	pub(super) fn emit_typed_list_runtime(&mut self) {
		let (list, array) = (self.type_manager.int_list_type, self.type_manager.int_array_type);
		let list_ref = Ref(self.int_list_ref());
		let array_ref = Ref(RefType { nullable: true, heap_type: HeapType::Concrete(array) });
		let node_ref = Ref(self.node_ref(false));
		let length = I::StructGet { struct_type_index: list, field_index: 0 };
		let items = I::StructGet { struct_type_index: list, field_index: 1 };
		// int_list_at(list, index) -> i64: the element at the 1-based index
		if self.should_emit_function(INT_LIST_AT) {
			self.runtime_function(INT_LIST_AT, vec![list_ref, ValType::I64], vec![ValType::I64], vec![], |s, f| {
				s.emit_list_position(f);
				f.instruction(&I::ArrayGet(array));
			});
		}
		// int_list_set(list, index, value): the element at the 1-based index becomes value
		if self.should_emit_function(INT_LIST_SET) {
			self.runtime_function(INT_LIST_SET, vec![list_ref, ValType::I64, ValType::I64], vec![], vec![], |s, f| {
				s.emit_list_position(f);
				Self::emit_list(f, &[I::LocalGet(2), I::ArraySet(array)]);
			});
		}
		// int_list_copy(list) -> list: its own items, no spare capacity
		if self.should_emit_function(INT_LIST_COPY) {
			self.runtime_function(INT_LIST_COPY, vec![list_ref], vec![list_ref], vec![array_ref], |_, f| {
				Self::emit_list(f, &[
					I::LocalGet(0), length.clone(), I::ArrayNewDefault(array), I::LocalSet(1),
					I::LocalGet(1), I::I32Const(0), I::LocalGet(0), items.clone(), I::I32Const(0), I::LocalGet(0), length.clone(),
					I::ArrayCopy { array_type_index_dst: array, array_type_index_src: array },
					I::LocalGet(0), length.clone(), I::LocalGet(1), I::RefAsNonNull, I::StructNew(list),
				]);
			});
		}
		// int_list_filled(count, zero) -> list: count times zero, empty when count is not positive
		if self.should_emit_function(INT_LIST_FILLED) {
			self.runtime_function(INT_LIST_FILLED, vec![ValType::I64, ValType::I64], vec![list_ref], vec![ValType::I32], |_, f| {
				Self::emit_list(f, &[
					I::LocalGet(0), I::I64Const(0), I::LocalGet(0), I::I64Const(0), I::I64GtS, I::Select, I::I32WrapI64, I::LocalTee(2),
					I::LocalGet(1), I::LocalGet(2), I::ArrayNew(array), I::StructNew(list),
				]);
			});
		}
		// int_list_push(list, value) -> list: value appended in place, the items doubled when full; a new list for ø
		if self.should_emit_function(INT_LIST_PUSH) {
			self.runtime_function(INT_LIST_PUSH, vec![list_ref, ValType::I64], vec![list_ref], vec![array_ref], |_, f| {
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
		// int_list_as_node(list) -> ref $Node: the square list of Int nodes, built from the last element; ø when empty
		if self.should_emit_function(INT_LIST_AS_NODE) {
			let node = self.type_manager.node_type;
			let nullable_node = Ref(self.node_ref(true));
			self.runtime_function(INT_LIST_AS_NODE, vec![list_ref], vec![node_ref], vec![ValType::I32, nullable_node], |s, f| {
				let (position, rest) = (1, 2);
				Self::emit_list(f, &[
					I::RefNull(HeapType::Concrete(node)), I::LocalSet(rest),
					I::LocalGet(0), length.clone(), I::LocalSet(position),
					I::Block(BlockType::Empty), I::Loop(BlockType::Empty),
					I::LocalGet(position), I::I32Eqz, I::BrIf(1),
					I::LocalGet(position), I::I32Const(1), I::I32Sub, I::LocalSet(position),
					I::LocalGet(0), items.clone(), I::LocalGet(position), I::ArrayGet(array),
				]);
				s.call(f, "new_int");
				Self::emit_list(f, &[I::LocalGet(rest), I::I64Const(SQUARE_BRACKET_INFO)]);
				s.call(f, "new_list");
				Self::emit_list(f, &[I::LocalSet(rest), I::Br(0), I::End, I::End, I::LocalGet(rest), I::RefIsNull, I::If(BlockType::Result(node_ref))]);
				s.call(f, "new_empty");
				Self::emit_list(f, &[I::Else, I::LocalGet(rest), I::RefAsNonNull, I::End]);
			});
		}
	}

	/// In a runtime function (list, index, …): fail unless the 1-based index (local 1) is an integer within the list,
	/// then push its items and the 0-based i32 position
	fn emit_list_position(&self, func: &mut Function) {
		let list = self.type_manager.int_list_type;
		self.emit_require_integral(func, 1);
		Self::emit_list(func, &[
			I::LocalGet(1), I::I64Const(1), I::I64LtS,
			I::LocalGet(1), I::LocalGet(0), I::StructGet { struct_type_index: list, field_index: 0 }, I::I64ExtendI32U, I::I64GtS, I::I32Or,
		]);
		self.emit_fail_if(func, "index_out_of_range");
		Self::emit_list(func, &[
			I::LocalGet(0), I::StructGet { struct_type_index: list, field_index: 1 },
			I::LocalGet(1), I::I32WrapI64, I::I32Const(1), I::I32Sub,
		]);
	}
}
