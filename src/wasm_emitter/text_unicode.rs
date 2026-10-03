//! Texts as Unicode: `upper` and `lower` with the case mapping of Rust's `char::to_uppercase` / `to_lowercase` for every
//! code point (user decision #26), `reverse` of a text and `chars` (the list of characters) by code points.
//!
//! The case mapping is a table in linear memory with one entry per code point that changes, sorted by code point and
//! binary searched: the code point, then up to three mapped code points (`ß` is `SS`), zeros for unused slots.

use crate::type_kinds::Kind;
use crate::wasm_emitter::WasmGcEmitter;
use wasm_encoder::*;
use Instruction as I;
use Instruction::I32Const;
use ValType::Ref;

const BYTE: MemArg = MemArg { offset: 0, align: 0, memory_index: 0 };
const WORD: MemArg = MemArg { offset: 0, align: 2, memory_index: 0 };
const COPY_BYTES: I<'static> = I::MemoryCopy { src_mem: 0, dst_mem: 0 };
const KIND_MASK: i64 = 0xFF;
const SQUARE_BRACKET_INFO: i64 = 1;
const KIND_SHIFT: i64 = 8;

/// The longest mapping of one code point (`ΐ` upper is three)
const MAPPED_SLOTS: u32 = 3;
const ENTRY_BYTES: u32 = (1 + MAPPED_SLOTS) * 4;
/// A code point takes at most this many bytes more after its mapping than before it (2 bytes → 3 × 2 bytes)
const OUTPUT_FACTOR: i32 = 3;

const CONTINUATION_MASK: i32 = 0xC0;
const CONTINUATION_MARK: i32 = 0x80;
const PAYLOAD_MASK: i32 = 0x3F;

/// The code points whose case mapping changes them, each as little-endian u32: the code point, then `MAPPED_SLOTS` mapped ones
fn case_table(is_upper: bool) -> Vec<u8> {
	let mut table = vec![];
	for letter in (0..=char::MAX as u32).filter_map(char::from_u32) {
		let mapped: Vec<u32> = if is_upper { letter.to_uppercase().map(u32::from).collect() } else { letter.to_lowercase().map(u32::from).collect() };
		if mapped == [u32::from(letter)] {
			continue;
		}
		table.extend(u32::from(letter).to_le_bytes());
		for slot in 0..MAPPED_SLOTS as usize {
			table.extend(mapped.get(slot).copied().unwrap_or(0).to_le_bytes());
		}
	}
	table
}

impl WasmGcEmitter {
	/// The code point at the byte offset in local `index` of the text bytes, into local `code_point`, and its byte length into `step`
	fn emit_decode_at(&self, func: &mut Function, index: u32, first_byte: u32, code_point: u32, step: u32) {
		let continuation = |f: &mut Function, offset: i32| {
			Self::emit_list(f, &[I::LocalGet(index), I32Const(offset), I::I32Add, I::I32Load8U(BYTE), I32Const(PAYLOAD_MASK), I::I32And]);
		};
		Self::emit_list(func, &[I::LocalGet(index), I::I32Load8U(BYTE), I::LocalTee(first_byte), I32Const(0x80), I::I32LtU, I::If(BlockType::Empty)]);
		Self::emit_list(func, &[I::LocalGet(first_byte), I::LocalSet(code_point), I32Const(1), I::LocalSet(step), I::Else]);
		Self::emit_list(func, &[I::LocalGet(first_byte), I32Const(0xE0), I::I32LtU, I::If(BlockType::Empty)]);
		Self::emit_list(func, &[I::LocalGet(first_byte), I32Const(0x1F), I::I32And, I32Const(6), I::I32Shl]);
		continuation(func, 1);
		Self::emit_list(func, &[I::I32Or, I::LocalSet(code_point), I32Const(2), I::LocalSet(step), I::Else]);
		Self::emit_list(func, &[I::LocalGet(first_byte), I32Const(0xF0), I::I32LtU, I::If(BlockType::Empty)]);
		Self::emit_list(func, &[I::LocalGet(first_byte), I32Const(0x0F), I::I32And, I32Const(12), I::I32Shl]);
		continuation(func, 1);
		Self::emit_list(func, &[I32Const(6), I::I32Shl, I::I32Or]);
		continuation(func, 2);
		Self::emit_list(func, &[I::I32Or, I::LocalSet(code_point), I32Const(3), I::LocalSet(step), I::Else]);
		Self::emit_list(func, &[I::LocalGet(first_byte), I32Const(0x07), I::I32And, I32Const(18), I::I32Shl]);
		continuation(func, 1);
		Self::emit_list(func, &[I32Const(12), I::I32Shl, I::I32Or]);
		continuation(func, 2);
		Self::emit_list(func, &[I32Const(6), I::I32Shl, I::I32Or]);
		continuation(func, 3);
		Self::emit_list(func, &[I::I32Or, I::LocalSet(code_point), I32Const(4), I::LocalSet(step), I::End, I::End, I::End]);
	}

