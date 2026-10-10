//! A list of ints in one call between the host and the module (card g_odW4): an image's pixels crossed item by item,
//! two host calls a pixel, which took most of a gpu_render or paint frame. The host writes or reads the ints in linear
//! memory above the text heap (src/host.rs with_scratch, web/playground/host.js withScratch) and the module walks them.

use super::WasmGcEmitter;
use crate::type_kinds::{Kind, KIND_MASK};
use wasm_encoder::{BlockType, Function, HeapType, Instruction as I, MemArg, ValType};

/// ints_to_list(address, count) -> list: the `count` u32 at `address` as a square list of Ints (gpu_render's pixels)
pub const INTS_TO_LIST: &str = "ints_to_list";
/// list_to_ints(list, address, room) -> count: the list's first `room` Ints as i64 at `address` (paint's pixels, negatives
/// as their magnitude), -1 when an item is no fixnum Int and the host reads the list itself
pub const LIST_TO_INTS: &str = "list_to_ints";
const U32: MemArg = MemArg { offset: 0, align: 2, memory_index: 0 };
const I64: MemArg = MemArg { offset: 0, align: 3, memory_index: 0 };
const SQUARE_BRACKET: i64 = 1;
const NOT_INTS: i32 = -1;

impl WasmGcEmitter {
	pub(super) fn emit_int_lists(&mut self) {
		if self.should_emit_function(INTS_TO_LIST) || self.should_emit_function(LIST_TO_INTS) {
			self.emit_text_heap_global(); // the host finds the free memory above it
		}
		if self.should_emit_function(INTS_TO_LIST) {
			self.emit_ints_to_list();
		}
		if self.should_emit_function(LIST_TO_INTS) {
			self.emit_list_to_ints();
		}
	}

	fn emit_ints_to_list(&mut self) {
		let (nullable, node_ref) = (self.node_ref(true), self.node_ref(false));
		self.exported_function(INTS_TO_LIST, vec![ValType::I32, ValType::I32], vec![ValType::Ref(node_ref)], vec![ValType::Ref(nullable)], |s, f| {
			let (address, count, rest) = (0, 1, 2);
			Self::emit_list(f, &[I::LocalGet(count), I::I32Eqz, I::If(BlockType::Empty)]);
			s.call(f, "new_empty");
			Self::emit_list(f, &[I::Return, I::End]);
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(count), I::I32Eqz, I::BrIf(1)]);
			Self::emit_list(f, &[I::LocalGet(count), I::I32Const(1), I::I32Sub, I::LocalSet(count)]);
			Self::emit_list(f, &[I::LocalGet(address), I::LocalGet(count), I::I32Const(4), I::I32Mul, I::I32Add, I::I32Load(U32), I::I64ExtendI32U]);
			s.call(f, "new_int");
			Self::emit_list(f, &[I::LocalGet(rest), I::I64Const(SQUARE_BRACKET)]);
			s.call(f, "new_list");
			Self::emit_list(f, &[I::LocalSet(rest), I::Br(0), I::End, I::End, I::LocalGet(rest), I::RefAsNonNull]);
		});
	}

	fn emit_list_to_ints(&mut self) {
		let nullable = self.node_ref(true);
		let (node_type, box_type) = (self.type_manager.node_type, self.type_manager.i64_box_type);
		let locals = vec![ValType::I32, ValType::Ref(nullable), ValType::I64];
		self.exported_function(LIST_TO_INTS, vec![ValType::Ref(nullable), ValType::I32, ValType::I32], vec![ValType::I32], locals, |s, f| {
			let (cell, address, room, count, item, value) = (0, 1, 2, 3, 4, 5);
			let kind_is = |f: &mut Function, local: u32, kind: Kind| {
				s.emit_field(f, local, 0);
				Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(kind as i64), I::I64Eq]);
			};
			s.emit_empty_as_null(f, cell);
			Self::emit_list(f, &[I::LocalGet(cell), I::RefIsNull, I::If(BlockType::Empty), I::I32Const(0), I::Return, I::End]);
			kind_is(f, cell, Kind::List);
			Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty), I::I32Const(NOT_INTS), I::Return, I::End]);
			s.emit_cell_walk(f, cell, |f| {
				Self::emit_list(f, &[I::LocalGet(count), I::LocalGet(room), I::I32GeU, I::BrIf(1)]);
				s.emit_field(f, cell, 1);
				Self::emit_list(f, &[I::RefCastNullable(HeapType::Concrete(node_type)), I::LocalSet(item)]);
				// an Int whose payload is an $i64box (not a big Int's handle), else the host reads the list
				Self::emit_list(f, &[I::LocalGet(item), I::RefIsNull, I::If(BlockType::Empty), I::I32Const(NOT_INTS), I::Return, I::End]);
				kind_is(f, item, Kind::Int);
				s.emit_field(f, item, 1);
				Self::emit_list(f, &[I::RefTestNonNull(HeapType::Concrete(box_type)), I::I32And, I::I32Eqz, I::If(BlockType::Empty), I::I32Const(NOT_INTS), I::Return, I::End]);
				s.emit_field(f, item, 1);
				Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(box_type)), I::StructGet { struct_type_index: box_type, field_index: 0 }, I::LocalSet(value)]);
				// |value|: (value ^ sign) - sign
				Self::emit_list(f, &[I::LocalGet(address), I::LocalGet(count), I::I32Const(8), I::I32Mul, I::I32Add]);
				Self::emit_list(f, &[I::LocalGet(value), I::LocalGet(value), I::I64Const(63), I::I64ShrS, I::I64Xor, I::LocalGet(value), I::I64Const(63), I::I64ShrS, I::I64Sub, I::I64Store(I64)]);
				Self::emit_list(f, &[I::LocalGet(count), I::I32Const(1), I::I32Add, I::LocalSet(count)]);
			});
			f.instruction(&I::LocalGet(count));
		});
	}
}
