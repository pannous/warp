//! Texts as runtime code sees them: their units, grapheme clusters, bounds, and the text heap
use super::*;

impl WasmGcEmitter {
	/// utf8_next and utf8_decode: every reader of text code point by code point steps with these two
	pub(super) fn emit_utf8_steps(&mut self) {
		let int = ValType::I32;
		// utf8_next(pointer, end): past the code point at pointer, never past end
		self.runtime_function("utf8_next", vec![int, int], vec![int], vec![int], |_, f| {
			let next = 2;
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(0), I::LocalGet(0), I::I32Load8U(BYTE), I::LocalSet(next),
				I32Const(1), I32Const(2), I32Const(3), I32Const(4), I::LocalGet(next), I32Const(utf8::FOUR_BYTE_LEAD),
				I::I32LtU, I::Select, I::LocalGet(next), I32Const(utf8::THREE_BYTE_LEAD), I::I32LtU, I::Select,
				I::LocalGet(next), I32Const(utf8::ONE_BYTE_END), I::I32LtU, I::Select, I::I32Add, I::LocalTee(next),
				I::LocalGet(1), I::LocalGet(next), I::LocalGet(1), I::I32LtU, I::Select,
			]);
		});

		// utf8_decode(pointer, end): the lead byte keeps 7, 5, 4 or 3 payload bits, each continuation byte adds 6
		self.runtime_function("utf8_decode", vec![int, int], vec![int], vec![int], |_, f| {
			let codepoint = 2;
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(0), I::I32Load8U(BYTE), I::LocalTee(codepoint), I32Const(utf8::ONE_BYTE_END), I::I32GeU,
				I::If(BlockType::Empty),
					I::LocalGet(codepoint), I32Const(utf8::FOUR_BYTE_PAYLOAD), I32Const(utf8::THREE_BYTE_PAYLOAD),
					I32Const(utf8::TWO_BYTE_PAYLOAD), I::LocalGet(codepoint), I32Const(utf8::THREE_BYTE_LEAD),
					I::I32GeU, I::Select, I::LocalGet(codepoint), I32Const(utf8::FOUR_BYTE_LEAD), I::I32GeU, I::Select,
					I::I32And, I::LocalSet(codepoint),
			]);
			Self::emit_skip_continuation_bytes(f, 0, 1, Some(codepoint));
			Self::emit_list(f, &[I::End, I::LocalGet(codepoint)]);
		});
	}

	/// Text is UTF-8 and is read by one of three units: `.bytes` counts bytes, `.chars` code points,
	/// `#`, `count`, `length` and `.graphemes` user-perceived characters (grapheme clusters, see GRAPHEME_EXTEND)
	pub(super) fn emit_text_units(&mut self) {
		let node_ref = Ref(self.node_ref(false));
		let (int, long) = (ValType::I32, ValType::I64);

		// grapheme_end(pointer, end): past the grapheme cluster starting at pointer, like strings::grapheme_clusters
		self.runtime_function("grapheme_end", vec![int, int], vec![int], vec![int; 3], |s, f| {
			let (previous, next, unpaired) = (2, 3, 4);
			// fast path (card text-costs): an ASCII character but \r before an ASCII one or the end is a whole grapheme
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(0), I::I32Load8U(BYTE), I::LocalTee(previous), I32Const(0x80), I::I32LtU,
				I::LocalGet(previous), I32Const('\r' as i32), I::I32Ne, I::I32And, I::If(BlockType::Empty),
					I::LocalGet(0), I32Const(1), I::I32Add, I::LocalTee(next), I::LocalGet(1), I::I32GeU,
					I::If(BlockType::Empty),
						I::LocalGet(next), I::Return,
					I::End,
					I::LocalGet(next), I::I32Load8U(BYTE), I32Const(0x80), I::I32LtU, I::If(BlockType::Empty),
						I::LocalGet(next), I::Return,
				I::End, I::End,
			]);
			s.emit_next_code_point(f, 0, 1, previous);
			// CR LF stays together, any other control ends the cluster
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(previous), I32Const('\r' as i32), I::I32Eq, I::LocalGet(0), I::LocalGet(1), I::I32LtU,
				I::I32And, I::If(BlockType::Empty),
					I::LocalGet(0), I::I32Load8U(BYTE), I32Const('\n' as i32), I::I32Eq, I::If(BlockType::Empty),
						I::LocalGet(0), I32Const(1), I::I32Add, I::Return,
				I::End, I::End,
			]);
			Self::emit_in_ranges(f, previous, &[(0x00, 0x1F), (0x7F, 0x9F)]);
			Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(0), I::Return, I::End]);
			Self::emit_in_ranges(f, previous, &[REGIONAL_INDICATORS]);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalSet(unpaired),
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(0), I::LocalGet(1), I::I32GeU, I::BrIf(1),
						I::LocalGet(0), I::LocalGet(1),
			]);
			s.call(f, "utf8_decode");
			f.instruction(&I::LocalSet(next));
			// joins: an extending mark, a pictograph after ZWJ, the second regional indicator of a flag
			Self::emit_in_ranges(f, next, &GRAPHEME_EXTEND);
			Self::emit_list(f, &[I::LocalGet(previous), I32Const(ZERO_WIDTH_JOINER as i32), I::I32Eq]);
			Self::emit_in_ranges(f, next, &GRAPHEME_PICTOGRAPHIC);
			Self::emit_list(f, &[I::I32And, I::I32Or, I::LocalGet(unpaired)]);
			Self::emit_in_ranges(f, next, &[REGIONAL_INDICATORS]);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::I32And, I::I32Or, I::I32Eqz, I::BrIf(1),
				I32Const(0), I::LocalSet(unpaired),
				I::LocalGet(next), I::LocalSet(previous),
				I::LocalGet(0), I::LocalGet(1),
			]);
			s.call(f, "utf8_next");
			Self::emit_list(f, &[I::LocalSet(0), I::Br(0), I::End, I::End, I::LocalGet(0)]);
		});

		// text_grapheme_back(start, at, end, index, wanted) -> (address, index): from the grapheme `index` at `at` back to
		// one at or before `wanted`, by the ASCII bytes before it (each but \n starts a grapheme: nothing joins an ASCII
		// character to what precedes it, \r\n aside) and the graphemes counted from there; (start, 1) beyond BACK_REACH
		self.runtime_function(
			"text_grapheme_back",
			vec![int, int, int, long, long],
			vec![int, long],
			vec![int; 3],
			|s, f| {
				let (start, at, end, index, wanted, back, pointer, counted) = (0, 1, 2, 3, 4, 5, 6, 7);
				Self::emit_list(
					f,
					&[
						I::LocalGet(at),
						I::LocalSet(pointer),
						I::Block(BlockType::Empty),
						I::Loop(BlockType::Empty),
						I::LocalGet(index),
						I::LocalGet(wanted),
						I::I64LeS,
						I::If(BlockType::Empty),
						I::LocalGet(pointer),
						I::LocalGet(index),
						I::Return,
						I::End,
						// the nearest ASCII byte but \n before pointer
						I::LocalGet(pointer),
						I::LocalSet(back),
						I::Loop(BlockType::Empty),
						I::LocalGet(back),
						I32Const(1),
						I::I32Sub,
						I::LocalTee(back),
						I::LocalGet(start),
						I::I32LtU,
						I::LocalGet(back),
						I32Const(BACK_REACH),
						I::I32Add,
						I::LocalGet(at),
						I::I32LtU,
						I::I32Or,
						I::BrIf(2),
						I::LocalGet(back),
						I::I32Load8U(BYTE),
						I::LocalTee(counted),
						I32Const(utf8::ONE_BYTE_END),
						I::I32GeU,
						I::LocalGet(counted),
						I32Const(b'\n' as i32),
						I::I32Eq,
						I::I32Or,
						I::BrIf(0),
						I::End,
						// its graphemes up to pointer
						I::LocalGet(back),
						I::LocalSet(counted),
						I::Loop(BlockType::Empty),
						I::LocalGet(counted),
						I::LocalGet(end),
					],
				);
				s.call(f, "grapheme_end");
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalSet(counted),
					I::LocalGet(index), I::I64Const(1), I::I64Sub, I::LocalSet(index),
					I::LocalGet(counted), I::LocalGet(pointer), I::I32LtU, I::BrIf(0),
					I::End,
					I::LocalGet(back), I::LocalSet(pointer),
					I::Br(0),
					I::End,
					I::End,
					I::LocalGet(start), I::I64Const(1),
				]);
			},
		);

		// text_grapheme_offset(text, index): address of the grapheme at the 1-based index, trapping out of range. It walks
		// on from the nearest text cursor at or before the index in the same text, or back from one past it (a look
		// back, `src#(i-1)`, text_grapheme_back), then moves that text's cursor there: a scan takes linear time. A text
		// without a cursor takes the one with the smallest index, the cheapest to walk again, so short texts read by
		// turns (`word#j`) leave a long text's cursor where it is (card long-text)
		let locals = vec![int, int, int, long, long, int, long, int, long];
		self.runtime_function("text_grapheme_offset", vec![node_ref, long], vec![int], locals, |s, f| {
			let (pointer, end, start, wanted, base, back, back_index, slot, least) = (2, 3, 4, 5, 6, 7, 8, 9, 10);
			Self::emit_index_compare(f, I::I64LtS);
			s.emit_fail_if(f, "index_out_of_range");
			s.emit_text_bounds(f, pointer, end);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(pointer), I::LocalSet(start),
				I::LocalGet(1), I::LocalSet(wanted),
				I::I64Const(1), I::LocalSet(base),
			]);
			let cursors = s.text_cursors();
			for &(text, index, at) in &cursors {
				#[rustfmt::skip]
				let in_this_text = [
					I::GlobalGet(text), I::LocalGet(start), I::I32Eq, I::GlobalGet(at), I::LocalGet(end), I::I32LtU,
					I::I32And,
				];
				Self::emit_list(f, &in_this_text);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::GlobalGet(index), I::LocalGet(wanted), I::I64LeS, I::I32And, I::GlobalGet(index),
					I::LocalGet(base), I::I64GtS, I::I32And, I::If(BlockType::Empty),
						I::GlobalGet(at), I::LocalSet(pointer),
						I::GlobalGet(index), I::LocalSet(base),
					I::End,
				]);
				Self::emit_list(f, &in_this_text);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::GlobalGet(index), I::LocalGet(wanted), I::I64GtS, I::I32And, I::LocalGet(base),
					I::LocalGet(wanted), I::I64LtS, I::I32And, I::If(BlockType::Empty),
						I::LocalGet(start), I::GlobalGet(at), I::LocalGet(end), I::GlobalGet(index),
						I::LocalGet(wanted),
				]);
				s.call(f, "text_grapheme_back");
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalSet(back_index),
					I::LocalSet(back),
					I::LocalGet(back_index), I::LocalGet(base), I::I64GtS, I::If(BlockType::Empty),
						I::LocalGet(back), I::LocalSet(pointer),
						I::LocalGet(back_index), I::LocalSet(base),
					I::End,
					I::End,
				]);
			}
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(wanted), I::LocalGet(base), I::I64Sub, I::I64Const(1), I::I64Add, I::LocalSet(1),
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(pointer), I::LocalGet(end), I::I32GeU,
			]);
			s.emit_fail_if(f, "index_out_of_range");
			Self::emit_index_compare(f, I::I64LeS);
			Self::emit_list(f, &[I::BrIf(1), I::LocalGet(pointer), I::LocalGet(end)]);
			s.call(f, "grapheme_end");
			f.instruction(&I::LocalSet(pointer));
			Self::emit_index_compare(f, I::I64Sub);
			Self::emit_list(f, &[I::LocalSet(1), I::Br(0), I::End, I::End]);
			s.emit_move_text_cursor(f, &cursors, (start, wanted, pointer), (slot, least));
			f.instruction(&I::LocalGet(pointer));
		});

		// text_grapheme_count(text) -> i64
		let locals = vec![int, int, long, int, int, long];
		self.runtime_function("text_grapheme_count", vec![node_ref], vec![long], locals, |s, f| {
			s.emit_codepoint_counts_one(f);
			let (pointer, end, count, start, slot, least) = (1, 2, 3, 4, 5, 6);
			s.emit_text_bounds(f, pointer, end);
			f.instruction(&I::LocalGet(pointer));
			f.instruction(&I::LocalSet(start));
			// a text counted before (its bytes never change): its count
			let counted = s.texts_counted();
			for &(counted_start, counted_end, counted_count) in &counted {
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(start), I::GlobalGet(counted_start), I::I32Eq, I::LocalGet(end),
					I::GlobalGet(counted_end), I::I32Eq, I::I32And, I::If(BlockType::Empty),
						I::GlobalGet(counted_count), I::Return,
					I::End,
				]);
			}
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(pointer), I::LocalGet(end), I::I32GeU, I::BrIf(1),
						I::LocalGet(pointer), I::LocalGet(end),
			]);
			s.call(f, "grapheme_end");
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalSet(pointer),
				I::LocalGet(count), I::I64Const(1), I::I64Add, I::LocalSet(count),
				I::Br(0),
				I::End,
				I::End,
			]);
			// it replaces the smaller count: a short text counted between the counts of a long one leaves it (card
			// text-slice: `count(t[a:b])` in a loop recounted t at each slice)
			Self::emit_least_slot(f, counted.iter().map(|&(_, _, counted_count)| counted_count), slot, least);
			let parts = counted
				.iter()
				.map(|&(text_start, text_end, text_count)| [(text_start, start), (text_end, end), (text_count, count)]);
			Self::emit_set_slot(f, slot, parts);
			f.instruction(&I::LocalGet(count));
		});

		// text_codepoint_count(text) -> i64: every byte but a continuation byte (10xxxxxx) starts a code point
		self.runtime_function("text_codepoint_count", vec![node_ref], vec![long], vec![int, int, long], |s, f| {
			s.emit_codepoint_counts_one(f);
			let (pointer, end, count) = (1, 2, 3);
			s.emit_text_bounds(f, pointer, end);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(pointer), I::LocalGet(end), I::I32GeU, I::BrIf(1),
						I::LocalGet(count), I::LocalGet(pointer), I::I32Load8U(BYTE), I32Const(utf8::CONTINUATION_MASK),
						I::I32And, I32Const(utf8::CONTINUATION_MARK), I::I32Ne, I::I64ExtendI32U, I::I64Add,
						I::LocalSet(count),
						I::LocalGet(pointer), I32Const(1), I::I32Add, I::LocalSet(pointer),
						I::Br(0),
				I::End, I::End,
				I::LocalGet(count),
			]);
		});

		// text_byte_count(text) -> i64
		self.runtime_function("text_byte_count", vec![node_ref], vec![long], vec![], |s, f| {
			s.emit_text_field(f, 0, 1);
			f.instruction(&I::I64ExtendI32U);
		});
	}

	/// Push whether code point `local` lies in one of the inclusive `ranges` (i32 0/1)
	pub(super) fn emit_in_ranges(func: &mut Function, local: u32, ranges: &[(u32, u32)]) {
		for (i, &(low, high)) in ranges.iter().enumerate() {
			Self::emit_list(
				func,
				&[I::LocalGet(local), I32Const(low as i32), I::I32Sub, I32Const((high - low + 1) as i32), I::I32LtU],
			);
			if i > 0 {
				func.instruction(&I::I32Or);
			}
		}
	}

	/// A one-character text, held as a codepoint in node local 0, is one code point and one grapheme
	fn emit_codepoint_counts_one(&self, func: &mut Function) {
		self.emit_field(func, 0, 0);
		Self::emit_kind_test(func, Kind::Codepoint as i64, I::I64Eq);
		Self::emit_list(func, &[I::If(BlockType::Empty), I::I64Const(1), I::Return, I::End]);
	}

	/// The cursor of the text at local `start` set to grapheme `wanted` at address `pointer` (the locals of `place`): the
	/// cursor holding that text, else the one with the smallest index (the locals of `scratch`)
	fn emit_move_text_cursor(
		&self,
		f: &mut Function,
		cursors: &[(u32, u32, u32)],
		(start, wanted, pointer): (u32, u32, u32),
		(slot, least): (u32, u32),
	) {
		Self::emit_least_slot(f, cursors.iter().map(|&(_, index, _)| index), slot, least);
		for (number, &(text, _, _)) in cursors.iter().enumerate() {
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::GlobalGet(text), I::LocalGet(start), I::I32Eq, I::If(BlockType::Empty),
					I::I32Const(number as i32), I::LocalSet(slot),
				I::End,
			]);
		}
		let parts = cursors.iter().map(|&(text, index, at)| [(text, start), (index, wanted), (at, pointer)]);
		Self::emit_set_slot(f, slot, parts);
	}

	/// Local `slot` set to the number of the slot whose i64 global in `keys` is smallest, the first of equals (local
	/// `least` is scratch): the cache slot that is cheapest to fill again
	fn emit_least_slot(f: &mut Function, keys: impl IntoIterator<Item = u32>, slot: u32, least: u32) {
		Self::emit_list(f, &[I::I32Const(0), I::LocalSet(slot), I::I64Const(i64::MAX), I::LocalSet(least)]);
		for (number, key) in keys.into_iter().enumerate() {
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::GlobalGet(key), I::LocalGet(least), I::I64LtS, I::If(BlockType::Empty),
					I::I32Const(number as i32), I::LocalSet(slot),
					I::GlobalGet(key), I::LocalSet(least),
				I::End,
			]);
		}
	}

	/// The globals of slot number local `slot` set from their locals: each slot lists its (global, local) pairs
	fn emit_set_slot<const PARTS: usize>(
		f: &mut Function,
		slot: u32,
		slots: impl IntoIterator<Item = [(u32, u32); PARTS]>,
	) {
		for (number, parts) in slots.into_iter().enumerate() {
			Self::emit_list(f, &[I::LocalGet(slot), I::I32Const(number as i32), I::I32Eq, I::If(BlockType::Empty)]);
			for (global, local) in parts {
				Self::emit_list(f, &[I::LocalGet(local), I::GlobalSet(global)]);
			}
			f.instruction(&I::End);
		}
	}

	/// Locals `pointer` and `end` span the bytes of text node local 0
	pub(in crate::wasm_emitter) fn emit_text_bounds(&self, func: &mut Function, pointer: u32, end: u32) {
		self.emit_text_field(func, 0, 0);
		func.instruction(&I::LocalTee(pointer));
		self.emit_text_field(func, 0, 1);
		Self::emit_list(func, &[I::I32Add, I::LocalSet(end)]);
	}

	/// i32 1 when the text from local `pointer` to local `end` is one character: as many bytes as its first byte's UTF-8
	/// sequence, 1 + (b ≥ C0) + (b ≥ E0) + (b ≥ F0); local `byte` is scratch
	pub(in crate::wasm_emitter) fn emit_is_one_character(func: &mut Function, pointer: u32, end: u32, byte: u32) {
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::LocalGet(end), I::LocalGet(pointer), I::I32Sub,
			I::I32Const(1),
			I::LocalGet(pointer), I::I32Load8U(BYTE), I::LocalTee(byte), I::I32Const(utf8::TWO_BYTE_LEAD), I::I32GeU,
			I::I32Add,
			I::LocalGet(byte), I::I32Const(utf8::THREE_BYTE_LEAD), I::I32GeU, I::I32Add,
			I::LocalGet(byte), I::I32Const(utf8::FOUR_BYTE_LEAD), I::I32GeU, I::I32Add,
			I::I32Eq,
		]);
	}

	/// The code point at local `pointer` into local `code_point`, and `pointer` past it (never past `end`)
	pub(in crate::wasm_emitter) fn emit_next_code_point(
		&self,
		func: &mut Function,
		pointer: u32,
		end: u32,
		code_point: u32,
	) {
		Self::emit_list(func, &[I::LocalGet(pointer), I::LocalGet(end)]);
		self.call(func, "utf8_decode");
		Self::emit_list(func, &[I::LocalSet(code_point), I::LocalGet(pointer), I::LocalGet(end)]);
		self.call(func, "utf8_next");
		func.instruction(&I::LocalSet(pointer));
	}

	/// Push the UTF-8 width of the code point in local `code_point`: 1 to 4 bytes
	pub(in crate::wasm_emitter) fn emit_utf8_width(func: &mut Function, code_point: u32) {
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I32Const(1), I32Const(2), I32Const(3), I32Const(4), I::LocalGet(code_point), I32Const(utf8::THREE_BYTE_END),
			I::I32LtU, I::Select, I::LocalGet(code_point), I32Const(utf8::TWO_BYTE_END), I::I32LtU, I::Select,
			I::LocalGet(code_point), I32Const(utf8::ONE_BYTE_END), I::I32LtU, I::Select,
		]);
	}

	/// Write the code point in local `code_point` as the `width` UTF-8 bytes at local `at`; uses up `code_point`, `width`
	/// and `lead`
	pub(in crate::wasm_emitter) fn emit_utf8_store(
		func: &mut Function,
		at: u32,
		code_point: u32,
		width: u32,
		lead: u32,
	) {
		Self::emit_list(
			func,
			&[
				// lead byte marker by width
				I32Const(0),
				I32Const(utf8::TWO_BYTE_LEAD),
				I32Const(utf8::THREE_BYTE_LEAD),
				I32Const(utf8::FOUR_BYTE_LEAD),
				I::LocalGet(width),
				I32Const(3),
				I::I32Eq,
				I::Select,
				I::LocalGet(width),
				I32Const(2),
				I::I32Eq,
				I::Select,
				I::LocalGet(width),
				I32Const(1),
				I::I32Eq,
				I::Select,
				I::LocalSet(lead),
				// continuation bytes from the last one back, 6 bits each
				I::Block(BlockType::Empty),
				I::Loop(BlockType::Empty),
				I::LocalGet(width),
				I32Const(1),
				I::I32LeU,
				I::BrIf(1),
				I::LocalGet(width),
				I32Const(1),
				I::I32Sub,
				I::LocalTee(width),
				I::LocalGet(at),
				I::I32Add,
				I::LocalGet(code_point),
				I32Const(utf8::CONTINUATION_PAYLOAD),
				I::I32And,
				I32Const(utf8::CONTINUATION_MARK),
				I::I32Or,
				I::I32Store8(BYTE),
				I::LocalGet(code_point),
				I32Const(utf8::PAYLOAD_BITS),
				I::I32ShrU,
				I::LocalSet(code_point),
				I::Br(0),
				I::End,
				I::End,
				I::LocalGet(at),
				I::LocalGet(code_point),
				I::LocalGet(lead),
				I::I32Or,
				I::I32Store8(BYTE),
			],
		);
	}

	/// pointer++ past following UTF-8 continuation bytes (10xxxxxx), shifting their 6 bits into `accumulator`
	pub(super) fn emit_skip_continuation_bytes(func: &mut Function, pointer: u32, end: u32, accumulator: Option<u32>) {
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::Loop(BlockType::Empty),
				I::LocalGet(pointer), I32Const(1), I::I32Add, I::LocalTee(pointer), I::LocalGet(end), I::I32LtU,
				I::If(BlockType::Empty),
					I::LocalGet(pointer), I::I32Load8U(BYTE), I32Const(utf8::CONTINUATION_MASK), I::I32And,
					I32Const(utf8::CONTINUATION_MARK), I::I32Eq, I::If(BlockType::Empty),
		]);
		if let Some(accumulator) = accumulator {
			#[rustfmt::skip]
			Self::emit_list(func, &[
				I::LocalGet(accumulator), I32Const(utf8::PAYLOAD_BITS), I::I32Shl, I::LocalGet(pointer),
				I::I32Load8U(BYTE), I32Const(utf8::CONTINUATION_PAYLOAD), I::I32And, I::I32Or, I::LocalSet(accumulator),
			]);
		}
		Self::emit_list(func, &[I::Br(2), I::End, I::End, I::End]);
	}
}