	/// Write the code point in local `code_point` as UTF-8 at local `out` and advance `out`
	fn emit_encode_at(&self, func: &mut Function, out: u32, code_point: u32) {
		let store = |f: &mut Function, offset: i32, lead: i32, shift: i32, mask: i32| {
			Self::emit_list(f, &[I::LocalGet(out), I32Const(offset), I::I32Add, I::LocalGet(code_point), I32Const(shift), I::I32ShrU]);
			Self::emit_list(f, &[I32Const(mask), I::I32And, I32Const(lead), I::I32Or, I::I32Store8(BYTE)]);
		};
		let advance = |f: &mut Function, bytes: i32| {
			Self::emit_list(f, &[I::LocalGet(out), I32Const(bytes), I::I32Add, I::LocalSet(out)]);
		};
		Self::emit_list(func, &[I::LocalGet(code_point), I32Const(0x80), I::I32LtU, I::If(BlockType::Empty)]);
		store(func, 0, 0, 0, 0x7F);
		advance(func, 1);
		Self::emit_list(func, &[I::Else, I::LocalGet(code_point), I32Const(0x800), I::I32LtU, I::If(BlockType::Empty)]);
		store(func, 0, 0xC0, 6, 0x1F);
		store(func, 1, 0x80, 0, PAYLOAD_MASK);
		advance(func, 2);
		Self::emit_list(func, &[I::Else, I::LocalGet(code_point), I32Const(0x10000), I::I32LtU, I::If(BlockType::Empty)]);
		store(func, 0, 0xE0, 12, 0x0F);
		store(func, 1, 0x80, 6, PAYLOAD_MASK);
		store(func, 2, 0x80, 0, PAYLOAD_MASK);
		advance(func, 3);
		Self::emit_list(func, &[I::Else]);
		store(func, 0, 0xF0, 18, 0x07);
		store(func, 1, 0x80, 12, PAYLOAD_MASK);
		store(func, 2, 0x80, 6, PAYLOAD_MASK);
		store(func, 3, 0x80, 0, PAYLOAD_MASK);
		advance(func, 4);
		Self::emit_list(func, &[I::End, I::End, I::End]);
	}

