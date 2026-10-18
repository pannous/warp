//! A list of ints in one call between the host and the module (card g_odW4): an image's pixels crossed item by item,
//! two host calls a pixel, which took most of a gpu_render or paint frame. The host writes or reads the ints in linear
//! memory above the text heap (src/host.rs with_scratch, web/playground/host.js withScratch) and the module walks them.

use super::super::WasmGcEmitter;
use super::list_ops::ItemWalk;
use crate::type_kinds::Kind;
use wasm_encoder::{BlockType, Function, HeapType, Instruction as I, MemArg, RefType, ValType};

/// ints_to_list(address, count) -> list: the `count` u32 at `address` as a square list of Ints (gpu_render's pixels)
pub const INTS_TO_LIST: &str = "ints_to_list";
/// list_to_ints(list, address, room) -> count: the list's first `room` Ints as i64 at `address` (paint's pixels, negatives
/// as their magnitude), -1 when an item is no fixnum Int and the host reads the list itself
pub const LIST_TO_INTS: &str = "list_to_ints";
const U32: MemArg = MemArg { offset: 0, align: 2, memory_index: 0 };
const I64: MemArg = MemArg { offset: 0, align: 3, memory_index: 0 };
const SQUARE_BRACKET: i64 = 1;
const NOT_INTS: i32 = -1;
/// A pixel beyond the machine ints: its magnitude i64::MAX has every color bit set
const SATURATED: i64 = i64::MAX;