impl WasmGcEmitter {
	/// The text heap is exported: the host allocates the texts it returns (`read`, `fetch`) from it too
	pub(in crate::wasm_emitter) fn emit_text_heap_global(&mut self) {
		if self.text_heap_global.is_some() {
			return;
		}
		let global = self.declare_global_of(ValType::I32, ConstExpr::i32_const(0));
		self.text_heap_global = Some(global);
		self.exports.export(crate::host::TEXT_HEAP_EXPORT, ExportKind::Global, global);
	}

	/// Bump-allocate `length` bytes into local `address`; fresh pages come from memory.grow,
	/// so runtime texts never overlap the string table at the start of memory
	pub(in crate::wasm_emitter) fn emit_text_allocation(&self, func: &mut Function, length: u32, address: u32) {
		const PAGE_BITS: i32 = 16;
		let heap = self.text_heap_global.expect("emit_text_heap_global before allocating texts");
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::GlobalGet(heap), I::I32Eqz, I::GlobalGet(heap), I::LocalGet(length), I::I32Add, I::MemorySize(0),
			I32Const(PAGE_BITS), I::I32Shl, I::I32GtU, I::I32Or, I::If(BlockType::Empty),
				I::LocalGet(length), I32Const(PAGE_BITS), I::I32ShrU, I32Const(1), I::I32Add, I::MemoryGrow(0),
				I::LocalTee(address), I32Const(-1), I::I32Eq,
		]);
		self.emit_fail_if(func, "out_of_memory");
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::LocalGet(address), I32Const(PAGE_BITS), I::I32Shl, I::GlobalSet(heap),
			I::End,
			I::GlobalGet(heap), I::LocalTee(address), I::LocalGet(length), I::I32Add, I::GlobalSet(heap),
		]);
	}

	/// Grow the memory, when needed, until `length` more bytes fit after the heap global (new pages come at the end)
	pub(in crate::wasm_emitter) fn emit_memory_room(&self, func: &mut Function, heap: u32, length: u32) {
		const PAGE_BITS: i32 = 16;
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::GlobalGet(heap), I::LocalGet(length), I::I32Add, I::MemorySize(0), I32Const(PAGE_BITS), I::I32Shl,
			I::I32GtU, I::If(BlockType::Empty),
				I::GlobalGet(heap), I::LocalGet(length), I::I32Add, I::MemorySize(0), I32Const(PAGE_BITS), I::I32Shl,
				I::I32Sub, I32Const(PAGE_BITS), I::I32ShrU, I32Const(1), I::I32Add, I::MemoryGrow(0), I32Const(-1),
				I::I32Eq,
		]);
		self.emit_fail_if(func, "out_of_memory");
		func.instruction(&I::End);
	}

	pub(in crate::wasm_emitter) fn emit_is_text(&self, func: &mut Function) {
		for kind in [Kind::Text, Kind::Symbol] {
			self.emit_field(func, 0, 0);
			Self::emit_list(func, &[I::I64Const(kind as i64), I::I64Eq]);
		}
		func.instruction(&I::I32Or);
	}
}
