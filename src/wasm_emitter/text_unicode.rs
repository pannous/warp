//! Texts as Unicode: `upper` and `lower` with the case mapping of Rust's `char::to_uppercase` / `to_lowercase` for every
//! code point (user decision #26), `reverse` of a text and `chars` (the list of characters) by code points.
//!
//! The case mapping is a table in linear memory of ranges, sorted by their first code point and binary searched (card
//! web-bundle: a twelfth of a table with an entry per code point, 1.2 KB gzipped for lower): `[start, last, stride,
//! delta, second, third]` maps each code point from start to last a stride apart (1, or 2 for alternating pairs like
//! Āā) to itself plus delta; a mapping to several code points (`ß` is `SS`) is a range of one with the others after it.

use crate::wasm_emitter::WasmGcEmitter;
use wasm_encoder::*;
use Instruction as I;
use Instruction::I32Const;
use ValType::Ref;
use crate::type_kinds::SQUARE_LIST_KIND;
use crate::wasm_emitter::layout::{utf8, BYTE, WORD};

const COPY_BYTES: I<'static> = I::MemoryCopy { src_mem: 0, dst_mem: 0 };

/// The longest mapping of one code point (`ΐ` upper is three)
const MAPPED_SLOTS: u32 = 3;
/// The fields of a range, each a little-endian u32: start, last, stride, delta, then the second and third mapped code points
const START: usize = 0;
const LAST: usize = 1;
const STRIDE: usize = 2;
const DELTA: usize = 3;
const SECOND: usize = 4;
const ENTRY_FIELDS: usize = 6;
const ENTRY_BYTES: u32 = ENTRY_FIELDS as u32 * 4;
/// The strides a range may have: every code point, or every second (a lower and an upper alternating)
const STRIDES: [u32; 2] = [1, 2];
/// A code point takes at most this many bytes more after its mapping than before it (2 bytes → 3 × 2 bytes)
const OUTPUT_FACTOR: i32 = 3;


/// How text_upper, text_lower and text_fold map a code point
#[derive(Clone, Copy)]
pub(super) enum CaseMapping {
	Upper,
	Lower,
	/// lower case without accents: `í` is `i`, `É` is `e` (a letter whose canonical decomposition adds only combining
	/// marks is its first code point; a Hangul syllable stays itself), for `≈`
	Fold,
}

impl CaseMapping {
	fn mapped(self, letter: char) -> Vec<u32> {
		match self {
			CaseMapping::Upper => letter.to_uppercase().map(u32::from).collect(),
			CaseMapping::Lower => letter.to_lowercase().map(u32::from).collect(),
			CaseMapping::Fold => {
				let mut parts = vec![];
				unicode_normalization::char::decompose_canonical(letter, |part| parts.push(part));
				let accented = parts.len() > 1 && parts[1..].iter().all(|&part| unicode_normalization::char::is_combining_mark(part));
				let base = if accented { parts[0] } else { letter };
				base.to_lowercase().map(u32::from).collect()
			}
		}
	}
}

/// case_table, worked out once per process: it walks every code point (100 ms in a debug build)
fn cached_case_table(mapping: CaseMapping) -> &'static [u8] {
	static TABLES: [std::sync::OnceLock<Vec<u8>>; 3] = [const { std::sync::OnceLock::new() }; 3];
	TABLES[mapping as usize].get_or_init(|| case_table(mapping))
}

/// The ranges of code points whose case mapping changes them (module doc), as little-endian u32
fn case_table(mapping: CaseMapping) -> Vec<u8> {
	let mut ranges: Vec<[u32; ENTRY_FIELDS]> = vec![];
	for letter in (0..=char::MAX as u32).filter_map(char::from_u32) {
		let mapped = mapping.mapped(letter);
		let code = u32::from(letter);
		if mapped == [code] {
			continue;
		}
		let delta = mapped[0].wrapping_sub(code);
		if let Some(range) = ranges.last_mut().filter(|range| mapped.len() == 1 && extends(range, code, delta)) {
			// a range of one takes its stride here; a longer one keeps it (extends checked the step)
			range[STRIDE] = code - range[LAST];
			range[LAST] = code;
			continue;
		}
		let more = |slot: usize| mapped.get(slot).copied().unwrap_or(0);
		ranges.push([code, code, 1, delta, more(1), more(2)]);
	}
	ranges.iter().flatten().flat_map(|field| field.to_le_bytes()).collect()
}

