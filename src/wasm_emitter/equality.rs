//! Structural equality and truthiness of Nodes at runtime.
//!
//! `values_equal(a, b)` compares by value, never by identity (wiki/equality.md): Nodes by kind and payload,
//! texts byte by byte (source text is NFC normalized), Int and Float numerically, lists and keys recursively.
//! `is_truthy(x)` applies the rule `Node::is_falsy` already uses for `and`/`or`: empty values and errors are falsy.

use super::WasmGcEmitter;
use crate::diagnostic::Diagnostic;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_kinds::{any_heap_type, Kind, KIND_MASK};
use wasm_encoder::*;
use Instruction as I;
use ValType::Ref;
use crate::wasm_emitter::layout::BYTE;

pub const VALUES_EQUAL: &str = "values_equal";
pub const IS_META_ENTRY: &str = "is_meta_entry";
pub const IS_TRUTHY: &str = "is_truthy";
/// same_node(a, b): are two objects one (P208); two other values compare by value and boolness
pub const SAME_NODE: &str = "same_node";

impl WasmGcEmitter {
	/// Values that only compare or test structurally: they have no numeric reading
	pub(crate) fn is_structured_value(&self, node: &Node) -> bool {
		match node.drop_meta() {
			Node::Empty | Node::Text(_) | Node::List(_, Bracket::Square, _) => true,
			Node::List(items, Bracket::Round, _) if items.len() == 1 => self.is_structured_value(&items[0]),
			// a tuple `(y, 4)`: compared element by element, never as its last value
			Node::List(_, Bracket::Round, Separator::Colon) => true,
			// a call whose result is a text or a list: `lower(a) == lower(b)`
			Node::List(_, Bracket::Round, Separator::None) => matches!(self.get_type(node), Kind::Text | Kind::Codepoint | Kind::List),
			// an indexed element is a Node of any kind: 'héllo'#2 is a codepoint
			Node::Key(indexed, Op::Hash, _) => !matches!(indexed.drop_meta(), Node::Empty),
			Node::Symbol(name) => {
				let bound = self.scope.lookup(name).is_some() || self.ctx.user_globals.contains_key(name);
				bound && self.get_type(node).is_ref()
			}
			_ => false,
		}
	}

	/// A list has no order against anything: `<`, `>`, `<=`, `>=` with a list operand is an error
	pub(crate) fn orders_a_list(&self, op: &Op, left: &Node, right: &Node) -> bool {
		let is_list = |node: &Node| match node.drop_meta() {
			Node::List(_, Bracket::Square, _) => true,
			Node::Symbol(_) => self.is_structured_value(node) && self.get_type(node) == Kind::List,
			_ => false,
		};
		op.is_ordering() && (is_list(left) || is_list(right))
	}

	pub(crate) fn emit_unordered_list_error(&mut self, func: &mut Function, left: &Node, op: &Op, right: &Node) {
		let message = format!("cannot compare a list with `{op}`: {} {op} {}", left.serialize(), right.serialize());
		self.emit_type_error(func, Diagnostic::at(left, message).to_string());
	}

	/// An object literal `{a:1 b:2}`, or an instance `point:{x:1 y:2}`: compared by its entries, never read as a number
	fn is_object_literal(node: &Node) -> bool {
		matches!(node.drop_meta(), Node::List(_, Bracket::Curly, _) | Node::Key(_, Op::Colon, _))
	}

	/// A value that is tested or compared as a Node: a structured value, an object, or either of them in parentheses
	fn is_structural_operand(&self, node: &Node) -> bool {
		match node.drop_meta() {
			Node::List(items, Bracket::Round, _) if items.len() == 1 => self.is_structural_operand(&items[0]),
			other => self.is_structured_value(other) || Self::is_object_literal(other),
		}
	}

	/// `==`/`!=` by value: structured values, any text (`peek() == " "` of a function returning text), and a value held
	/// as a Node whose kind shows only at run time (a cell's, `cell_get(c) == 5`)
	pub(crate) fn compares_structurally(&self, op: &Op, left: &Node, right: &Node) -> bool {
		let by_value = |node: &Node| self.get_type(node) == Kind::Text || self.is_held_cell_value(node);
		matches!(op, Op::Eq | Op::Ne) && (self.is_structural_operand(left) || self.is_structural_operand(right) || by_value(left) || by_value(right))
	}