	/// text_upper(text) / text_lower(text): a fresh text with every code point mapped
	pub(super) fn emit_text_case(&mut self, name: &'static str, is_upper: bool) {
		if !self.should_emit_function(name) {
			return;
		}
		self.emit_text_heap_global();
		let case_table = case_table(is_upper);
		let entries = (case_table.len() as u32 / ENTRY_BYTES) as i32;
		let table = self.allocate_bytes(name, &case_table) as i32;
		let node_ref = Ref(self.node_ref(false));
		self.runtime_function(name, vec![node_ref], vec![node_ref], vec![ValType::I32; 15], |s, f| {
			let (pointer, end, capacity, destination, index, out) = (1, 2, 3, 4, 5, 6);
			let (first_byte, code_point, step, slot, mapped, entry) = (7, 8, 9, 10, 11, 12);
			let (low, high, middle) = (13, 14, 15);
			s.emit_codepoint_as_text(f, 0);
			s.emit_is_text(f);
			f.instruction(&I::I32Eqz);
			s.emit_fail_if(f, "not_a_text");
			s.emit_text_bounds(f, pointer, end);
			Self::emit_list(f, &[I::LocalGet(end), I::LocalGet(pointer), I::I32Sub, I32Const(OUTPUT_FACTOR), I::I32Mul, I::LocalSet(capacity)]);
			s.emit_text_allocation(f, capacity, destination);
			Self::emit_list(f, &[I::LocalGet(pointer), I::LocalSet(index), I::LocalGet(destination), I::LocalSet(out)]);
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(index), I::LocalGet(end), I::I32GeU, I::BrIf(1)]);
			s.emit_decode_at(f, index, first_byte, code_point, step);
			Self::emit_list(f, &[I::LocalGet(index), I::LocalGet(step), I::I32Add, I::LocalSet(index)]);
			// binary search for the entry of the code point, -1 when it does not change
			Self::emit_list(f, &[I32Const(0), I::LocalSet(low), I32Const(entries), I::LocalSet(high)]);
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(low), I::LocalGet(high), I::I32GeU, I::BrIf(1)]);
			Self::emit_list(f, &[I::LocalGet(low), I::LocalGet(high), I::I32Add, I32Const(1), I::I32ShrU, I::LocalTee(middle)]);
			Self::emit_list(f, &[I32Const(ENTRY_BYTES as i32), I::I32Mul, I32Const(table), I::I32Add, I::I32Load(WORD), I::LocalGet(code_point), I::I32LtU]);
			Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(middle), I32Const(1), I::I32Add, I::LocalSet(low), I::Else, I::LocalGet(middle), I::LocalSet(high), I::End]);
			Self::emit_list(f, &[I::Br(0), I::End, I::End]);
			Self::emit_list(f, &[I32Const(-1), I::LocalSet(entry), I::LocalGet(low), I32Const(entries), I::I32LtU, I::If(BlockType::Empty)]);
			Self::emit_list(f, &[I::LocalGet(low), I32Const(ENTRY_BYTES as i32), I::I32Mul, I32Const(table), I::I32Add, I::I32Load(WORD), I::LocalGet(code_point), I::I32Eq]);
			Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(low), I::LocalSet(entry), I::End, I::End]);
			// write the mapped code points, or the code point itself when the entry is empty or missing
			Self::emit_list(f, &[I32Const(0), I::LocalSet(slot), I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
			Self::emit_list(f, &[I::LocalGet(entry), I32Const(0), I::I32GeS, I::If(BlockType::Result(ValType::I32))]);
			Self::emit_list(f, &[I::LocalGet(entry), I32Const(ENTRY_BYTES as i32), I::I32Mul, I::LocalGet(slot), I32Const(1), I::I32Add, I32Const(4), I::I32Mul, I::I32Add, I32Const(table), I::I32Add, I::I32Load(WORD)]);
			Self::emit_list(f, &[I::Else, I32Const(0), I::End, I::LocalSet(mapped)]);
			Self::emit_list(f, &[I::LocalGet(mapped), I::I32Eqz, I::If(BlockType::Empty), I::LocalGet(slot), I::BrIf(2)]);
			Self::emit_list(f, &[I::LocalGet(code_point), I::LocalSet(mapped), I::End]);
			s.emit_encode_at(f, out, mapped);
			// only the first slot of an unchanged code point is read: its mapped value stays 0 and slot 1 ends the loop
			Self::emit_list(f, &[I::LocalGet(slot), I32Const(1), I::I32Add, I::LocalTee(slot), I32Const(MAPPED_SLOTS as i32), I::I32LtU, I::BrIf(0), I::End, I::End]);
			Self::emit_list(f, &[I::Br(0), I::End, I::End]);
			Self::emit_list(f, &[I::LocalGet(destination), I::LocalGet(out), I::LocalGet(destination), I::I32Sub]);
			s.call(f, "new_text");
		});
	}

	/// text_reverse(text): a fresh text with the code points in the opposite order
	pub(super) fn emit_text_reverse(&mut self) {
		// list_reverse, which text_split also uses, hands a text on to it
		if !["list_reverse", "text_reverse", "text_split"].iter().any(|name| self.should_emit_function(name)) {
			return;
		}
		self.emit_text_heap_global();
		let node_ref = Ref(self.node_ref(false));
		self.runtime_function("text_reverse", vec![node_ref], vec![node_ref], vec![ValType::I32; 6], |s, f| {
			let (pointer, end, destination, out, scan, start) = (1, 2, 3, 4, 5, 6);
			s.emit_codepoint_as_text(f, 0);
			s.emit_is_text(f);
			f.instruction(&I::I32Eqz);
			s.emit_fail_if(f, "not_a_text");
			s.emit_text_bounds(f, pointer, end);
			Self::emit_list(f, &[I::LocalGet(end), I::LocalGet(pointer), I::I32Sub, I::LocalSet(scan)]);
			s.emit_text_allocation(f, scan, destination);
			Self::emit_list(f, &[I::LocalGet(destination), I::LocalSet(out), I::LocalGet(end), I::LocalSet(scan)]);
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(scan), I::LocalGet(pointer), I::I32LeU, I::BrIf(1)]);
			// the code point ending at `scan` starts at the last byte that is no continuation byte
			Self::emit_list(f, &[I::LocalGet(scan), I32Const(1), I::I32Sub, I::LocalSet(start)]);
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(start), I::I32Load8U(BYTE), I32Const(CONTINUATION_MASK), I::I32And]);
			Self::emit_list(f, &[I32Const(CONTINUATION_MARK), I::I32Ne, I::BrIf(1), I::LocalGet(start), I32Const(1), I::I32Sub, I::LocalSet(start), I::Br(0), I::End, I::End]);
			Self::emit_list(f, &[I::LocalGet(out), I::LocalGet(start), I::LocalGet(scan), I::LocalGet(start), I::I32Sub, COPY_BYTES]);
			Self::emit_list(f, &[I::LocalGet(out), I::LocalGet(scan), I::LocalGet(start), I::I32Sub, I::I32Add, I::LocalSet(out), I::LocalGet(start), I::LocalSet(scan), I::Br(0), I::End, I::End]);
			Self::emit_list(f, &[I::LocalGet(destination), I::LocalGet(out), I::LocalGet(destination), I::I32Sub]);
			s.call(f, "new_text");
		});
	}

	/// text_chars(text): the list of its code points as characters
	pub(super) fn emit_text_chars(&mut self) {
		if !self.should_emit_function("text_chars") {
			return;
		}
		let (node_ref, nullable) = (Ref(self.node_ref(false)), Ref(self.node_ref(true)));
		let node_type = self.type_manager.node_type;
		let square_list = (SQUARE_BRACKET_INFO << KIND_SHIFT) | Kind::List as i64;
		let mut locals = vec![nullable];
		locals.extend([ValType::I32; 6]);
		self.runtime_function("text_chars", vec![node_ref], vec![node_ref], locals, |s, f| {
			let (characters, pointer, end, index, first_byte, code_point, step) = (1, 2, 3, 4, 5, 6, 7);
			s.emit_codepoint_as_text(f, 0);
			s.emit_is_text(f);
			f.instruction(&I::I32Eqz);
			s.emit_fail_if(f, "not_a_text");
			s.emit_text_bounds(f, pointer, end);
			Self::emit_list(f, &[I::LocalGet(pointer), I::LocalSet(index)]);
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(index), I::LocalGet(end), I::I32GeU, I::BrIf(1)]);
			s.emit_decode_at(f, index, first_byte, code_point, step);
			Self::emit_list(f, &[I::LocalGet(index), I::LocalGet(step), I::I32Add, I::LocalSet(index)]);
			Self::emit_list(f, &[I::I64Const(square_list), I::LocalGet(code_point)]);
			s.call(f, "new_codepoint");
			Self::emit_list(f, &[I::LocalGet(characters), I::StructNew(node_type), I::LocalSet(characters), I::Br(0), I::End, I::End]);
			Self::emit_list(f, &[I::LocalGet(characters)]);
			s.call(f, "list_reverse");
		});
	}
}