/// The instructions loading field `index` of the range at the address in local `entry`
fn range_field(entry: u32, index: usize) -> [I<'static>; 4] {
	[I::LocalGet(entry), I32Const(index as i32 * 4), I::I32Add, I::I32Load(WORD)]
}

/// Whether `code` mapped by `delta` continues `range`: the same delta, one stride after its last code point (a range of
/// one takes the stride from it), and the range maps to one code point
fn extends(range: &[u32; ENTRY_FIELDS], code: u32, delta: u32) -> bool {
	let step = code - range[LAST];
	let stride_fits = if range[LAST] == range[START] { STRIDES.contains(&step) } else { step == range[STRIDE] };
	range[DELTA] == delta && range[SECOND] == 0 && stride_fits
}

impl WasmGcEmitter {
	/// The code point at the byte offset in local `index` of the text bytes, into local `code_point`, and its byte length into `step`
	fn emit_decode_at(&self, func: &mut Function, index: u32, first_byte: u32, code_point: u32, step: u32) {
		let continuation = |f: &mut Function, offset: i32| {
			Self::emit_list(f, &[I::LocalGet(index), I32Const(offset), I::I32Add, I::I32Load8U(BYTE), I32Const(utf8::CONTINUATION_PAYLOAD), I::I32And]);
		};
		Self::emit_list(func, &[I::LocalGet(index), I::I32Load8U(BYTE), I::LocalTee(first_byte), I32Const(utf8::ONE_BYTE_END), I::I32LtU, I::If(BlockType::Empty)]);
		Self::emit_list(func, &[I::LocalGet(first_byte), I::LocalSet(code_point), I32Const(1), I::LocalSet(step), I::Else]);
		Self::emit_list(func, &[I::LocalGet(first_byte), I32Const(utf8::THREE_BYTE_LEAD), I::I32LtU, I::If(BlockType::Empty)]);
		Self::emit_list(func, &[I::LocalGet(first_byte), I32Const(utf8::TWO_BYTE_PAYLOAD), I::I32And, I32Const(utf8::PAYLOAD_BITS), I::I32Shl]);
		continuation(func, 1);
		Self::emit_list(func, &[I::I32Or, I::LocalSet(code_point), I32Const(2), I::LocalSet(step), I::Else]);
		Self::emit_list(func, &[I::LocalGet(first_byte), I32Const(utf8::FOUR_BYTE_LEAD), I::I32LtU, I::If(BlockType::Empty)]);
		Self::emit_list(func, &[I::LocalGet(first_byte), I32Const(utf8::THREE_BYTE_PAYLOAD), I::I32And, I32Const(2 * utf8::PAYLOAD_BITS), I::I32Shl]);
		continuation(func, 1);
		Self::emit_list(func, &[I32Const(utf8::PAYLOAD_BITS), I::I32Shl, I::I32Or]);
		continuation(func, 2);
		Self::emit_list(func, &[I::I32Or, I::LocalSet(code_point), I32Const(3), I::LocalSet(step), I::Else]);
		Self::emit_list(func, &[I::LocalGet(first_byte), I32Const(utf8::FOUR_BYTE_PAYLOAD), I::I32And, I32Const(3 * utf8::PAYLOAD_BITS), I::I32Shl]);
		continuation(func, 1);
		Self::emit_list(func, &[I32Const(2 * utf8::PAYLOAD_BITS), I::I32Shl, I::I32Or]);
		continuation(func, 2);
		Self::emit_list(func, &[I32Const(utf8::PAYLOAD_BITS), I::I32Shl, I::I32Or]);
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
		Self::emit_list(func, &[I::LocalGet(code_point), I32Const(utf8::ONE_BYTE_END), I::I32LtU, I::If(BlockType::Empty)]);
		store(func, 0, 0, 0, utf8::ONE_BYTE_PAYLOAD);
		advance(func, 1);
		Self::emit_list(func, &[I::Else, I::LocalGet(code_point), I32Const(utf8::TWO_BYTE_END), I::I32LtU, I::If(BlockType::Empty)]);
		store(func, 0, utf8::TWO_BYTE_LEAD, utf8::PAYLOAD_BITS, utf8::TWO_BYTE_PAYLOAD);
		store(func, 1, utf8::CONTINUATION_MARK, 0, utf8::CONTINUATION_PAYLOAD);
		advance(func, 2);
		Self::emit_list(func, &[I::Else, I::LocalGet(code_point), I32Const(utf8::THREE_BYTE_END), I::I32LtU, I::If(BlockType::Empty)]);
		store(func, 0, utf8::THREE_BYTE_LEAD, 2 * utf8::PAYLOAD_BITS, utf8::THREE_BYTE_PAYLOAD);
		store(func, 1, utf8::CONTINUATION_MARK, utf8::PAYLOAD_BITS, utf8::CONTINUATION_PAYLOAD);
		store(func, 2, utf8::CONTINUATION_MARK, 0, utf8::CONTINUATION_PAYLOAD);
		advance(func, 3);
		Self::emit_list(func, &[I::Else]);
		store(func, 0, utf8::FOUR_BYTE_LEAD, 3 * utf8::PAYLOAD_BITS, utf8::FOUR_BYTE_PAYLOAD);
		store(func, 1, utf8::CONTINUATION_MARK, 2 * utf8::PAYLOAD_BITS, utf8::CONTINUATION_PAYLOAD);
		store(func, 2, utf8::CONTINUATION_MARK, utf8::PAYLOAD_BITS, utf8::CONTINUATION_PAYLOAD);
		store(func, 3, utf8::CONTINUATION_MARK, 0, utf8::CONTINUATION_PAYLOAD);
		advance(func, 4);
		Self::emit_list(func, &[I::End, I::End, I::End]);
	}