	/// `a same b` / `a === b` of two values that may be objects (instances, maps, lists): are they one (P208)? The
	/// sides and whether the question is negated (`!==`); texts, ø and numbers known as such are no objects, `===`
	/// compares their type and value
	pub(super) fn object_identity<'a>(&self, node: &'a Node) -> Option<(&'a Node, &'a Node, bool)> {
		let Node::Key(left, op @ (Op::Identical | Op::NotIdentical), right) = node.drop_meta() else { return None };
		let may_be_object = |side: &Node| !matches!(side.drop_meta(), Node::Empty | Node::Text(_))
			&& (self.is_structural_operand(side) || self.get_type(side).is_ref())
			&& !matches!(self.get_type(side), Kind::Text | Kind::Codepoint | Kind::Empty);
		(may_be_object(left) && may_be_object(right)).then_some((left, right, *op == Op::NotIdentical))
	}

	/// Push i64 1/0: are the two objects one
	pub(super) fn emit_object_identity(&mut self, func: &mut Function, (left, right, negated): (&Node, &Node, bool)) {
		self.emit_node_instructions(func, left);
		self.emit_node_instructions(func, right);
		self.emit_call(func, SAME_NODE);
		if negated {
			func.instruction(&I::I32Eqz);
		}
		func.instruction(&I::I64ExtendI32U);
	}

	/// `a === b`: `a == b` of two values of the same type, else false (`!==` true): `0 === false` and `1 === 1.5` are
	/// false (P196, card zero-false). A side whose static type is unknown (`value:any`) is told apart by its boolness only
	pub(super) fn identity_as_equality(&self, node: &Node) -> Option<Node> {
		let Node::Key(left, op @ (Op::Identical | Op::NotIdentical), right) = node.drop_meta() else { return None };
		let known_type = |side: &Node| Some(self.static_type_name(side)).filter(|name| name != "empty" || matches!(side.drop_meta(), Node::Empty));
		let same_type = match (known_type(left), known_type(right)) {
			(Some(left_type), Some(right_type)) => left_type == right_type,
			_ => crate::analyzer::is_boolean(left, &self.scope) == crate::analyzer::is_boolean(right, &self.scope),
		};
		let identical = *op == Op::Identical;
		if !same_type {
			return Some(if identical { Node::False } else { Node::True });
		}
		let equality = if identical { Op::Eq } else { Op::Ne };
		Some(Node::Key(left.clone(), equality, right.clone()))
	}

	/// `cell_get(c)`: a cell's value, a Node of any kind
	fn is_held_cell_value(&self, node: &Node) -> bool {
		matches!(node.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(|word| word.drop_meta().name() == super::cells::CELL_GET))
	}

	/// Push i64 1/0 for `left == right` or `left != right` compared by value
	pub(crate) fn emit_structural_equality(&mut self, func: &mut Function, left: &Node, op: &Op, right: &Node) {
		self.emit_node_instructions(func, left);
		self.emit_node_instructions(func, right);
		self.emit_call(func, VALUES_EQUAL);
		if *op == Op::Ne {
			func.instruction(&I::I32Eqz);
		}
		func.instruction(&I::I64ExtendI32U);
	}

	/// Push i32 truth value of an if/while condition; a structured value or a cell's value is tested as a Node
	pub(crate) fn emit_condition(&mut self, func: &mut Function, condition: &Node, emit_number: fn(&mut Self, &mut Function, &Node)) {
		if self.is_structural_operand(condition) || self.is_held_cell_value(condition) {
			self.emit_node_instructions(func, condition);
			self.emit_call(func, IS_TRUTHY);
		} else if self.get_type(condition).is_float() {
			self.emit_float_value(func, condition);
			func.instruction(&I::F64Const(Ieee64::new(0.0f64.to_bits())));
			func.instruction(&I::F64Ne);
		} else {
			emit_number(self, func, condition);
			func.instruction(&I::I64Const(0));
			func.instruction(&I::I64Ne); // not I32WrapI64: 2^32 is truthy
		}
	}

	pub(crate) fn emit_equality_ops(&mut self) {
		if self.should_emit_function(VALUES_EQUAL) {
			self.emit_values_equal();
		}
		if self.should_emit_function(IS_TRUTHY) {
			self.emit_is_truthy();
		}
		if self.should_emit_function(SAME_NODE) {
			self.emit_same_node();
		}
	}

	fn emit_same_node(&mut self) {
		let node_ref = Ref(self.node_ref(false));
		let node_type = self.type_manager.node_type;
		let kind_of = |f: &mut Function, local: u32| Self::emit_list(f, &[I::LocalGet(local), I::StructGet { struct_type_index: node_type, field_index: 0 }]);
		self.runtime_function(SAME_NODE, vec![node_ref, node_ref], vec![ValType::I32], vec![ValType::I64], |s, f| {
			let (left, right, kind) = (0, 1, 2);
			kind_of(f, left);
			Self::emit_list(f, &[I::I64Const(crate::type_kinds::KIND_MASK), I::I64And, I::LocalTee(kind), I::I64Const(Kind::Key as i64), I::I64Eq]);
			Self::emit_list(f, &[I::LocalGet(kind), I::I64Const(Kind::List as i64), I::I64Eq, I::I32Or, I::If(BlockType::Empty)]);
			Self::emit_list(f, &[I::LocalGet(left), I::LocalGet(right), I::RefEq, I::Return, I::End, I::LocalGet(left), I::LocalGet(right)]);
			s.emit_call(f, VALUES_EQUAL);
			for side in [left, right] {
				kind_of(f, side);
				f.instruction(&I::I64Const(crate::type_kinds::BOOL_KIND));
				f.instruction(&I::I64Eq);
			}
			Self::emit_list(f, &[I::I32Eq, I::I32And]);
		});
	}

	/// is_meta_entry(entry: anyref) -> i32: 1 for a meta entry `@name:value`, which is never a field, not counted and
	/// ignored by == (user decision "Meta information on objects")
	pub(crate) fn emit_is_meta_entry(&mut self) {
		let (node, string) = (self.type_manager.node_type, self.type_manager.string_type);
		let string_ref = Ref(RefType { nullable: true, heap_type: HeapType::Concrete(string) });
		let (entry, key, name) = (0, 1, 2);
		self.runtime_function(IS_META_ENTRY, vec![Self::any_ref()], vec![ValType::I32], vec![Self::any_ref(), string_ref], |_, f| {
			let is_node_of_kind = |f: &mut Function, local: u32, kind: Kind| {
				Self::test(f, local, HeapType::Concrete(node));
				f.instruction(&I::I32Eqz);
				Self::return_if(f, 0);
				Self::field(f, local, node, 0);
				f.instruction(&I::I64Const(KIND_MASK));
				f.instruction(&I::I64And);
				f.instruction(&I::I64Const(kind as i64));
				f.instruction(&I::I64Ne);
				Self::return_if(f, 0);
			};
			is_node_of_kind(f, entry, Kind::Key);
			Self::field(f, entry, node, 1);
			f.instruction(&I::LocalSet(key));
			is_node_of_kind(f, key, Kind::Symbol);
			Self::field(f, key, node, 1);
			f.instruction(&I::RefCastNonNull(HeapType::Concrete(string)));
			f.instruction(&I::LocalSet(name));
			f.instruction(&I::LocalGet(name));
			f.instruction(&I::StructGet { struct_type_index: string, field_index: 1 });
			f.instruction(&I::I32Eqz);
			Self::return_if(f, 0);
			f.instruction(&I::LocalGet(name));
			f.instruction(&I::StructGet { struct_type_index: string, field_index: 0 });
			f.instruction(&I::I32Load8U(BYTE));
			f.instruction(&I::I32Const(crate::node::ATTRIBUTE_MARK as i32));
			f.instruction(&I::I32Eq);
		});
	}

	/// A list cell (local `cell`) whose entry is a meta entry: compare its rest instead, `values_equal(rest, other)`;
	/// `extra` pushes the comparison's further arguments (values_similar's tolerance)
	pub(super) fn skip_meta_cell(&self, f: &mut Function, cell: u32, other: u32, recurse: u32, extra: &[I<'static>]) {
		let node = self.type_manager.node_type;
		Self::test(f, cell, HeapType::Concrete(node));
		f.instruction(&I::If(BlockType::Empty));
		Self::field(f, cell, node, 0);
		f.instruction(&I::I64Const(KIND_MASK));
		f.instruction(&I::I64And);
		f.instruction(&I::I64Const(Kind::List as i64));
		f.instruction(&I::I64Eq);
		f.instruction(&I::If(BlockType::Empty));
		Self::field(f, cell, node, 1);
		self.call(f, IS_META_ENTRY);
		f.instruction(&I::If(BlockType::Empty));
		let push_rest = |f: &mut Function| Self::field(f, cell, node, 2);
		if cell == 0 {
			push_rest(f);
			f.instruction(&I::LocalGet(other));
		} else {
			f.instruction(&I::LocalGet(other));
			push_rest(f);
		}
		Self::emit_list(f, extra);
		f.instruction(&I::Call(recurse));
		f.instruction(&I::Return);
		f.instruction(&I::End);
		f.instruction(&I::End);
		f.instruction(&I::End);
	}

	pub(super) fn any_ref() -> ValType {
		Ref(RefType { nullable: true, heap_type: any_heap_type() })
	}

	pub(super) fn i31_heap() -> HeapType {
		HeapType::Abstract { shared: false, ty: AbstractHeapType::I31 }
	}

	pub(super) fn test(func: &mut Function, local: u32, heap: HeapType) {
		func.instruction(&I::LocalGet(local));
		func.instruction(&I::RefTestNonNull(heap));
	}

	pub(super) fn field(func: &mut Function, local: u32, struct_type: u32, field_index: u32) {
		func.instruction(&I::LocalGet(local));
		func.instruction(&I::RefCastNonNull(HeapType::Concrete(struct_type)));
		func.instruction(&I::StructGet { struct_type_index: struct_type, field_index });
	}

	/// `"A"` lexes as the character 'A': a character equals the one-character text of it, compared as that text.
	/// The kinds of the two Nodes (locals 0, 1) are in locals 2, 3.
	fn emit_character_equals_text(&self, func: &mut Function, node: u32, recurse: u32) {
		for (character, text) in [(0, 1), (1, 0)] {
			let has_kind = |func: &mut Function, local: u32, kind: Kind| {
				func.instruction(&I::LocalGet(local + 2));
				func.instruction(&I::I64Const(kind as i64));
				func.instruction(&I::I64Eq);
			};
			has_kind(func, character, Kind::Codepoint);
			has_kind(func, text, Kind::Text);
			func.instruction(&I::I32And);
			func.instruction(&I::If(BlockType::Empty));
			for local in [0, 1] {
				func.instruction(&I::LocalGet(local));
				if local == character {
					func.instruction(&I::RefCastNonNull(HeapType::Concrete(node)));
					self.call(func, crate::wasm_emitter::text_builtins::TEXT_OF);
				}
			}
			func.instruction(&I::Call(recurse));
			func.instruction(&I::Return);
			func.instruction(&I::End);
		}
	}

	pub(super) fn return_if(func: &mut Function, result: i32) {
		func.instruction(&I::If(BlockType::Empty));
		func.instruction(&I::I32Const(result));
		func.instruction(&I::Return);
		func.instruction(&I::End);
	}

	/// The f64 of a number payload: a float's value, an Int converted (an exact one, a ratio too, by exact_to_f64)
	pub(super) fn number_as_f64(&self, func: &mut Function, local: u32) {
		let (i64_box, f64_box) = (self.type_manager.i64_box_type, self.type_manager.f64_box_type);
		Self::test(func, local, HeapType::Concrete(f64_box));
		func.instruction(&I::If(BlockType::Result(ValType::F64)));
		Self::field(func, local, f64_box, 0);
		func.instruction(&I::Else);
		if self.int_runtime() {
			func.instruction(&I::LocalGet(local));
			self.call(func, "int_from_payload");
			self.call(func, "exact_to_f64");
		} else {
			Self::field(func, local, i64_box, 0);
			func.instruction(&I::F64ConvertI64S);
		}
		func.instruction(&I::End);
	}

	/// The payload heap types of numbers: boxed ints and floats, with the int runtime also big ints and ratios
	fn number_payload_types(&self) -> Vec<u32> {
		let types = &self.type_manager;
		let exact = if self.int_runtime() { vec![types.big_int_type, types.ratio_type] } else { vec![] };
		[vec![types.i64_box_type, types.f64_box_type], exact].concat()
	}

	pub(super) fn is_number_payload(&self, func: &mut Function, local: u32) {
		for (index, heap) in self.number_payload_types().into_iter().enumerate() {
			Self::test(func, local, HeapType::Concrete(heap));
			if index > 0 {
				func.instruction(&I::I32Or);
			}
		}
	}

	/// An Int, a Float or a bool (an Int marked bool: `false == 0`, card bool-type)
	pub(super) fn is_number_kind(func: &mut Function, kind_local: u32) {
		let kinds = [Kind::Int as i64, Kind::Float as i64, crate::type_kinds::BOOL_KIND];
		for kind in kinds {
			func.instruction(&I::LocalGet(kind_local));
			func.instruction(&I::I64Const(kind));
			func.instruction(&I::I64Eq);
		}
		for _ in 1..kinds.len() {
			func.instruction(&I::I32Or);
		}
	}

	/// Returns from values_equal: two lists are equal as sets of entries when every entry of each has an equal entry in the other.
	/// Locals: 7 and 8 walk the cells of the list being checked and of the other, 9 flags that the entry was found.
	/// `extra` pushes the comparison's further arguments (values_similar's tolerance)
	pub(super) fn emit_unordered_entries_equal(f: &mut Function, node: u32, recurse: u32, is_meta_entry: u32, extra: &[I<'static>]) {
		let (walked, other, found) = (7, 8, 9);
		let cell_field = |f: &mut Function, local: u32, field_index: u32| {
			f.instruction(&I::LocalGet(local));
			f.instruction(&I::RefAsNonNull);
			f.instruction(&I::StructGet { struct_type_index: node, field_index });
		};
		for (from, against) in [(0, 1), (1, 0)] {
			for (local, list) in [(walked, from), (other, against)] {
				f.instruction(&I::LocalGet(list));
				f.instruction(&I::RefCastNonNull(HeapType::Concrete(node)));
				f.instruction(&I::LocalSet(local));
			}
			f.instruction(&I::Block(BlockType::Empty));
			f.instruction(&I::Loop(BlockType::Empty));
			f.instruction(&I::LocalGet(walked));
			f.instruction(&I::RefIsNull);
			f.instruction(&I::BrIf(1));
			cell_field(f, walked, 1);
			f.instruction(&I::Call(is_meta_entry));
			f.instruction(&I::LocalSet(found)); // a meta entry needs no partner
			f.instruction(&I::LocalGet(against));
			f.instruction(&I::RefCastNonNull(HeapType::Concrete(node)));
			f.instruction(&I::LocalSet(other));
			f.instruction(&I::Block(BlockType::Empty));
			f.instruction(&I::Loop(BlockType::Empty));
			f.instruction(&I::LocalGet(other));
			f.instruction(&I::RefIsNull);
			f.instruction(&I::BrIf(1));
			cell_field(f, walked, 1);
			cell_field(f, other, 1);
			Self::emit_list(f, extra);
			f.instruction(&I::Call(recurse));
			f.instruction(&I::If(BlockType::Empty));
			f.instruction(&I::I32Const(1));
			f.instruction(&I::LocalSet(found));
			f.instruction(&I::Br(2));
			f.instruction(&I::End);
			cell_field(f, other, 2);
			f.instruction(&I::LocalSet(other));
			f.instruction(&I::Br(0));
			f.instruction(&I::End);
			f.instruction(&I::End);
			f.instruction(&I::LocalGet(found));
			f.instruction(&I::I32Eqz);
			Self::return_if(f, 0);
			cell_field(f, walked, 2);
			f.instruction(&I::LocalSet(walked));
			f.instruction(&I::Br(0));
			f.instruction(&I::End);
			f.instruction(&I::End);
		}
		f.instruction(&I::I32Const(1));
		f.instruction(&I::Return);
	}

	/// values_equal(a: anyref, b: anyref) -> i32; locals: kind a, kind b, ptr a, ptr b, remaining bytes
	fn emit_values_equal(&mut self) {
		let recurse = self.next_func_idx;
		let node = self.type_manager.node_type;
		let string = self.type_manager.string_type;
		let (i64_box, f64_box, big_int) = (self.type_manager.i64_box_type, self.type_manager.f64_box_type, self.type_manager.big_int_type);
		let ratio = self.type_manager.ratio_type;
		let int_runtime = self.int_runtime();
		let (i64t, i32t) = (ValType::I64, ValType::I32);
		let cell = Ref(self.node_ref(true));
		let locals = vec![i64t, i64t, i32t, i32t, i32t, cell, cell, i32t];
		self.runtime_function(VALUES_EQUAL, vec![Self::any_ref(), Self::any_ref()], vec![i32t], locals, |s, f| {
			s.skip_meta_cell(f, 0, 1, recurse, &[]);
			s.skip_meta_cell(f, 1, 0, recurse, &[]);
			// null equals only null
			f.instruction(&I::LocalGet(0));
			f.instruction(&I::RefIsNull);
			f.instruction(&I::If(BlockType::Empty));
			f.instruction(&I::LocalGet(1));
			f.instruction(&I::RefIsNull);
			f.instruction(&I::Return);
			f.instruction(&I::End);
			f.instruction(&I::LocalGet(1));
			f.instruction(&I::RefIsNull);
			Self::return_if(f, 0);

			// Nodes: same kind (Int and Float are compatible), then equal data and equal value
			Self::test(f, 0, HeapType::Concrete(node));
			f.instruction(&I::If(BlockType::Empty));
			Self::test(f, 1, HeapType::Concrete(node));
			f.instruction(&I::I32Eqz);
			Self::return_if(f, 0);
			for (from, to) in [(0, 2), (1, 3)] {
				Self::field(f, from, node, 0);
				Self::emit_list(f, &[I::I64Const(crate::type_kinds::UNMARKED_KIND), I::I64And, I::LocalSet(to)]);
			}
			s.emit_character_equals_text(f, node, recurse);
			f.instruction(&I::LocalGet(2));
			f.instruction(&I::LocalGet(3));
			f.instruction(&I::I64Ne);
			f.instruction(&I::If(BlockType::Empty));
			Self::is_number_kind(f, 2);
			Self::is_number_kind(f, 3);
			f.instruction(&I::I32And);
			f.instruction(&I::I32Eqz);
			Self::return_if(f, 0);
			f.instruction(&I::End);
			// `{…}` objects are maps: their entries may come in any order
			for kind in [2, 3] {
				f.instruction(&I::LocalGet(kind));
				f.instruction(&I::I64Const(Kind::List as i64));
				f.instruction(&I::I64Eq);
			}
			f.instruction(&I::I32And);
			f.instruction(&I::If(BlockType::Empty));
			Self::emit_unordered_entries_equal(f, node, recurse, s.func_index(IS_META_ENTRY), &[]);
			f.instruction(&I::End);
			Self::field(f, 0, node, 1);
			Self::field(f, 1, node, 1);
			f.instruction(&I::Call(recurse));
			f.instruction(&I::If(BlockType::Result(i32t)));
			Self::field(f, 0, node, 2);
			Self::field(f, 1, node, 2);
			f.instruction(&I::Call(recurse));
			f.instruction(&I::Else);
			f.instruction(&I::I32Const(0));
			f.instruction(&I::End);
			f.instruction(&I::Return);
			f.instruction(&I::End);

			// Codepoints
			Self::test(f, 0, Self::i31_heap());
			f.instruction(&I::If(BlockType::Empty));
			Self::test(f, 1, Self::i31_heap());
			f.instruction(&I::If(BlockType::Result(i32t)));
			for local in [0, 1] {
				f.instruction(&I::LocalGet(local));
				f.instruction(&I::RefCastNonNull(Self::i31_heap()));
				f.instruction(&I::I31GetS);
			}
			f.instruction(&I::I32Eq);
			f.instruction(&I::Else);
			f.instruction(&I::I32Const(0));
			f.instruction(&I::End);
			f.instruction(&I::Return);
			f.instruction(&I::End);

			// Texts: equal length, then equal bytes
			Self::test(f, 0, HeapType::Concrete(string));
			f.instruction(&I::If(BlockType::Empty));
			Self::test(f, 1, HeapType::Concrete(string));
			f.instruction(&I::I32Eqz);
			Self::return_if(f, 0);
			Self::field(f, 0, string, 1);
			Self::field(f, 1, string, 1);
			f.instruction(&I::I32Ne);
			Self::return_if(f, 0);
			for (from, field_index, to) in [(0, 0, 4), (1, 0, 5), (0, 1, 6)] {
				Self::field(f, from, string, field_index);
				f.instruction(&I::LocalSet(to));
			}
			f.instruction(&I::Block(BlockType::Empty));
			f.instruction(&I::Loop(BlockType::Empty));
			f.instruction(&I::LocalGet(6));
			f.instruction(&I::I32Eqz);
			f.instruction(&I::BrIf(1));
			for pointer in [4, 5] {
				f.instruction(&I::LocalGet(pointer));
				f.instruction(&I::I32Load8U(BYTE));
			}
			f.instruction(&I::I32Ne);
			Self::return_if(f, 0);
			for (local, step) in [(4, 1), (5, 1), (6, -1)] {
				f.instruction(&I::LocalGet(local));
				f.instruction(&I::I32Const(step));
				f.instruction(&I::I32Add);
				f.instruction(&I::LocalSet(local));
			}
			f.instruction(&I::Br(0));
			f.instruction(&I::End);
			f.instruction(&I::End);
			f.instruction(&I::I32Const(1));
			f.instruction(&I::Return);
			f.instruction(&I::End);

			// Ints exactly, Int and Float numerically
			Self::test(f, 0, HeapType::Concrete(i64_box));
			Self::test(f, 1, HeapType::Concrete(i64_box));
			f.instruction(&I::I32And);
			f.instruction(&I::If(BlockType::Empty));
			Self::field(f, 0, i64_box, 0);
			Self::field(f, 1, i64_box, 0);
			f.instruction(&I::I64Eq);
			f.instruction(&I::Return);
			f.instruction(&I::End);
			// BigInts and ratios are normalized (a ratio is never integral), so only two of the same kind can be equal
			if int_runtime {
				for heap in [big_int, ratio] {
					Self::test(f, 0, HeapType::Concrete(heap));
					Self::test(f, 1, HeapType::Concrete(heap));
					f.instruction(&I::I32And);
					f.instruction(&I::If(BlockType::Empty));
					for local in [0, 1] {
						f.instruction(&I::LocalGet(local));
						s.call(f, "int_from_payload");
					}
					s.call(f, "exact_cmp");
					f.instruction(&I::I32Eqz);
					f.instruction(&I::Return);
					f.instruction(&I::End);
				}
			}
			// a float and any number numerically: 0.5 equals the exact 1/2
			s.is_number_payload(f, 0);
			s.is_number_payload(f, 1);
			f.instruction(&I::I32And);
			for local in [0, 1] {
				Self::test(f, local, HeapType::Concrete(f64_box));
			}
			Self::emit_list(f, &[I::I32Or, I::I32And, I::If(BlockType::Empty)]);
			s.number_as_f64(f, 0);
			s.number_as_f64(f, 1);
			Self::emit_list(f, &[I::F64Eq, I::Return, I::End]);
			f.instruction(&I::I32Const(0));
		});
		self.export_runtime_function(VALUES_EQUAL);
	}

	/// is_truthy(x: anyref) -> i32, the rule of `Node::is_falsy`; locals: kind
	fn emit_is_truthy(&mut self) {
		let recurse = self.next_func_idx;
		let node = self.type_manager.node_type;
		let string = self.type_manager.string_type;
		let (i64_box, f64_box) = (self.type_manager.i64_box_type, self.type_manager.f64_box_type);
		let i32t = ValType::I32;
		self.runtime_function(IS_TRUTHY, vec![Self::any_ref()], vec![i32t], vec![ValType::I64], |_, f| {
			f.instruction(&I::LocalGet(0));
			f.instruction(&I::RefIsNull);
			Self::return_if(f, 0);

			Self::test(f, 0, HeapType::Concrete(node));
			f.instruction(&I::If(BlockType::Empty));
			Self::field(f, 0, node, 0);
			f.instruction(&I::I64Const(KIND_MASK));
			f.instruction(&I::I64And);
			f.instruction(&I::LocalSet(1));
			// ø and errors are falsy: `if x {…}` is the check of a result that may have failed (fetch)
			f.instruction(&I::LocalGet(1));
			f.instruction(&I::I64Const(Kind::Empty as i64));
			f.instruction(&I::I64Eq);
			f.instruction(&I::LocalGet(1));
			f.instruction(&I::I64Const(Kind::Error as i64));
			f.instruction(&I::I64Eq);
			f.instruction(&I::I32Or);
			Self::return_if(f, 0);
			// lists and blocks are truthy when they have a first element
			f.instruction(&I::LocalGet(1));
			f.instruction(&I::I64Const(Kind::Block as i64));
			f.instruction(&I::I64Eq);
			f.instruction(&I::LocalGet(1));
			f.instruction(&I::I64Const(Kind::List as i64));
			f.instruction(&I::I64Eq);
			f.instruction(&I::I32Or);
			f.instruction(&I::If(BlockType::Empty));
			Self::field(f, 0, node, 1);
			f.instruction(&I::RefIsNull);
			f.instruction(&I::I32Eqz);
			f.instruction(&I::Return);
			f.instruction(&I::End);
			// keys are falsy only when key and value are
			f.instruction(&I::LocalGet(1));
			f.instruction(&I::I64Const(Kind::Key as i64));
			f.instruction(&I::I64Eq);
			f.instruction(&I::If(BlockType::Empty));
			Self::field(f, 0, node, 1);
			f.instruction(&I::Call(recurse));
			f.instruction(&I::If(BlockType::Result(i32t)));
			f.instruction(&I::I32Const(1));
			f.instruction(&I::Else);
			Self::field(f, 0, node, 2);
			f.instruction(&I::Call(recurse));
			f.instruction(&I::End);
			f.instruction(&I::Return);
			f.instruction(&I::End);
			Self::field(f, 0, node, 1);
			f.instruction(&I::Call(recurse));
			f.instruction(&I::Return);
			f.instruction(&I::End);

			// payloads: zero numbers, '\0' and empty text are falsy
			Self::test(f, 0, Self::i31_heap());
			f.instruction(&I::If(BlockType::Empty));
			f.instruction(&I::LocalGet(0));
			f.instruction(&I::RefCastNonNull(Self::i31_heap()));
			f.instruction(&I::I31GetS);
			f.instruction(&I::I32Const(0));
			f.instruction(&I::I32Ne);
			f.instruction(&I::Return);
			f.instruction(&I::End);
			Self::test(f, 0, HeapType::Concrete(string));
			f.instruction(&I::If(BlockType::Empty));
			Self::field(f, 0, string, 1);
			f.instruction(&I::I32Const(0));
			f.instruction(&I::I32Ne);
			f.instruction(&I::Return);
			f.instruction(&I::End);
			Self::test(f, 0, HeapType::Concrete(i64_box));
			f.instruction(&I::If(BlockType::Empty));
			Self::field(f, 0, i64_box, 0);
			f.instruction(&I::I64Const(0));
			f.instruction(&I::I64Ne);
			f.instruction(&I::Return);
			f.instruction(&I::End);
			Self::test(f, 0, HeapType::Concrete(f64_box));
			f.instruction(&I::If(BlockType::Empty));
			Self::field(f, 0, f64_box, 0);
			f.instruction(&I::F64Const(Ieee64::new(0f64.to_bits())));
			f.instruction(&I::F64Ne);
			f.instruction(&I::Return);
			f.instruction(&I::End);
			f.instruction(&I::I32Const(1));
		});
		self.export_runtime_function(IS_TRUTHY);
	}
}
