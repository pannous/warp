//! Texts as Unicode: `upper` and `lower` with the case mapping of Rust's `char::to_uppercase` / `to_lowercase` for every
//! code point (user decision #26), `reverse` of a text by code points, and `chars` (the list of characters, a grapheme
//! cluster each) with its twin by code points.
//!
//! The case mapping is a table in linear memory of ranges, sorted by their first code point and binary searched (card
//! web-bundle: a twelfth of a table with an entry per code point, 1.2 KB gzipped for lower): `[start, last, stride,
//! delta, second, third]` maps each code point from start to last a stride apart (1, or 2 for alternating pairs like
//! Āā) to itself plus delta; a mapping to several code points (`ß` is `SS`) is a range of one with the others after it.

use crate::type_kinds::{Kind, KIND_MASK, SQUARE_LIST_KIND};
use crate::wasm_emitter::layout::WORD;
use crate::wasm_emitter::WasmGcEmitter;
use wasm_encoder::*;
use Instruction as I;
use Instruction::I32Const;
use ValType::Ref;

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
pub(in crate::wasm_emitter) enum CaseMapping {
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
				let accented = parts.len() > 1
					&& parts[1..].iter().all(|&part| unicode_normalization::char::is_combining_mark(part));
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
	/// text_upper(text) / text_lower(text) / text_fold(text): a fresh text with every code point mapped
	pub(in crate::wasm_emitter) fn emit_text_case(&mut self, name: &'static str, mapping: CaseMapping) {
		if !self.should_emit_function(name) {
			return;
		}
		let case_table = cached_case_table(mapping);
		let entries = (case_table.len() as u32 / ENTRY_BYTES) as i32;
		let table = self.allocate_bytes(name, case_table) as i32;
		let (node_ref, text) = (Ref(self.node_ref(false)), self.text_ref());
		let text_type = self.type_manager.text_type;
		let mut locals = vec![ValType::I32; 16];
		locals[3] = text;
		locals.push(text);
		self.runtime_function(name, vec![node_ref], vec![node_ref], locals, |s, f| {
			let (pointer, end, capacity, destination, index, out) = (1, 2, 3, 4, 5, 6);
			let (width, code_point, lead, slot, mapped, entry) = (7, 8, 9, 10, 11, 12);
			let (low, high, middle, next, bytes) = (13, 14, 15, 16, 17);
			s.emit_text_argument_bounds(f, (bytes, pointer, end));
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(end), I32Const(OUTPUT_FACTOR), I::I32Mul, I::LocalTee(capacity),
				I::ArrayNewDefault(text_type), I::LocalSet(destination),
				I::LocalGet(pointer), I::LocalSet(index),
				I32Const(0), I::LocalSet(out),
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(index), I::LocalGet(end), I::I32GeU, I::BrIf(1),
			]);
			s.emit_next_code_point(f, (bytes, index, end), code_point);
			// binary search for the last range starting at or before the code point
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I32Const(0), I::LocalSet(low),
				I32Const(entries), I::LocalSet(high),
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(low), I::LocalGet(high), I::I32GeU, I::BrIf(1),
						I::LocalGet(low), I::LocalGet(high), I::I32Add, I32Const(1), I::I32ShrU, I::LocalTee(middle),
						I32Const(ENTRY_BYTES as i32), I::I32Mul, I32Const(table), I::I32Add, I::LocalSet(entry),
			]);
			Self::emit_list(f, &range_field(entry, START));
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(code_point), I::I32LeU, I::If(BlockType::Empty),
					I::LocalGet(middle), I32Const(1), I::I32Add, I::LocalSet(low),
				I::Else,
					I::LocalGet(middle), I::LocalSet(high),
				I::End,
				I::Br(0),
				I::End,
				I::End,
			]);
			// entry: the address of that range when it maps the code point (up to its last, a stride apart), else -1
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I32Const(-1), I::LocalSet(mapped),
				I::LocalGet(low), I::If(BlockType::Empty),
					I::LocalGet(low), I32Const(1), I::I32Sub, I32Const(ENTRY_BYTES as i32), I::I32Mul, I32Const(table),
					I::I32Add, I::LocalSet(entry),
					I::LocalGet(code_point),
			]);
			Self::emit_list(f, &range_field(entry, LAST));
			Self::emit_list(f, &[I::I32LeU, I::LocalGet(code_point)]);
			Self::emit_list(f, &range_field(entry, START));
			Self::emit_list(f, &[I::I32Sub]);
			Self::emit_list(f, &range_field(entry, STRIDE));
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I32Const(1), I::I32Sub, I::I32And, I::I32Eqz, I::I32And, I::If(BlockType::Empty),
					I::LocalGet(entry), I::LocalSet(mapped),
				I::End,
				I::End,
				I::LocalGet(mapped), I::LocalSet(entry),
			]);
			// write the mapped code points (the first is the code point plus delta), or the code point itself without a range
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I32Const(0), I::LocalSet(slot),
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(entry), I32Const(0), I::I32GeS, I::If(BlockType::Result(ValType::I32)),
							I::LocalGet(entry), I::LocalGet(slot), I32Const(DELTA as i32), I::I32Add, I32Const(4),
							I::I32Mul, I::I32Add, I::I32Load(WORD), I::LocalGet(code_point), I32Const(0),
							I::LocalGet(slot), I::I32Eqz, I::Select, I::I32Add,
						I::Else,
							I32Const(0),
						I::End,
						I::LocalSet(mapped),
						I::LocalGet(mapped), I::I32Eqz, I::If(BlockType::Empty),
							I::LocalGet(slot), I::BrIf(2),
							I::LocalGet(code_point), I::LocalSet(mapped),
						I::End,
			]);
			Self::emit_utf8_width(f, mapped);
			Self::emit_list(f, &[I::LocalTee(width), I::LocalGet(out), I::I32Add, I::LocalSet(next)]);
			s.emit_utf8_store(f, (destination, out), mapped, width, lead);
			Self::emit_list(f, &[I::LocalGet(next), I::LocalSet(out)]);
			// only the first slot of an unchanged code point is read: its mapped value stays 0 and slot 1 ends the loop
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(slot), I32Const(1), I::I32Add, I::LocalTee(slot), I32Const(MAPPED_SLOTS as i32), I::I32LtU,
				I::BrIf(0),
				I::End,
				I::End,
				I::Br(0),
				I::End,
				I::End,
			]);
			// the bytes written, from index 0 (capacity is free again)
			Self::emit_list(f, &[I32Const(0), I::LocalSet(capacity)]);
			s.emit_text_piece(f, (destination, capacity, out));
		});
	}

	/// A character argument (local 0) becomes a text, anything else but a text fails; local `bytes` holds its bytes,
	/// `pointer` and `end` span them
	fn emit_text_argument_bounds(&self, func: &mut Function, bounds: (u32, u32, u32)) {
		self.emit_codepoint_as_text(func, 0);
		self.emit_is_text(func);
		func.instruction(&I::I32Eqz);
		self.emit_fail_if(func, "not_a_text");
		self.emit_text_bounds(func, bounds);
	}

	/// text_reverse(text): a fresh text with the characters in the opposite order, each grapheme cluster kept whole
	/// (`reverse "👍🏽ab"` is "ba👍🏽", card reverse-characters): a cluster at bytes [at, past) lands at end - past
	pub(in crate::wasm_emitter) fn emit_text_reverse(&mut self) {
		// list_reverse, which text_split also uses, hands a text on to it
		if !["list_reverse", "text_reverse", "text_split"].iter().any(|name| self.should_emit_function(name)) {
			return;
		}
		let (node_ref, text) = (Ref(self.node_ref(false)), self.text_ref());
		let text_type = self.type_manager.text_type;
		let locals = vec![ValType::I32, ValType::I32, text, ValType::I32, ValType::I32, text];
		self.runtime_function("text_reverse", vec![node_ref], vec![node_ref], locals, |s, f| {
			let (pointer, end, destination, at, past, bytes) = (1, 2, 3, 4, 5, 6);
			s.emit_text_argument_bounds(f, (bytes, pointer, end));
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(end), I::ArrayNewDefault(text_type), I::LocalSet(destination),
				I::LocalGet(pointer), I::LocalSet(at),
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(at), I::LocalGet(end), I::I32GeU, I::BrIf(1),
						I::LocalGet(bytes), I::LocalGet(at), I::LocalGet(end),
			]);
			s.call(f, "grapheme_end");
			#[rustfmt::skip]
			Self::emit_list(f, &[
						I::LocalSet(past),
						I::LocalGet(destination), I::LocalGet(end), I::LocalGet(past), I::I32Sub,
						I::LocalGet(bytes), I::LocalGet(at), I::LocalGet(past), I::LocalGet(at), I::I32Sub,
						I::ArrayCopy { array_type_index_dst: text_type, array_type_index_src: text_type },
						I::LocalGet(past), I::LocalSet(at),
						I::Br(0),
					I::End,
				I::End,
				I::LocalGet(destination),
			]);
			s.call(f, crate::wasm_emitter::NEW_TEXT_OF);
		});
	}

	/// text_chars(text): the list of its code points as characters; text_graphemes(text): of its characters, a
	/// grapheme cluster each (`"שָלוֹם".chars`, decision chars-codepoints amended). A list comes back as it is, so that
	/// `x as list` of a value whose static type is unknown calls them (casts.rs)
	pub(in crate::wasm_emitter) fn emit_text_items(&mut self) {
		self.emit_text_item_list(crate::library_words::TEXT_CODE_POINTS, false);
		self.emit_text_item_list(crate::library_words::TEXT_GRAPHEMES, true);
	}

	fn emit_text_item_list(&mut self, name: &'static str, by_grapheme: bool) {
		if !self.should_emit_function(name) {
			return;
		}
		let (node_ref, nullable) = (Ref(self.node_ref(false)), Ref(self.node_ref(true)));
		let node_type = self.type_manager.node_type;
		let mut locals = vec![nullable];
		locals.extend([ValType::I32; 3]);
		locals.push(self.text_ref());
		self.runtime_function(name, vec![node_ref], vec![node_ref], locals, |s, f| {
			// code_point holds the grapheme's end when walking by grapheme
			let (characters, pointer, end, code_point, bytes) = (1, 2, 3, 4, 5);
			s.emit_field(f, 0, 0);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::List as i64), I::I64Eq,
				I::If(BlockType::Empty), I::LocalGet(0), I::Return, I::End,
			]);
			s.emit_text_argument_bounds(f, (bytes, pointer, end));
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(pointer), I::LocalGet(end), I::I32GeU, I::BrIf(1),
			]);
			f.instruction(&I::I64Const(SQUARE_LIST_KIND));
			if by_grapheme {
				s.emit_grapheme_node(f, (bytes, pointer, end), code_point);
				Self::emit_list(f, &[I::LocalGet(code_point), I::LocalSet(pointer)]);
			} else {
				s.emit_next_code_point(f, (bytes, pointer, end), code_point);
				f.instruction(&I::LocalGet(code_point));
				s.call(f, "new_codepoint");
			}
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(characters), I::StructNew(node_type), I::LocalSet(characters),
				I::Br(0),
				I::End,
				I::End,
				I::LocalGet(characters),
			]);
			s.call(f, "list_reverse");
		});
	}
}