	/// text_upper(text) / text_lower(text) / text_fold(text): a fresh text with every code point mapped
	pub(super) fn emit_text_case(&mut self, name: &'static str, mapping: CaseMapping) {
		if !self.should_emit_function(name) {
			return;
		}
		self.emit_text_heap_global();
		let case_table = cached_case_table(mapping);
		let entries = (case_table.len() as u32 / ENTRY_BYTES) as i32;
		let table = self.allocate_bytes(name, case_table) as i32;
		let node_ref = Ref(self.node_ref(false));
		self.runtime_function(name, vec![node_ref], vec![node_ref], vec![ValType::I32; 15], |s, f| {
			let (pointer, end, capacity, destination, index, out) = (1, 2, 3, 4, 5, 6);
			let (first_byte, code_point, step, slot, mapped, entry) = (7, 8, 9, 10, 11, 12);
			let (low, high, middle) = (13, 14, 15);
			s.emit_text_argument_bounds(f, pointer, end);
			Self::emit_list(f, &[I::LocalGet(end), I::LocalGet(pointer), I::I32Sub, I32Const(OUTPUT_FACTOR), I::I32Mul, I::LocalSet(capacity)]);
			s.emit_text_allocation(f, capacity, destination);
			Self::emit_list(f, &[I::LocalGet(pointer), I::LocalSet(index), I::LocalGet(destination), I::LocalSet(out)]);
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(index), I::LocalGet(end), I::I32GeU, I::BrIf(1)]);
			s.emit_decode_at(f, index, first_byte, code_point, step);
			Self::emit_list(f, &[I::LocalGet(index), I::LocalGet(step), I::I32Add, I::LocalSet(index)]);
			// binary search for the last range starting at or before the code point
			Self::emit_list(f, &[I32Const(0), I::LocalSet(low), I32Const(entries), I::LocalSet(high)]);
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(low), I::LocalGet(high), I::I32GeU, I::BrIf(1)]);
			Self::emit_list(f, &[I::LocalGet(low), I::LocalGet(high), I::I32Add, I32Const(1), I::I32ShrU, I::LocalTee(middle)]);
			Self::emit_list(f, &[I32Const(ENTRY_BYTES as i32), I::I32Mul, I32Const(table), I::I32Add, I::LocalSet(entry)]);
			Self::emit_list(f, &range_field(entry, START));
			Self::emit_list(f, &[I::LocalGet(code_point), I::I32LeU]);
			Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(middle), I32Const(1), I::I32Add, I::LocalSet(low), I::Else, I::LocalGet(middle), I::LocalSet(high), I::End]);
			Self::emit_list(f, &[I::Br(0), I::End, I::End]);
			// entry: the address of that range when it maps the code point (up to its last, a stride apart), else -1
			Self::emit_list(f, &[I32Const(-1), I::LocalSet(mapped), I::LocalGet(low), I::If(BlockType::Empty)]);
			Self::emit_list(f, &[I::LocalGet(low), I32Const(1), I::I32Sub, I32Const(ENTRY_BYTES as i32), I::I32Mul, I32Const(table), I::I32Add, I::LocalSet(entry)]);
			Self::emit_list(f, &[I::LocalGet(code_point)]);
			Self::emit_list(f, &range_field(entry, LAST));
			Self::emit_list(f, &[I::I32LeU, I::LocalGet(code_point)]);
			Self::emit_list(f, &range_field(entry, START));
			Self::emit_list(f, &[I::I32Sub]);
			Self::emit_list(f, &range_field(entry, STRIDE));
			Self::emit_list(f, &[I32Const(1), I::I32Sub, I::I32And, I::I32Eqz, I::I32And, I::If(BlockType::Empty), I::LocalGet(entry), I::LocalSet(mapped), I::End, I::End]);
			Self::emit_list(f, &[I::LocalGet(mapped), I::LocalSet(entry)]);
			// write the mapped code points (the first is the code point plus delta), or the code point itself without a range
			Self::emit_list(f, &[I32Const(0), I::LocalSet(slot), I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
			Self::emit_list(f, &[I::LocalGet(entry), I32Const(0), I::I32GeS, I::If(BlockType::Result(ValType::I32))]);
			Self::emit_list(f, &[I::LocalGet(entry), I::LocalGet(slot), I32Const(DELTA as i32), I::I32Add, I32Const(4), I::I32Mul, I::I32Add, I::I32Load(WORD)]);
			Self::emit_list(f, &[I::LocalGet(code_point), I32Const(0), I::LocalGet(slot), I::I32Eqz, I::Select, I::I32Add]);
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

	/// A character argument (local 0) becomes a text, anything else but a text fails; `pointer` and `end` span its bytes
	fn emit_text_argument_bounds(&self, func: &mut Function, pointer: u32, end: u32) {
		self.emit_codepoint_as_text(func, 0);
		self.emit_is_text(func);
		func.instruction(&I::I32Eqz);
		self.emit_fail_if(func, "not_a_text");
		self.emit_text_bounds(func, pointer, end);
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
			s.emit_text_argument_bounds(f, pointer, end);
			Self::emit_list(f, &[I::LocalGet(end), I::LocalGet(pointer), I::I32Sub, I::LocalSet(scan)]);
			s.emit_text_allocation(f, scan, destination);
			Self::emit_list(f, &[I::LocalGet(destination), I::LocalSet(out), I::LocalGet(end), I::LocalSet(scan)]);
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(scan), I::LocalGet(pointer), I::I32LeU, I::BrIf(1)]);
			// the code point ending at `scan` starts at the last byte that is no continuation byte
			Self::emit_list(f, &[I::LocalGet(scan), I32Const(1), I::I32Sub, I::LocalSet(start)]);
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(start), I::I32Load8U(BYTE), I32Const(utf8::CONTINUATION_MASK), I::I32And]);
			Self::emit_list(f, &[I32Const(utf8::CONTINUATION_MARK), I::I32Ne, I::BrIf(1), I::LocalGet(start), I32Const(1), I::I32Sub, I::LocalSet(start), I::Br(0), I::End, I::End]);
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
		let mut locals = vec![nullable];
		locals.extend([ValType::I32; 6]);
		self.runtime_function("text_chars", vec![node_ref], vec![node_ref], locals, |s, f| {
			let (characters, pointer, end, index, first_byte, code_point, step) = (1, 2, 3, 4, 5, 6, 7);
			s.emit_text_argument_bounds(f, pointer, end);
			Self::emit_list(f, &[I::LocalGet(pointer), I::LocalSet(index)]);
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(index), I::LocalGet(end), I::I32GeU, I::BrIf(1)]);
			s.emit_decode_at(f, index, first_byte, code_point, step);
			Self::emit_list(f, &[I::LocalGet(index), I::LocalGet(step), I::I32Add, I::LocalSet(index)]);
			Self::emit_list(f, &[I::I64Const(SQUARE_LIST_KIND), I::LocalGet(code_point)]);
			s.call(f, "new_codepoint");
			Self::emit_list(f, &[I::LocalGet(characters), I::StructNew(node_type), I::LocalSet(characters), I::Br(0), I::End, I::End]);
			Self::emit_list(f, &[I::LocalGet(characters)]);
			s.call(f, "list_reverse");
		});
	}
}