impl WasmGcEmitter {
	pub(in crate::wasm_emitter) fn emit_int_lists(&mut self) {
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
		self.exported_function(
			INTS_TO_LIST,
			vec![ValType::I32, ValType::I32],
			vec![ValType::Ref(node_ref)],
			vec![ValType::Ref(nullable)],
			|s, f| {
				let (address, count, rest) = (0, 1, 2);
				Self::emit_list(f, &[I::LocalGet(count), I::I32Eqz, I::If(BlockType::Empty)]);
				s.call(f, "new_empty");
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::Return,
					I::End,
					I::Block(BlockType::Empty),
						I::Loop(BlockType::Empty),
							I::LocalGet(count), I::I32Eqz, I::BrIf(1),
							I::LocalGet(count), I::I32Const(1), I::I32Sub, I::LocalSet(count),
							I::LocalGet(address), I::LocalGet(count), I::I32Const(4), I::I32Mul, I::I32Add,
							I::I32Load(U32), I::I64ExtendI32U,
				]);
				s.call(f, "new_int");
				Self::emit_list(f, &[I::LocalGet(rest), I::I64Const(SQUARE_BRACKET)]);
				s.call(f, "new_list");
				Self::emit_list(f, &[I::LocalSet(rest), I::Br(0), I::End, I::End, I::LocalGet(rest), I::RefAsNonNull]);
			},
		);
	}

	fn emit_list_to_ints(&mut self) {
		let nullable = self.node_ref(true);
		let (node_type, box_type) = (self.type_manager.node_type, self.type_manager.i64_box_type);
		let (int_array, int_list) = (self.type_manager.int_array_type, self.type_manager.int_list_type);
		let int_list_ref = ValType::Ref(RefType { nullable: true, heap_type: HeapType::Concrete(int_list) });
		let locals = [
			vec![ValType::I32, ValType::Ref(nullable), ValType::I64, ValType::Ref(nullable), int_list_ref],
			self.item_walk_locals().to_vec(),
		]
		.concat();
		self.exported_function(
			LIST_TO_INTS,
			vec![ValType::Ref(RefType::ANYREF), ValType::I32, ValType::I32],
			vec![ValType::I32],
			locals,
			|s, f| {
				let (list, address, room, count, item, value, cell, ints, walk) =
					(0, 1, 2, 3, 4, 5, 6, 7, ItemWalk::at(8));
				let kind_is = |f: &mut Function, local: u32, kind: Kind| {
					s.emit_field(f, local, 0);
					Self::emit_kind_test(f, kind as i64, I::I64Eq);
				};
				// a typed list of ints (draw's canvas_pixels) is copied from its array, no Node built per item
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(list), I::RefTestNonNull(HeapType::Concrete(int_list)), I::If(BlockType::Empty),
						I::LocalGet(list), I::RefCastNonNull(HeapType::Concrete(int_list)), I::LocalSet(ints),
						I::Block(BlockType::Empty),
							I::Loop(BlockType::Empty),
								I::LocalGet(count), I::LocalGet(room), I::I32GeU, I::LocalGet(count), I::LocalGet(ints),
								I::StructGet { struct_type_index: int_list, field_index: 0 }, I::I32GeU, I::I32Or,
								I::BrIf(1),
								I::LocalGet(ints), I::StructGet { struct_type_index: int_list, field_index: 1 },
								I::LocalGet(count), I::ArrayGet(int_array), I::LocalTee(value),
				]);
				// a big Int's handle is no fixnum: its magnitude is past every color, saturated as the host reads a big Number
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::I64Const(super::super::numbers::big_int::FIXNUM_OFFSET), I::I64Add, I::I64Const(0), I::I64LtS,
					I::If(BlockType::Empty),
						I::I64Const(SATURATED), I::LocalSet(value),
					I::End,
				]);
				Self::emit_magnitude_store(f, address, count, value);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::Br(0),
					I::End,
					I::End,
					I::LocalGet(count), I::Return,
					I::End,
					I::LocalGet(list), I::RefCastNullable(HeapType::Concrete(node_type)), I::LocalSet(cell),
				]);
				s.emit_empty_as_null(f, cell);
				Self::emit_list(
					f,
					&[I::LocalGet(cell), I::RefIsNull, I::If(BlockType::Empty), I::I32Const(0), I::Return, I::End],
				);
				kind_is(f, cell, Kind::List);
				Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty), I::I32Const(NOT_INTS), I::Return, I::End]);
				s.emit_item_walk(f, cell, walk, |f| {
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(count), I::LocalGet(room), I::I32GeU, I::BrIf(1), I::LocalGet(walk.item), I::LocalSet(item),
					]);
					// an Int whose payload is an $i64box (not a big Int's handle), else the host reads the list
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(item), I::RefIsNull, I::If(BlockType::Empty),
							I::I32Const(NOT_INTS), I::Return,
						I::End,
					]);
					kind_is(f, item, Kind::Int);
					s.emit_field(f, item, 1);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::RefTestNonNull(HeapType::Concrete(box_type)), I::I32And, I::I32Eqz, I::If(BlockType::Empty),
							I::I32Const(NOT_INTS), I::Return,
						I::End,
					]);
					s.emit_field(f, item, 1);
					Self::emit_list(
						f,
						&[
							I::RefCastNonNull(HeapType::Concrete(box_type)),
							I::StructGet { struct_type_index: box_type, field_index: 0 },
							I::LocalSet(value),
						],
					);
					Self::emit_magnitude_store(f, address, count, value);
				});
				f.instruction(&I::LocalGet(count));
			},
		);
	}

	/// |value| as the i64 at address + 8 × count, then count + 1: |value| = (value ^ sign) - sign
	fn emit_magnitude_store(f: &mut Function, address: u32, count: u32, value: u32) {
		#[rustfmt::skip]
		Self::emit_list(f, &[
			I::LocalGet(address), I::LocalGet(count), I::I32Const(8), I::I32Mul, I::I32Add, I::LocalGet(value),
			I::LocalGet(value), I::I64Const(63), I::I64ShrS, I::I64Xor, I::LocalGet(value), I::I64Const(63), I::I64ShrS,
			I::I64Sub, I::I64Store(I64),
			I::LocalGet(count), I::I32Const(1), I::I32Add, I::LocalSet(count),
		]);
	}
}
