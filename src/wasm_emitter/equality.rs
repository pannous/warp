//! Structural equality and truthiness of Nodes at runtime.
//!
//! `values_equal(a, b)` compares by value, never by identity (wiki/equality.md): Nodes by kind and payload,
//! texts byte by byte (source text is NFC normalized), Int and Float numerically, lists and keys recursively.
//! `is_truthy(x)` applies the rule `Node::is_falsy` already uses for `and`/`or`: empty values are falsy.

use super::WasmGcEmitter;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use crate::type_kinds::{any_heap_type, Kind};
use wasm_encoder::*;
use Instruction as I;
use ValType::Ref;

pub const VALUES_EQUAL: &str = "values_equal";
pub const IS_TRUTHY: &str = "is_truthy";
const KIND_MASK: i64 = 0xFF;
const MEMORY: MemArg = MemArg { offset: 0, align: 0, memory_index: 0 };

impl WasmGcEmitter {
	/// Values that only compare or test structurally: they have no numeric reading
	pub(crate) fn is_structured_value(&self, node: &Node) -> bool {
		match node.drop_meta() {
			Node::Empty | Node::Text(_) | Node::List(_, Bracket::Square, _) => true,
			Node::List(items, Bracket::Round, _) if items.len() == 1 => self.is_structured_value(&items[0]),
			// an indexed element is a Node of any kind: 'héllo'#2 is a codepoint
			Node::Key(indexed, Op::Hash, _) => !matches!(indexed.drop_meta(), Node::Empty),
			Node::Symbol(name) => {
				let bound = self.scope.lookup(name).is_some() || self.ctx.user_globals.contains_key(name);
				bound && self.get_type(node).is_ref()
			}
			_ => false,
		}
	}

	pub(crate) fn compares_structurally(&self, op: &Op, left: &Node, right: &Node) -> bool {
		matches!(op, Op::Eq | Op::Ne) && (self.is_structured_value(left) || self.is_structured_value(right))
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

	/// Push i32 truth value of an if/while condition
	pub(crate) fn emit_condition(&mut self, func: &mut Function, condition: &Node, emit_number: fn(&mut Self, &mut Function, &Node)) {
		if self.is_structured_value(condition) {
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
	}

	fn any_ref() -> ValType {
		Ref(RefType { nullable: true, heap_type: any_heap_type() })
	}

	fn i31_heap() -> HeapType {
		HeapType::Abstract { shared: false, ty: AbstractHeapType::I31 }
	}

	fn test(func: &mut Function, local: u32, heap: HeapType) {
		func.instruction(&I::LocalGet(local));
		func.instruction(&I::RefTestNonNull(heap));
	}

	fn field(func: &mut Function, local: u32, struct_type: u32, field_index: u32) {
		func.instruction(&I::LocalGet(local));
		func.instruction(&I::RefCastNonNull(HeapType::Concrete(struct_type)));
		func.instruction(&I::StructGet { struct_type_index: struct_type, field_index });
	}

	fn return_if(func: &mut Function, result: i32) {
		func.instruction(&I::If(BlockType::Empty));
		func.instruction(&I::I32Const(result));
		func.instruction(&I::Return);
		func.instruction(&I::End);
	}

	fn number_as_f64(&self, func: &mut Function, local: u32) {
		let (i64_box, f64_box) = (self.type_manager.i64_box_type, self.type_manager.f64_box_type);
		Self::test(func, local, HeapType::Concrete(f64_box));
		func.instruction(&I::If(BlockType::Result(ValType::F64)));
		Self::field(func, local, f64_box, 0);
		func.instruction(&I::Else);
		Self::field(func, local, i64_box, 0);
		func.instruction(&I::F64ConvertI64S);
		func.instruction(&I::End);
	}

	fn is_number_payload(&self, func: &mut Function, local: u32) {
		Self::test(func, local, HeapType::Concrete(self.type_manager.i64_box_type));
		Self::test(func, local, HeapType::Concrete(self.type_manager.f64_box_type));
		func.instruction(&I::I32Or);
	}

	fn is_number_kind(func: &mut Function, kind_local: u32) {
		for kind in [Kind::Int, Kind::Float] {
			func.instruction(&I::LocalGet(kind_local));
			func.instruction(&I::I64Const(kind as i64));
			func.instruction(&I::I64Eq);
		}
		func.instruction(&I::I32Or);
	}

	/// values_equal(a: anyref, b: anyref) -> i32; locals: kind a, kind b, ptr a, ptr b, remaining bytes
	fn emit_values_equal(&mut self) {
		let recurse = self.next_func_idx;
		let node = self.type_manager.node_type;
		let string = self.type_manager.string_type;
		let (i64_box, big_int) = (self.type_manager.i64_box_type, self.type_manager.big_int_type);
		let int_runtime = self.int_runtime();
		let (i64t, i32t) = (ValType::I64, ValType::I32);
		let locals = vec![i64t, i64t, i32t, i32t, i32t];
		self.runtime_function(VALUES_EQUAL, vec![Self::any_ref(), Self::any_ref()], vec![i32t], locals, |s, f| {
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
				f.instruction(&I::LocalSet(to));
			}
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
				f.instruction(&I::I32Load8U(MEMORY));
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
			s.is_number_payload(f, 0);
			s.is_number_payload(f, 1);
			f.instruction(&I::I32And);
			f.instruction(&I::If(BlockType::Empty));
			s.number_as_f64(f, 0);
			s.number_as_f64(f, 1);
			f.instruction(&I::F64Eq);
			f.instruction(&I::Return);
			f.instruction(&I::End);

			// BigInts are normalized, so only two BigInts can be equal
			if int_runtime {
				Self::test(f, 0, HeapType::Concrete(big_int));
				Self::test(f, 1, HeapType::Concrete(big_int));
				f.instruction(&I::I32And);
				f.instruction(&I::If(BlockType::Empty));
				for local in [0, 1] {
					f.instruction(&I::LocalGet(local));
					s.call(f, "int_from_payload");
				}
				s.call(f, "int_cmp");
				f.instruction(&I::I32Eqz);
				f.instruction(&I::Return);
				f.instruction(&I::End);
			}
			f.instruction(&I::I32Const(0));
		});
		let idx = self.func_index(VALUES_EQUAL);
		self.exports.export(VALUES_EQUAL, ExportKind::Func, idx);
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
			// ø is falsy
			f.instruction(&I::LocalGet(1));
			f.instruction(&I::I64Const(Kind::Empty as i64));
			f.instruction(&I::I64Eq);
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
		let idx = self.func_index(IS_TRUTHY);
		self.exports.export(IS_TRUTHY, ExportKind::Func, idx);
	}
}
