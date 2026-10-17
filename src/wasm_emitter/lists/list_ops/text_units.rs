//! Texts as runtime code sees them: their units, grapheme clusters, bounds, and the text heap. Readers walk the $Text
//! byte array by byte index: `bytes` the array, `at` a byte index, `end` the length
use super::*;

impl WasmGcEmitter {
	/// utf8_next and utf8_decode: every reader of text code point by code point steps with these two
	pub(super) fn emit_utf8_steps(&mut self) {
		let (int, bytes_type) = (ValType::I32, self.text_ref_nullable());
		let text_type = self.type_manager.text_type;
		let params = vec![bytes_type, int, int];
		let (bytes, at, end) = (0, 1, 2);
		// utf8_next(bytes, at, end): past the code point at `at`, never past end
		self.runtime_function("utf8_next", params.clone(), vec![int], vec![int], |_, f| {
			let next = 3;
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(at), I::LocalGet(bytes), I::LocalGet(at), I::ArrayGetU(text_type), I::LocalSet(next),
				I32Const(1), I32Const(2), I32Const(3), I32Const(4), I::LocalGet(next), I32Const(utf8::FOUR_BYTE_LEAD),
				I::I32LtU, I::Select, I::LocalGet(next), I32Const(utf8::THREE_BYTE_LEAD), I::I32LtU, I::Select,
				I::LocalGet(next), I32Const(utf8::ONE_BYTE_END), I::I32LtU, I::Select, I::I32Add, I::LocalTee(next),
				I::LocalGet(end), I::LocalGet(next), I::LocalGet(end), I::I32LtU, I::Select,
			]);
		});

		// utf8_decode(bytes, at, end): the lead byte keeps 7, 5, 4 or 3 payload bits, each continuation byte adds 6
		self.runtime_function("utf8_decode", params, vec![int], vec![int], |s, f| {
			let codepoint = 3;
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(bytes), I::LocalGet(at), I::ArrayGetU(text_type), I::LocalTee(codepoint),
				I32Const(utf8::ONE_BYTE_END), I::I32GeU,
				I::If(BlockType::Empty),
					I::LocalGet(codepoint), I32Const(utf8::FOUR_BYTE_PAYLOAD), I32Const(utf8::THREE_BYTE_PAYLOAD),
					I32Const(utf8::TWO_BYTE_PAYLOAD), I::LocalGet(codepoint), I32Const(utf8::THREE_BYTE_LEAD),
					I::I32GeU, I::Select, I::LocalGet(codepoint), I32Const(utf8::FOUR_BYTE_LEAD), I::I32GeU, I::Select,
					I::I32And, I::LocalSet(codepoint),
			]);
			s.emit_skip_continuation_bytes(f, (bytes, at, end), Some(codepoint));
			Self::emit_list(f, &[I::End, I::LocalGet(codepoint)]);
		});
	}

	/// Text is UTF-8 and is read by one of three units: `.bytes` counts bytes, `.chars` code points,
	/// `#`, `count`, `length` and `.graphemes` user-perceived characters (grapheme clusters, see GRAPHEME_EXTEND)
	pub(super) fn emit_text_units(&mut self) {
		let node_ref = Ref(self.node_ref(false));
		let (int, long, bytes_type) = (ValType::I32, ValType::I64, self.text_ref_nullable());
		let text_type = self.type_manager.text_type;

		// grapheme_end(bytes, at, end): past the grapheme cluster starting at `at`, like strings::grapheme_clusters
		self.runtime_function("grapheme_end", vec![bytes_type, int, int], vec![int], vec![int; 3], |s, f| {
			let (bytes, at, end, previous, next, unpaired) = (0, 1, 2, 3, 4, 5);
			// fast path (card text-costs): an ASCII character but \r before an ASCII one or the end is a whole grapheme
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(bytes), I::LocalGet(at), I::ArrayGetU(text_type), I::LocalTee(previous), I32Const(0x80),
				I::I32LtU, I::LocalGet(previous), I32Const('\r' as i32), I::I32Ne, I::I32And, I::If(BlockType::Empty),
					I::LocalGet(at), I32Const(1), I::I32Add, I::LocalTee(next), I::LocalGet(end), I::I32GeU,
					I::If(BlockType::Empty),
						I::LocalGet(next), I::Return,
					I::End,
					I::LocalGet(bytes), I::LocalGet(next), I::ArrayGetU(text_type), I32Const(0x80), I::I32LtU,
					I::If(BlockType::Empty),
						I::LocalGet(next), I::Return,
				I::End, I::End,
			]);
			s.emit_next_code_point(f, (bytes, at, end), previous);
			// CR LF stays together, any other control ends the cluster
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(previous), I32Const('\r' as i32), I::I32Eq, I::LocalGet(at), I::LocalGet(end), I::I32LtU,
				I::I32And, I::If(BlockType::Empty),
					I::LocalGet(bytes), I::LocalGet(at), I::ArrayGetU(text_type), I32Const('\n' as i32), I::I32Eq,
					I::If(BlockType::Empty),
						I::LocalGet(at), I32Const(1), I::I32Add, I::Return,
				I::End, I::End,
			]);
			Self::emit_in_ranges(f, previous, &[(0x00, 0x1F), (0x7F, 0x9F)]);
			Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(at), I::Return, I::End]);
			Self::emit_in_ranges(f, previous, &[REGIONAL_INDICATORS]);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalSet(unpaired),
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(at), I::LocalGet(end), I::I32GeU, I::BrIf(1),
						I::LocalGet(bytes), I::LocalGet(at), I::LocalGet(end),
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
				I::LocalGet(bytes), I::LocalGet(at), I::LocalGet(end),
			]);
			s.call(f, "utf8_next");
			Self::emit_list(f, &[I::LocalSet(at), I::Br(0), I::End, I::End, I::LocalGet(at)]);
		});

		// text_grapheme_back(bytes, at, end, index, wanted) -> (at, index): from the grapheme `index` at `at` back to one
		// at or before `wanted`, by the ASCII bytes before it (each but \n starts a grapheme: nothing joins an ASCII
		// character to what precedes it, \r\n aside) and the graphemes counted from there; (0, 1) beyond BACK_REACH
		self.runtime_function(
			"text_grapheme_back",
			vec![bytes_type, int, int, long, long],
			vec![int, long],
			vec![int; 3],
			|s, f| {
				let (bytes, at, end, index, wanted, back, pointer, counted) = (0, 1, 2, 3, 4, 5, 6, 7);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(at), I::LocalSet(pointer),
					I::Block(BlockType::Empty),
						I::Loop(BlockType::Empty),
							I::LocalGet(index), I::LocalGet(wanted), I::I64LeS, I::If(BlockType::Empty),
								I::LocalGet(pointer), I::LocalGet(index), I::Return,
							I::End,
							// the nearest ASCII byte but \n before pointer
							I::LocalGet(pointer), I::LocalSet(back),
							I::Loop(BlockType::Empty),
								I::LocalGet(back), I32Const(1), I::I32Sub, I::LocalTee(back), I32Const(0), I::I32LtS,
								I::LocalGet(back), I32Const(BACK_REACH), I::I32Add, I::LocalGet(at), I::I32LtS,
								I::I32Or, I::BrIf(2),
								I::LocalGet(bytes), I::LocalGet(back), I::ArrayGetU(text_type), I::LocalTee(counted),
								I32Const(utf8::ONE_BYTE_END), I::I32GeU, I::LocalGet(counted), I32Const(b'\n' as i32),
								I::I32Eq, I::I32Or, I::BrIf(0),
							I::End,
							// its graphemes up to pointer
							I::LocalGet(back), I::LocalSet(counted),
							I::Loop(BlockType::Empty),
								I::LocalGet(bytes), I::LocalGet(counted), I::LocalGet(end),
				]);
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
					I32Const(0), I::I64Const(1),
				]);
			},
		);

		// text_grapheme_offset(text, index): byte index of the grapheme at the 1-based index, trapping out of range. It
		// walks on from the nearest text cursor at or before the index in the same text, or back from one past it (a
		// look back, `src#(i-1)`, text_grapheme_back), then moves that text's cursor there: a scan takes linear time. A
		// text without a cursor takes the one with the smallest index, the cheapest to walk again, so short texts read
		// by turns (`word#j`) leave a long text's cursor where it is (card long-text)
		let locals = vec![int, int, long, long, int, long, int, long, bytes_type];
		self.runtime_function("text_grapheme_offset", vec![node_ref, long], vec![int], locals, |s, f| {
			let (pointer, end, wanted, base, back, back_index, slot, least, bytes) = (2, 3, 4, 5, 6, 7, 8, 9, 10);
			Self::emit_index_compare(f, I::I64LtS);
			s.emit_fail_if(f, "index_out_of_range");
			s.emit_text_bounds(f, (bytes, pointer, end));
			Self::emit_list(f, &[I::LocalGet(1), I::LocalSet(wanted), I::I64Const(1), I::LocalSet(base)]);
			let cursors = s.text_cursors();
			for &(text, index, at) in &cursors {
				#[rustfmt::skip]
				let in_this_text = [
					I::GlobalGet(text), I::LocalGet(bytes), I::RefEq, I::GlobalGet(at), I::LocalGet(end), I::I32LtU,
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
						I::LocalGet(bytes), I::GlobalGet(at), I::LocalGet(end), I::GlobalGet(index), I::LocalGet(wanted),
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
			Self::emit_list(f, &[I::BrIf(1), I::LocalGet(bytes), I::LocalGet(pointer), I::LocalGet(end)]);
			s.call(f, "grapheme_end");
			f.instruction(&I::LocalSet(pointer));
			Self::emit_index_compare(f, I::I64Sub);
			Self::emit_list(f, &[I::LocalSet(1), I::Br(0), I::End, I::End]);
			s.emit_move_text_cursor(f, &cursors, (bytes, wanted, pointer), (slot, least));
			f.instruction(&I::LocalGet(pointer));
		});

		// text_grapheme_count(text) -> i64
		let locals = vec![int, int, long, bytes_type, int, long];
		self.runtime_function("text_grapheme_count", vec![node_ref], vec![long], locals, |s, f| {
			s.emit_codepoint_counts_one(f);
			let (pointer, end, count, bytes, slot, least) = (1, 2, 3, 4, 5, 6);
			s.emit_text_bounds(f, (bytes, pointer, end));
			// a text counted before (its bytes never change): its count
			let counted = s.texts_counted();
			for &(counted_text, counted_count) in &counted {
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(bytes), I::GlobalGet(counted_text), I::RefEq, I::If(BlockType::Empty),
						I::GlobalGet(counted_count), I::Return,
					I::End,
				]);
			}
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(pointer), I::LocalGet(end), I::I32GeU, I::BrIf(1),
						I::LocalGet(bytes), I::LocalGet(pointer), I::LocalGet(end),
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
			Self::emit_least_slot(f, counted.iter().map(|&(_, counted_count)| counted_count), slot, least);
			let parts = counted.iter().map(|&(text, text_count)| [(text, bytes), (text_count, count)]);
			Self::emit_set_slot(f, slot, parts);
			f.instruction(&I::LocalGet(count));
		});

		// text_codepoint_count(text) -> i64: every byte but a continuation byte (10xxxxxx) starts a code point
		let locals = vec![int, int, long, bytes_type];
		self.runtime_function("text_codepoint_count", vec![node_ref], vec![long], locals, |s, f| {
			s.emit_codepoint_counts_one(f);
			let (pointer, end, count, bytes) = (1, 2, 3, 4);
			s.emit_text_bounds(f, (bytes, pointer, end));
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(pointer), I::LocalGet(end), I::I32GeU, I::BrIf(1),
						I::LocalGet(count), I::LocalGet(bytes), I::LocalGet(pointer), I::ArrayGetU(text_type),
						I32Const(utf8::CONTINUATION_MASK), I::I32And, I32Const(utf8::CONTINUATION_MARK), I::I32Ne,
						I::I64ExtendI32U, I::I64Add, I::LocalSet(count),
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

	/// The cursor of the text in local `bytes` set to grapheme `wanted` at byte index `pointer`: the cursor holding that
	/// text, else the one with the smallest index (locals `slot` and `least` are scratch)
	fn emit_move_text_cursor(
		&self,
		f: &mut Function,
		cursors: &[(u32, u32, u32)],
		(bytes, wanted, pointer): (u32, u32, u32),
		(slot, least): (u32, u32),
	) {
		Self::emit_least_slot(f, cursors.iter().map(|&(_, index, _)| index), slot, least);
		for (number, &(text, _, _)) in cursors.iter().enumerate() {
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::GlobalGet(text), I::LocalGet(bytes), I::RefEq, I::If(BlockType::Empty),
					I::I32Const(number as i32), I::LocalSet(slot),
				I::End,
			]);
		}
		let parts = cursors.iter().map(|&(text, index, at)| [(text, bytes), (index, wanted), (at, pointer)]);
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

	/// Local `bytes` set to the $Text of text node local 0, `pointer` to 0 and `end` to its length
	pub(in crate::wasm_emitter) fn emit_text_bounds(&self, func: &mut Function, (bytes, pointer, end): (u32, u32, u32)) {
		self.emit_text_bytes(func, 0);
		Self::emit_list(func, &[I::LocalTee(bytes), I::ArrayLen, I::LocalSet(end), I32Const(0), I::LocalSet(pointer)]);
	}

	/// i32 1 when the text from local `pointer` to local `end` is one character: as many bytes as its first byte's UTF-8
	/// sequence, 1 + (b ≥ C0) + (b ≥ E0) + (b ≥ F0); local `byte` is scratch
	pub(in crate::wasm_emitter) fn emit_is_one_character(
		&self,
		func: &mut Function,
		(bytes, pointer, end): (u32, u32, u32),
		byte: u32,
	) {
		// an empty text has no first byte to read
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::LocalGet(end), I::LocalGet(pointer), I::I32GtU, I::If(BlockType::Result(ValType::I32)),
			I::LocalGet(end), I::LocalGet(pointer), I::I32Sub,
			I::I32Const(1),
			I::LocalGet(bytes), I::LocalGet(pointer), I::ArrayGetU(self.type_manager.text_type), I::LocalTee(byte), I::I32Const(utf8::TWO_BYTE_LEAD), I::I32GeU,
			I::I32Add,
			I::LocalGet(byte), I::I32Const(utf8::THREE_BYTE_LEAD), I::I32GeU, I::I32Add,
			I::LocalGet(byte), I::I32Const(utf8::FOUR_BYTE_LEAD), I::I32GeU, I::I32Add,
			I::I32Eq,
			I::Else, I::I32Const(0), I::End,
		]);
	}

	/// The code point at byte index local `pointer` of local `bytes` into local `code_point`, and `pointer` past it (never
	/// past `end`)
	pub(in crate::wasm_emitter) fn emit_next_code_point(
		&self,
		func: &mut Function,
		(bytes, pointer, end): (u32, u32, u32),
		code_point: u32,
	) {
		Self::emit_list(func, &[I::LocalGet(bytes), I::LocalGet(pointer), I::LocalGet(end)]);
		self.call(func, "utf8_decode");
		Self::emit_list(func, &[I::LocalSet(code_point), I::LocalGet(bytes), I::LocalGet(pointer), I::LocalGet(end)]);
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

	/// Write the code point in local `code_point` as the `width` UTF-8 bytes at byte index local `at` of the $Text in
	/// local `bytes`; uses up `code_point`, `width` and `lead`
	pub(in crate::wasm_emitter) fn emit_utf8_store(
		&self,
		func: &mut Function,
		(bytes, at): (u32, u32),
		code_point: u32,
		width: u32,
		lead: u32,
	) {
		let text_type = self.type_manager.text_type;
		#[rustfmt::skip]
		Self::emit_list(func, &[
			// lead byte marker by width
			I32Const(0), I32Const(utf8::TWO_BYTE_LEAD), I32Const(utf8::THREE_BYTE_LEAD), I32Const(utf8::FOUR_BYTE_LEAD),
			I::LocalGet(width), I32Const(3), I::I32Eq, I::Select,
			I::LocalGet(width), I32Const(2), I::I32Eq, I::Select,
			I::LocalGet(width), I32Const(1), I::I32Eq, I::Select,
			I::LocalSet(lead),
			// continuation bytes from the last one back, 6 bits each
			I::Block(BlockType::Empty),
				I::Loop(BlockType::Empty),
					I::LocalGet(width), I32Const(1), I::I32LeU, I::BrIf(1),
					I::LocalGet(bytes),
					I::LocalGet(width), I32Const(1), I::I32Sub, I::LocalTee(width), I::LocalGet(at), I::I32Add,
					I::LocalGet(code_point), I32Const(utf8::CONTINUATION_PAYLOAD), I::I32And,
					I32Const(utf8::CONTINUATION_MARK), I::I32Or, I::ArraySet(text_type),
					I::LocalGet(code_point), I32Const(utf8::PAYLOAD_BITS), I::I32ShrU, I::LocalSet(code_point),
					I::Br(0),
				I::End,
			I::End,
			I::LocalGet(bytes), I::LocalGet(at), I::LocalGet(code_point), I::LocalGet(lead), I::I32Or,
			I::ArraySet(text_type),
		]);
	}

	/// pointer++ past following UTF-8 continuation bytes (10xxxxxx), shifting their 6 bits into `accumulator`
	pub(super) fn emit_skip_continuation_bytes(
		&self,
		func: &mut Function,
		(bytes, pointer, end): (u32, u32, u32),
		accumulator: Option<u32>,
	) {
		let text_type = self.type_manager.text_type;
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::Loop(BlockType::Empty),
				I::LocalGet(pointer), I32Const(1), I::I32Add, I::LocalTee(pointer), I::LocalGet(end), I::I32LtU,
				I::If(BlockType::Empty),
					I::LocalGet(bytes), I::LocalGet(pointer), I::ArrayGetU(text_type), I32Const(utf8::CONTINUATION_MASK),
					I::I32And,
					I32Const(utf8::CONTINUATION_MARK), I::I32Eq, I::If(BlockType::Empty),
		]);
		if let Some(accumulator) = accumulator {
			#[rustfmt::skip]
			Self::emit_list(func, &[
				I::LocalGet(accumulator), I32Const(utf8::PAYLOAD_BITS), I::I32Shl, I::LocalGet(bytes),
				I::LocalGet(pointer), I::ArrayGetU(text_type), I32Const(utf8::CONTINUATION_PAYLOAD), I::I32And, I::I32Or, I::LocalSet(accumulator),
			]);
		}
		Self::emit_list(func, &[I::Br(2), I::End, I::End, I::End]);
	}
}

impl WasmGcEmitter {
	/// The functions every text operation builds on: text_from_memory(ptr, len) copies bytes a host or a builder left in
	/// linear memory into a new $Text, text_bytes_join, text_bytes_equal and text_flat
	pub(in crate::wasm_emitter) fn emit_text_foundation(&mut self) {
		let (int, text) = (ValType::I32, self.text_ref());
		let text_type = self.type_manager.text_type;
		self.runtime_function(TEXT_FROM_MEMORY, vec![int, int], vec![text], vec![text, int], |_, f| {
			let (pointer, length, bytes, index) = (0, 1, 2, 3);
			Self::emit_list(f, &[I::LocalGet(length), I::ArrayNewDefault(text_type), I::LocalSet(bytes)]);
			let condition = [I::LocalGet(index), I::LocalGet(length), I::I32LtU];
			#[rustfmt::skip]
			Self::emit_while(f, &condition, &[
				I::LocalGet(bytes), I::LocalGet(index), I::LocalGet(pointer), I::LocalGet(index), I::I32Add,
				I::I32Load8U(BYTE), I::ArraySet(text_type),
				I::LocalGet(index), I32Const(1), I::I32Add, I::LocalSet(index),
			]);
			Self::emit_list(f, &[I::LocalGet(bytes), I::RefAsNonNull]);
		});
		self.runtime_function(TEXT_BYTES_JOIN, vec![text, text], vec![text], vec![text], |_, f| {
			let (first, second, joined) = (0, 1, 2);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(first), I::ArrayLen, I::LocalGet(second), I::ArrayLen, I::I32Add,
				I::ArrayNewDefault(text_type), I::LocalTee(joined),
				I32Const(0), I::LocalGet(first), I32Const(0), I::LocalGet(first), I::ArrayLen,
				I::ArrayCopy { array_type_index_dst: text_type, array_type_index_src: text_type },
				I::LocalGet(joined), I::LocalGet(first), I::ArrayLen, I::LocalGet(second), I32Const(0),
				I::LocalGet(second), I::ArrayLen,
				I::ArrayCopy { array_type_index_dst: text_type, array_type_index_src: text_type },
				I::LocalGet(joined), I::RefAsNonNull,
			]);
		});
		let nullable_text = self.text_ref_nullable();
		self.runtime_function(TEXT_PIECE, vec![nullable_text, int, int], vec![text], vec![nullable_text], |_, f| {
			let (bytes, start, length, piece) = (0, 1, 2, 3);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(length), I::ArrayNewDefault(text_type), I::LocalTee(piece),
				I32Const(0), I::LocalGet(bytes), I::LocalGet(start), I::LocalGet(length),
				I::ArrayCopy { array_type_index_dst: text_type, array_type_index_src: text_type },
				I::LocalGet(piece), I::RefAsNonNull,
			]);
		});
		self.emit_text_bytes_equal();
		self.emit_text_flat();
	}

	/// text_bytes_equal($Text, $Text) -> i32: the same bytes
	pub(in crate::wasm_emitter) fn emit_text_bytes_equal(&mut self) {
		let (text, text_type) = (self.text_ref(), self.type_manager.text_type);
		self.runtime_function(TEXT_BYTES_EQUAL, vec![text, text], vec![ValType::I32], vec![ValType::I32], |_, f| {
			let (first, second, index) = (0, 1, 2);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(first), I::ArrayLen, I::LocalGet(second), I::ArrayLen, I::I32Ne,
				I::If(BlockType::Empty), I32Const(0), I::Return, I::End,
			]);
			let condition = [I::LocalGet(index), I::LocalGet(first), I::ArrayLen, I::I32LtU];
			#[rustfmt::skip]
			Self::emit_while(f, &condition, &[
				I::LocalGet(first), I::LocalGet(index), I::ArrayGetU(text_type),
				I::LocalGet(second), I::LocalGet(index), I::ArrayGetU(text_type), I::I32Ne,
				I::If(BlockType::Empty), I32Const(0), I::Return, I::End,
				I::LocalGet(index), I32Const(1), I::I32Add, I::LocalSet(index),
			]);
			f.instruction(&I32Const(1));
		});
	}

	/// text_flat(node) -> $Text: the bytes of a Text, Symbol or Error node; a rope (text_append: data the last piece,
	/// value the text before it) is flattened once, its node then holding the whole text
	fn emit_text_flat(&mut self) {
		let (node, text_type) = (self.type_manager.node_type, self.type_manager.text_type);
		let (node_ref, nullable) = (Ref(self.node_ref(false)), Ref(self.node_ref(true)));
		let (text, int) = (self.text_ref(), ValType::I32);
		let nullable_text = self.text_ref_nullable();
		self.runtime_function(TEXT_FLAT, vec![node_ref], vec![text], vec![nullable, int, nullable_text, int], |s, f| {
			let (rope, piece, length, flat, piece_length) = (0, 1, 2, 3, 4);
			let data = |f: &mut Function, local: u32| {
				Self::emit_list(f, &[I::LocalGet(local), I::RefAsNonNull]);
				Self::emit_list(f, &[I::StructGet { struct_type_index: node, field_index: 1 }]);
				f.instruction(&I::RefCastNonNull(HeapType::Concrete(text_type)));
			};
			let rest = |f: &mut Function, local: u32| {
				Self::emit_list(f, &[I::LocalGet(local), I::RefAsNonNull]);
				Self::emit_list(f, &[I::StructGet { struct_type_index: node, field_index: 2 }, I::LocalSet(local)]);
			};
			s.emit_field(f, rope, 2);
			Self::emit_list(f, &[I::RefIsNull, I::If(BlockType::Empty)]);
			data(f, rope);
			Self::emit_list(f, &[I::Return, I::End, I::LocalGet(rope), I::LocalSet(piece)]);
			let walk = |f: &mut Function, body: &dyn Fn(&mut Function)| {
				let condition = [I::LocalGet(piece), I::RefIsNull, I::I32Eqz];
				Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
				Self::emit_list(f, &condition);
				Self::emit_list(f, &[I::I32Eqz, I::BrIf(1)]);
				body(f);
				rest(f, piece);
				Self::emit_list(f, &[I::Br(0), I::End, I::End]);
			};
			walk(f, &|f| {
				data(f, piece);
				Self::emit_list(f, &[I::ArrayLen, I::LocalGet(length), I::I32Add, I::LocalSet(length)]);
			});
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(length), I::ArrayNewDefault(text_type), I::LocalSet(flat),
				I::LocalGet(rope), I::LocalSet(piece),
			]);
			walk(f, &|f| {
				data(f, piece);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::ArrayLen, I::LocalTee(piece_length), I::LocalGet(length), I::LocalGet(piece_length), I::I32Sub,
					I::LocalSet(length), I::Drop,
					I::LocalGet(flat), I::LocalGet(length),
				]);
				data(f, piece);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I32Const(0), I::LocalGet(piece_length),
					I::ArrayCopy { array_type_index_dst: text_type, array_type_index_src: text_type },
				]);
			});
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(rope), I::LocalGet(flat), I::StructSet { struct_type_index: node, field_index: 1 },
				I::LocalGet(rope), I::RefNull(HeapType::Concrete(node)), I::StructSet { struct_type_index: node, field_index: 2 },
				I::LocalGet(flat), I::RefAsNonNull,
			]);
		});
	}

	/// Push the $Text of the Text or Symbol node in local `node`: its data, unless it is a rope (text_flat)
	pub(in crate::wasm_emitter) fn emit_text_bytes(&self, func: &mut Function, node: u32) {
		let (node_type, text_type) = (self.type_manager.node_type, self.type_manager.text_type);
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::LocalGet(node), I::StructGet { struct_type_index: node_type, field_index: 2 }, I::RefIsNull,
			I::If(BlockType::Result(self.text_ref())),
				I::LocalGet(node), I::StructGet { struct_type_index: node_type, field_index: 1 },
				I::RefCastNonNull(HeapType::Concrete(text_type)),
			I::Else,
				I::LocalGet(node), I::RefAsNonNull,
		]);
		self.call(func, TEXT_FLAT);
		func.instruction(&I::End);
	}

	/// text_address(text): a copy of the text's bytes in the heap, for a host or C function reading by address; after
	/// the runtime errors, out_of_memory among them
	pub(in crate::wasm_emitter) fn emit_text_address(&mut self) {
		self.emit_text_heap_global();
		let (int, text) = (ValType::I32, self.text_ref());
		let text_type = self.type_manager.text_type;
		self.runtime_function(TEXT_ADDRESS, vec![text], vec![int], vec![int; 3], |s, f| {
			let (bytes, length, address, index) = (0, 1, 2, 3);
			Self::emit_list(f, &[I::LocalGet(bytes), I::ArrayLen, I::LocalSet(length)]);
			s.emit_text_allocation(f, length, address);
			let condition = [I::LocalGet(index), I::LocalGet(length), I::I32LtU];
			#[rustfmt::skip]
			Self::emit_while(f, &condition, &[
				I::LocalGet(address), I::LocalGet(index), I::I32Add, I::LocalGet(bytes), I::LocalGet(index),
				I::ArrayGetU(text_type), I::I32Store8(BYTE),
				I::LocalGet(index), I32Const(1), I::I32Add, I::LocalSet(index),
			]);
			f.instruction(&I::LocalGet(address));
		});
	}

	/// (ref $Text)
	pub(in crate::wasm_emitter) fn text_ref(&self) -> ValType {
		Ref(RefType { nullable: false, heap_type: HeapType::Concrete(self.type_manager.text_type) })
	}

	/// (ref null $Text): a local or global holding a text's bytes
	pub(in crate::wasm_emitter) fn text_ref_nullable(&self) -> ValType {
		Ref(RefType { nullable: true, heap_type: HeapType::Concrete(self.type_manager.text_type) })
	}

	/// Push a new Text node of the bytes of local `bytes` from byte index local `start` to `stop`
	pub(in crate::wasm_emitter) fn emit_text_piece(&self, func: &mut Function, (bytes, start, stop): (u32, u32, u32)) {
		Self::emit_list(func, &[I::LocalGet(bytes), I::LocalGet(start), I::LocalGet(stop), I::LocalGet(start), I::I32Sub]);
		self.call(func, TEXT_PIECE);
		self.call(func, NEW_TEXT_OF);
	}

	/// Push the literal `s` as a new $Text, from the literal segment
	pub(in crate::wasm_emitter) fn emit_text_literal(&mut self, func: &mut Function, s: &str) {
		let literal = self.string_table.literal(s);
		self.emit_literal_text(func, literal);
	}

	/// Push the literal at (offset, length) of the literal segment (StringTable::literal) as a new $Text
	pub(in crate::wasm_emitter) fn emit_literal_text(&self, func: &mut Function, (offset, length): (u32, u32)) {
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I32Const(offset as i32), I32Const(length as i32),
			I::ArrayNewData { array_type_index: self.type_manager.text_type, array_data_index: TEXT_SEGMENT },
		]);
	}

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

	pub(in crate::wasm_emitter) fn emit_is_text(&self, func: &mut Function) {
		for kind in [Kind::Text, Kind::Symbol] {
			self.emit_field(func, 0, 0);
			Self::emit_list(func, &[I::I64Const(kind as i64), I::I64Eq]);
		}
		func.instruction(&I::I32Or);
	}
}
