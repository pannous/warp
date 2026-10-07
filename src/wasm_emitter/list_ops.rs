//! List and string operation functions for WASM

use crate::wasm_emitter::{WasmGcEmitter, VALUES_EQUAL};
use wasm_encoder::*;
use Instruction::I32Const;
use Instruction as I;
use ValType::Ref;
use crate::type_kinds::{Kind, CURLY_LIST_KIND, KIND_MASK, SQUARE_LIST_KIND};
use crate::wasm_emitter::layout::{utf8, BYTE};
use crate::node::Node;
use crate::extensions::strings::{GRAPHEME_EXTEND, GRAPHEME_PICTOGRAPHIC, REGIONAL_INDICATORS, ZERO_WIDTH_JOINER};


/// An ASCII letter with this bit set is lowercase: `byte | ASCII_LOWERCASE_BIT == 'e'` takes e and E
const ASCII_LOWERCASE_BIT: i32 = 0x20;
/// Runtime functions that read text by one of its units (byte, code point, grapheme)
const TEXT_UNIT_USERS: [&str; 7] =
	["node_count", "node_bytes", "string_char_at", "node_with_at", "text_byte_count", "text_codepoint_count", "text_grapheme_count"];

/// The locals of a copy of a list's first cells (emit_collect_cells, emit_rebuild_cells), consecutive from `cell`
pub(super) struct CopyCells {
	cell: u32,
	cells: u32,
	position: u32,
	copied: u32,
	/// what the copied cells go in front of; the copy when rebuilt
	result: u32,
}

impl CopyCells {
	pub(super) fn at(first: u32) -> Self {
		CopyCells { cell: first, cells: first + 1, position: first + 2, copied: first + 3, result: first + 4 }
	}
}

impl WasmGcEmitter {
	/// Emit list and string operation helper functions
	pub(crate) fn emit_list_ops(&mut self) {
		self.emit_is_meta_entry();
		self.emit_zero_fill();
		if TEXT_UNIT_USERS.iter().any(|name| self.should_emit_function(name)) {
			self.emit_text_units();
		}
		self.emit_list_cell_access();
		self.emit_node_counting();
		self.emit_node_kind_test();
		self.emit_node_indexing();
		self.emit_with_at_functions();
		self.emit_typed_list_runtime();
		self.emit_list_concat();
		self.emit_list_insert_at();
	}

	/// list_at and list_node_at: the element of a cons list at a 1-based index
	fn emit_list_cell_access(&mut self) {
		let node_ref_nullable = self.node_ref(true);
		let node_ref = self.node_ref(false);
		// list_at(list: ref $Node, index: i64) -> i64
		// Get the numeric value of the element at index (1-based)
		// Traverses linked list: for index N, follow value pointer N-1 times, return data
		if self.should_emit_function("list_at") {
			self.exported_function("list_at", vec![Ref(node_ref), ValType::I64], vec![ValType::I64], vec![Ref(node_ref_nullable)], |s, func| {
				// Locals: 0=list, 1=index, 2=current (loop variable)
				s.emit_require_integral(func, 1);

				s.emit_list_walk(func, 2);

				// Get current.data (which is a ref to the element Node, cast from anyref)
				func.instruction(&I::LocalGet(2));
				func.instruction(&I::StructGet {
					struct_type_index: s.type_manager.node_type,
					field_index: 1, // data field (anyref holding ref $Node)
				});
				// a character element is its code point, any other non-Int element not_an_int (no cast trap)
				func.instruction(&I::RefCastNonNull(HeapType::Concrete(s.type_manager.node_type)));
				s.emit_call(func, "get_int_value");
			});
		}

		// list_node_at(list: ref $Node, index: i64) -> ref $Node
		// Get the element node at index (1-based), returns the node itself (for symbol/text lists)
		if self.should_emit_function("list_node_at") {
			self.exported_function("list_node_at", vec![Ref(node_ref), ValType::I64], vec![Ref(node_ref)], vec![Ref(node_ref_nullable)], |s, func| {
				// Locals: 0=list, 1=index, 2=current (loop variable)
				s.emit_require_integral(func, 1);

				s.emit_list_walk(func, 2);

				// Get current.data (which is a ref to the element Node, cast from anyref)
				func.instruction(&I::LocalGet(2));
				func.instruction(&I::StructGet {
					struct_type_index: s.type_manager.node_type,
					field_index: 1, // data field (anyref holding ref $Node)
				});
				// Cast anyref to ref $Node and return it directly
				func.instruction(&I::RefCastNonNull(HeapType::Concrete(s.type_manager.node_type)));
			});
		}
	}

	/// node_count and node_bytes
	fn emit_node_kind_test(&mut self) {
		let node_ref = self.node_ref(false);
		// node_kind_in(node, mask) -> i64: 1 when bit `kind` of the mask is set (type tests of values of unknown static type)
		if self.should_emit_function(crate::type_tests::NODE_KIND_IN) {
			self.runtime_function(crate::type_tests::NODE_KIND_IN, vec![Ref(node_ref), ValType::I64], vec![ValType::I64], vec![], |s, f| {
				f.instruction(&I::LocalGet(1));
				s.emit_field(f, 0, 0);
				Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::I64ShrU, I::I64Const(1), I::I64And]);
			});
		}
	}

	fn emit_node_counting(&mut self) {
		let node_ref_nullable = self.node_ref(true);
		let node_ref = self.node_ref(false);
		// node_count(node: ref $Node) -> i64
		// Count the number of elements in a list/block by traversing the value chain
		if self.should_emit_function("node_count") {
			self.exported_function("node_count", vec![Ref(node_ref)], vec![ValType::I64], vec![ValType::I64, Ref(node_ref_nullable)], |s, func| {
				// Locals: 0=node, 1=count, 2=current

				// a text counts its user-perceived characters (grapheme clusters)
				s.emit_is_text(func);
				func.instruction(&I::If(BlockType::Empty));
				func.instruction(&I::LocalGet(0));
				s.call(func, "text_grapheme_count");
				func.instruction(&I::Return);
				func.instruction(&I::End);

				// ø is the empty list
				s.emit_field(func, 0, 0);
				func.instruction(&I::I64Const(Kind::Empty as i64));
				func.instruction(&I::I64Eq);
				func.instruction(&I::If(BlockType::Empty));
				func.instruction(&I::I64Const(0));
				func.instruction(&I::Return);
				func.instruction(&I::End);

				// a key:value pair is one item, the one-entry object `{a:1}`
				s.emit_field(func, 0, 0);
				func.instruction(&I::I64Const(KIND_MASK));
				func.instruction(&I::I64And);
				func.instruction(&I::I64Const(KEY_KIND));
				func.instruction(&I::I64Eq);
				func.instruction(&I::If(BlockType::Empty));
				func.instruction(&I::I64Const(1));
				func.instruction(&I::Return);
				func.instruction(&I::End);

				// count = 0
				func.instruction(&I::I64Const(0));
				func.instruction(&I::LocalSet(1));

				// current = node
				func.instruction(&I::LocalGet(0));
				func.instruction(&I::LocalSet(2));

				// Loop: while current is not null, count++ and current = current.value
				func.instruction(&I::Block(BlockType::Empty));
				func.instruction(&I::Loop(BlockType::Empty));

				// if current is null, break
				func.instruction(&I::LocalGet(2));
				func.instruction(&I::RefIsNull);
				func.instruction(&I::BrIf(1)); // break to outer block

				// count = count + 1, unless the entry is a meta entry `@name:value`
				func.instruction(&I::LocalGet(1));
				s.emit_field(func, 2, 1);
				s.call(func, super::equality::IS_META_ENTRY);
				func.instruction(&I::I32Eqz);
				func.instruction(&I::I64ExtendI32U);
				func.instruction(&I::I64Add);
				func.instruction(&I::LocalSet(1));

				// current = current.value (field 2)
				func.instruction(&I::LocalGet(2));
				func.instruction(&I::StructGet {
					struct_type_index: s.type_manager.node_type,
					field_index: 2,
				});
				func.instruction(&I::LocalSet(2));

				// continue loop
				func.instruction(&I::Br(0));
				func.instruction(&I::End); // end loop
				func.instruction(&I::End); // end block

				// return count
				func.instruction(&I::LocalGet(1));
			});
		}

		// node_bytes(node) -> i64: the bytes of a text, 8 bytes per element otherwise
		if self.should_emit_function("node_bytes") {
			self.runtime_function("node_bytes", vec![Ref(node_ref)], vec![ValType::I64], vec![], |s, f| {
				s.emit_is_text(f);
				f.instruction(&I::If(BlockType::Result(ValType::I64)));
				f.instruction(&I::LocalGet(0));
				s.call(f, "text_byte_count");
				f.instruction(&I::Else);
				f.instruction(&I::LocalGet(0));
				s.call(f, "node_count");
				Self::emit_list(f, &[I::I64Const(8), I::I64Mul, I::End]);
			});
		}
	}

	/// string_char_at and node_index_at: the element of a text or a list at a 1-based index
	fn emit_node_indexing(&mut self) {
		let node_ref = self.node_ref(false);
		// string_char_at(node: ref $Node, index: i64) -> ref $Node
		// The character at index (1-based) of a Text/Symbol node as Codepoint node, decoding UTF-8
		if self.should_emit_function("string_char_at") {
			self.emit_string_char_at();
		}

		// node_index_at(node: ref $Node, index: i64) -> ref $Node
		// Runtime dispatch: for Text/Symbol call string_char_at, for List/Block call list_node_at
		if self.should_emit_function("node_index_at") {
			self.exported_function("node_index_at", vec![Ref(node_ref), ValType::I64], vec![Ref(node_ref)], vec![], |s, func| {
				s.emit_require_integral(func, 1);

				// a one-character text is held as a Codepoint: its only element is itself
				func.instruction(&I::LocalGet(0));
				func.instruction(&I::StructGet { struct_type_index: s.type_manager.node_type, field_index: 0 });
				func.instruction(&I::I64Const(Kind::Codepoint as i64));
				func.instruction(&I::I64Eq);
				func.instruction(&I::If(BlockType::Empty));
				Self::emit_index_compare(func, I::I64Ne);
				s.emit_fail_if(func, "index_out_of_range");
				func.instruction(&I::LocalGet(0));
				func.instruction(&I::Return);
				func.instruction(&I::End);
				s.emit_pair_part(func);

				// Get node.kind
				func.instruction(&I::LocalGet(0));
				func.instruction(&I::StructGet {
					struct_type_index: s.type_manager.node_type,
					field_index: 0, // kind field
				});

				// Check if kind is Text (3) or Symbol (5)
				// kind == 3 || kind == 5 means it's a string
				func.instruction(&I::I64Const(3)); // Kind::Text
				func.instruction(&I::I64Eq);
				func.instruction(&I::If(BlockType::Result(Ref(node_ref))));
				// It's a Text - call string_char_at
				func.instruction(&I::LocalGet(0));
				func.instruction(&I::LocalGet(1));
				s.emit_call(func, "string_char_at");
				func.instruction(&I::Else);
				// Check for Symbol
				func.instruction(&I::LocalGet(0));
				func.instruction(&I::StructGet {
					struct_type_index: s.type_manager.node_type,
					field_index: 0,
				});
				func.instruction(&I::I64Const(5)); // Kind::Symbol
				func.instruction(&I::I64Eq);
				func.instruction(&I::If(BlockType::Result(Ref(node_ref))));
				// It's a Symbol - call string_char_at
				func.instruction(&I::LocalGet(0));
				func.instruction(&I::LocalGet(1));
				s.emit_call(func, "string_char_at");
				func.instruction(&I::Else);
				// Otherwise it's a list - call list_node_at
				func.instruction(&I::LocalGet(0));
				func.instruction(&I::LocalGet(1));
				s.emit_call(func, "list_node_at");
				func.instruction(&I::End); // end inner if
				func.instruction(&I::End); // end outer if
			});
		}
	}

	/// list_insert_at(list, position, value): copies the cells before the 0-based position, puts value there and shares
	/// the rest; a position past the end (or negative) appends. Iteratively (copy_cells)
	fn emit_list_insert_at(&mut self) {
		if !self.should_emit_function(crate::analyzer::INSERT_AT_CALL) {
			return;
		}
		let node_ref = Ref(self.node_ref(false));
		let node_ref_nullable = Ref(self.node_ref(true));
		let node_type = self.type_manager.node_type;
		let params = vec![node_ref_nullable, ValType::I64, node_ref];
		self.runtime_function(crate::analyzer::INSERT_AT_CALL, params, vec![node_ref], self.copy_cells_locals(), |s, f| {
			let (list, position, value) = (0, 1, 2);
			let copy = CopyCells::at(3);
			s.emit_empty_as_null(f, list);
			s.emit_collect_cells(f, list, Some(position), &copy);
			Self::emit_list(f, &[I::I64Const(SQUARE_LIST_KIND), I::LocalGet(value), I::LocalGet(copy.cell), I::StructNew(node_type), I::LocalSet(copy.result)]);
			s.emit_rebuild_cells(f, &copy);
			Self::emit_list(f, &[I::LocalGet(copy.result), I::RefAsNonNull]);
		});
	}

	/// The grapheme at a 1-based index: a Codepoint when it is one code point (`'héllo'#2` → 'é'),
	/// else the cluster as Text sharing the bytes (`'👍🏽'#1` keeps its skin tone modifier)
	fn emit_string_char_at(&mut self) {
		let node_ref = Ref(self.node_ref(false));
		let params = vec![node_ref, ValType::I64];
		self.runtime_function("string_char_at", params, vec![node_ref], vec![ValType::I32; 3], |s, f| {
			let (start, end, stop) = (2, 3, 4);
			Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "text_grapheme_offset");
			f.instruction(&I::LocalSet(start));
			s.emit_text_bounds(f, stop, end);
			Self::emit_list(f, &[I::LocalGet(start), I::LocalGet(end)]);
			s.call(f, "grapheme_end");
			f.instruction(&I::LocalSet(stop));
			Self::emit_list(f, &[I::LocalGet(start), I::LocalGet(end)]);
			s.call(f, "utf8_next");
			Self::emit_list(f, &[I::LocalGet(stop), I::I32Eq, I::If(BlockType::Result(node_ref)), I::LocalGet(start), I::LocalGet(end)]);
			s.call(f, "utf8_decode");
			s.call(f, "new_codepoint");
			Self::emit_list(f, &[I::Else, I::LocalGet(start), I::LocalGet(stop), I::LocalGet(start), I::I32Sub]);
			s.call(f, "new_text");
			f.instruction(&I::End);
		});
	}

	/// Text is UTF-8 and is read by one of three units: `.bytes` counts bytes, `.chars` code points,
	/// `#`, `count`, `length` and `.graphemes` user-perceived characters (grapheme clusters, see GRAPHEME_EXTEND)
	fn emit_text_units(&mut self) {
		let node_ref = Ref(self.node_ref(false));
		let (int, long) = (ValType::I32, ValType::I64);

		// utf8_next(pointer, end): past the code point at pointer, never past end
		self.runtime_function("utf8_next", vec![int, int], vec![int], vec![int], |_, f| {
			let next = 2;
			Self::emit_list(f, &[
				I::LocalGet(0), I::LocalGet(0), I::I32Load8U(BYTE), I::LocalSet(next),
				I32Const(1), I32Const(2), I32Const(3), I32Const(4),
				I::LocalGet(next), I32Const(utf8::FOUR_BYTE_LEAD), I::I32LtU, I::Select,
				I::LocalGet(next), I32Const(utf8::THREE_BYTE_LEAD), I::I32LtU, I::Select,
				I::LocalGet(next), I32Const(utf8::ONE_BYTE_END), I::I32LtU, I::Select,
				I::I32Add, I::LocalTee(next),
				I::LocalGet(1), I::LocalGet(next), I::LocalGet(1), I::I32LtU, I::Select,
			]);
		});

		// utf8_decode(pointer, end): the lead byte keeps 7, 5, 4 or 3 payload bits, each continuation byte adds 6
		self.runtime_function("utf8_decode", vec![int, int], vec![int], vec![int], |_, f| {
			let codepoint = 2;
			Self::emit_list(f, &[
				I::LocalGet(0), I::I32Load8U(BYTE), I::LocalTee(codepoint), I32Const(utf8::ONE_BYTE_END), I::I32GeU, I::If(BlockType::Empty),
				I::LocalGet(codepoint), I32Const(utf8::FOUR_BYTE_PAYLOAD), I32Const(utf8::THREE_BYTE_PAYLOAD), I32Const(utf8::TWO_BYTE_PAYLOAD),
				I::LocalGet(codepoint), I32Const(utf8::THREE_BYTE_LEAD), I::I32GeU, I::Select,
				I::LocalGet(codepoint), I32Const(utf8::FOUR_BYTE_LEAD), I::I32GeU, I::Select,
				I::I32And, I::LocalSet(codepoint),
			]);
			Self::emit_skip_continuation_bytes(f, 0, 1, Some(codepoint));
			Self::emit_list(f, &[I::End, I::LocalGet(codepoint)]);
		});

		// grapheme_end(pointer, end): past the grapheme cluster starting at pointer, like strings::grapheme_clusters
		self.runtime_function("grapheme_end", vec![int, int], vec![int], vec![int; 3], |s, f| {
			let (previous, next, unpaired) = (2, 3, 4);
			Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "utf8_decode");
			Self::emit_list(f, &[I::LocalSet(previous), I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "utf8_next");
			f.instruction(&I::LocalSet(0));
			// CR LF stays together, any other control ends the cluster
			Self::emit_list(f, &[
				I::LocalGet(previous), I32Const('\r' as i32), I::I32Eq, I::LocalGet(0), I::LocalGet(1), I::I32LtU, I::I32And,
				I::If(BlockType::Empty),
				I::LocalGet(0), I::I32Load8U(BYTE), I32Const('\n' as i32), I::I32Eq,
				I::If(BlockType::Empty), I::LocalGet(0), I32Const(1), I::I32Add, I::Return, I::End,
				I::End,
			]);
			Self::emit_in_ranges(f, previous, &[(0x00, 0x1F), (0x7F, 0x9F)]);
			Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(0), I::Return, I::End]);
			Self::emit_in_ranges(f, previous, &[REGIONAL_INDICATORS]);
			Self::emit_list(f, &[
				I::LocalSet(unpaired),
				I::Block(BlockType::Empty), I::Loop(BlockType::Empty),
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
			Self::emit_list(f, &[
				I::I32And, I::I32Or, I::I32Eqz, I::BrIf(1),
				I32Const(0), I::LocalSet(unpaired),
				I::LocalGet(next), I::LocalSet(previous),
				I::LocalGet(0), I::LocalGet(1),
			]);
			s.call(f, "utf8_next");
			Self::emit_list(f, &[I::LocalSet(0), I::Br(0), I::End, I::End, I::LocalGet(0)]);
		});

		// text_grapheme_offset(text, index): address of the grapheme at the 1-based index, trapping out of range
		self.runtime_function("text_grapheme_offset", vec![node_ref, long], vec![int], vec![int; 2], |s, f| {
			let (pointer, end) = (2, 3);
			Self::emit_index_compare(f, I::I64LtS);
			s.emit_fail_if(f, "index_out_of_range");
			s.emit_text_bounds(f, pointer, end);
			Self::emit_list(f, &[
				I::Block(BlockType::Empty), I::Loop(BlockType::Empty),
				I::LocalGet(pointer), I::LocalGet(end), I::I32GeU,
			]);
			s.emit_fail_if(f, "index_out_of_range");
			Self::emit_index_compare(f, I::I64LeS);
			Self::emit_list(f, &[I::BrIf(1), I::LocalGet(pointer), I::LocalGet(end)]);
			s.call(f, "grapheme_end");
			f.instruction(&I::LocalSet(pointer));
			Self::emit_index_compare(f, I::I64Sub);
			Self::emit_list(f, &[I::LocalSet(1), I::Br(0), I::End, I::End, I::LocalGet(pointer)]);
		});

		// text_grapheme_count(text) -> i64
		self.runtime_function("text_grapheme_count", vec![node_ref], vec![long], vec![int, int, long], |s, f| {
			let (pointer, end, count) = (1, 2, 3);
			s.emit_text_bounds(f, pointer, end);
			Self::emit_list(f, &[
				I::Block(BlockType::Empty), I::Loop(BlockType::Empty),
				I::LocalGet(pointer), I::LocalGet(end), I::I32GeU, I::BrIf(1),
				I::LocalGet(pointer), I::LocalGet(end),
			]);
			s.call(f, "grapheme_end");
			Self::emit_list(f, &[
				I::LocalSet(pointer),
				I::LocalGet(count), I::I64Const(1), I::I64Add, I::LocalSet(count),
				I::Br(0), I::End, I::End, I::LocalGet(count),
			]);
		});

		// text_codepoint_count(text) -> i64: every byte but a continuation byte (10xxxxxx) starts a code point
		self.runtime_function("text_codepoint_count", vec![node_ref], vec![long], vec![int, int, long], |s, f| {
			let (pointer, end, count) = (1, 2, 3);
			s.emit_text_bounds(f, pointer, end);
			Self::emit_list(f, &[
				I::Block(BlockType::Empty), I::Loop(BlockType::Empty),
				I::LocalGet(pointer), I::LocalGet(end), I::I32GeU, I::BrIf(1),
				I::LocalGet(count),
				I::LocalGet(pointer), I::I32Load8U(BYTE), I32Const(utf8::CONTINUATION_MASK), I::I32And, I32Const(utf8::CONTINUATION_MARK), I::I32Ne,
				I::I64ExtendI32U, I::I64Add, I::LocalSet(count),
				I::LocalGet(pointer), I32Const(1), I::I32Add, I::LocalSet(pointer),
				I::Br(0), I::End, I::End, I::LocalGet(count),
			]);
		});

		// text_byte_count(text) -> i64
		self.runtime_function("text_byte_count", vec![node_ref], vec![long], vec![], |s, f| {
			s.emit_text_field(f, 0, 1);
			f.instruction(&I::I64ExtendI32U);
		});
	}

	/// Push whether code point `local` lies in one of the inclusive `ranges` (i32 0/1)
	fn emit_in_ranges(func: &mut Function, local: u32, ranges: &[(u32, u32)]) {
		for (i, &(low, high)) in ranges.iter().enumerate() {
			Self::emit_list(func, &[
				I::LocalGet(local), I32Const(low as i32), I::I32Sub, I32Const((high - low + 1) as i32), I::I32LtU,
			]);
			if i > 0 {
				func.instruction(&I::I32Or);
			}
		}
	}

	/// Locals `pointer` and `end` span the bytes of text node local 0
	pub(super) fn emit_text_bounds(&self, func: &mut Function, pointer: u32, end: u32) {
		self.emit_text_field(func, 0, 0);
		func.instruction(&I::LocalTee(pointer));
		self.emit_text_field(func, 0, 1);
		Self::emit_list(func, &[I::I32Add, I::LocalSet(end)]);
	}

	/// pointer++ past following UTF-8 continuation bytes (10xxxxxx), shifting their 6 bits into `accumulator`
	fn emit_skip_continuation_bytes(func: &mut Function, pointer: u32, end: u32, accumulator: Option<u32>) {
		func.instruction(&I::Loop(BlockType::Empty));
		func.instruction(&I::LocalGet(pointer));
		func.instruction(&I32Const(1));
		func.instruction(&I::I32Add);
		func.instruction(&I::LocalTee(pointer));
		func.instruction(&I::LocalGet(end));
		func.instruction(&I::I32LtU);
		func.instruction(&I::If(BlockType::Empty));
		func.instruction(&I::LocalGet(pointer));
		func.instruction(&I::I32Load8U(BYTE));
		func.instruction(&I32Const(utf8::CONTINUATION_MASK));
		func.instruction(&I::I32And);
		func.instruction(&I32Const(utf8::CONTINUATION_MARK));
		func.instruction(&I::I32Eq);
		func.instruction(&I::If(BlockType::Empty));
		if let Some(accumulator) = accumulator {
			func.instruction(&I::LocalGet(accumulator));
			func.instruction(&I32Const(utf8::PAYLOAD_BITS));
			func.instruction(&I::I32Shl);
			func.instruction(&I::LocalGet(pointer));
			func.instruction(&I::I32Load8U(BYTE));
			func.instruction(&I32Const(utf8::CONTINUATION_PAYLOAD));
			func.instruction(&I::I32And);
			func.instruction(&I::I32Or);
			func.instruction(&I::LocalSet(accumulator));
		}
		func.instruction(&I::Br(2));
		func.instruction(&I::End);
		func.instruction(&I::End);
		func.instruction(&I::End);
	}
}

/// Runtime errors trap inside a function of that name; eval reports the name as an error value
const INDEX_NOT_INTEGRAL: &str = "index_must_be_an_integer";
/// `y times "ab"` with y = 2.5 held in a variable (a literal fraction is a compile-time type error)
const COUNT_NOT_INTEGRAL: &str = "count_must_be_an_integer";
/// `x:int = 3; x += 0.5`: a declared int (variable or field) given a fraction at run time; a literal one is a type error
pub(super) const INT_NOT_WHOLE: &str = "an_int_must_be_a_whole_number";
/// `m.a * 2` with a text in m.a, where the kinds are known only at run time (NODE_ARITHMETIC)
pub(super) const NOT_A_NUMBER: &str = "not_a_number";
/// P65: arithmetic a text can't do on a character fails; its code point is ord(c)
pub(super) const CHARACTER_IS_NO_NUMBER: &str = "a character is not a number in arithmetic: codepoint(c) gives its code point";

/// The division in an index `n/2`, also behind the 0-based shift of `xs[n/2]` (`xs#(n/2 + 1)`)
fn divided_index(index: &Node) -> Option<(&Node, &Node)> {
	use crate::operators::Op;
	match index.drop_meta() {
		Node::Key(dividend, Op::Div, divisor) => Some((dividend, divisor)),
		Node::Key(shifted, Op::Add | Op::Sub, offset) if matches!(offset.drop_meta(), Node::Number(_)) => divided_index(shifted),
		Node::List(items, crate::node::Bracket::Round, _) if items.len() == 1 => divided_index(&items[0]),
		_ => None,
	}
}

/// How to fix a runtime error, appended to its message: the trap knows no source position, so the fix is generic
const WHOLE_NUMBER_FIX: &str = "fix: compute it with // (floor division) or `… as int`";
const RUNTIME_ERROR_FIXES: [(&str, &str); 3] = [(INDEX_NOT_INTEGRAL, WHOLE_NUMBER_FIX), (COUNT_NOT_INTEGRAL, WHOLE_NUMBER_FIX), (INT_NOT_WHOLE, WHOLE_NUMBER_FIX)];

/// The message of the runtime error trapped in the function `name`: its words, then its fix if it has one
pub fn runtime_error_message(name: &str) -> String {
	let words = name.replace('_', " ");
	match RUNTIME_ERROR_FIXES.iter().find(|(error, _)| *error == name) {
		Some((_, fix)) => format!("{words}; {fix}"),
		None => words,
	}
}

/// `return error("…")` from a function that returns numbers: the run fails, the message is the trap detail
pub const RETURNED_ERROR: &str = "returned_error";

/// map_without_cells(cells, key): what map_without builds the remaining cells with
const MAP_WITHOUT_CELLS: &str = "map_without_cells";

/// `a % 0`, `a rem 0`: an integer divide by zero (big_int::emit_nonzero_divisor)
pub const DIVIDE_BY_ZERO: &str = "divide_by_zero";

pub const RUNTIME_ERRORS: [&str; 26] = [
	"index_out_of_range", INDEX_NOT_INTEGRAL, "invalid_number", "out_of_memory", "key_not_found", "float_out_of_int_range",
	"min_of_an_empty_list", "max_of_an_empty_list", "reduce_of_an_empty_list",
	"not_a_list", "not_a_text", "not_an_int", "non_ascii_text", "not_a_joinable_item", "empty_separator", "not_an_object",
	"not_comparable", super::closures::NOT_A_FUNCTION, super::closures::WRONG_ARGUMENT_COUNT, super::tuple_emitter::WRONG_NUMBER_OF_VALUES,
	RETURNED_ERROR, "not_a_character", COUNT_NOT_INTEGRAL, NOT_A_NUMBER, DIVIDE_BY_ZERO, INT_NOT_WHOLE,
];

/// text_as_int(node) -> i64: a Text's optional sign and decimal digits, any other node's Int (get_int_value)
pub const TEXT_AS_INT: &str = "text_as_int";
/// Arithmetic on two Nodes of run-time kind: (function, its f64 instruction, the exact Int function)
pub const NODE_ADD: &str = "node_add";
pub const NODE_ARITHMETIC: [(&str, Instruction<'static>, &str); 4] = [
	(NODE_ADD, I::F64Add, "exact_add"),
	("node_sub", I::F64Sub, "exact_sub"),
	("node_mul", I::F64Mul, "exact_mul"),
	("node_div", I::F64Div, "exact_div"),
];

/// text_as_float(node): the f64 of a text like "-12.5e3" (a number node converts as it is); anything else is invalid_number
pub const TEXT_AS_FLOAT: &str = "text_as_float";
/// The educate-once topic of `"ab"*2`, whose explicit form is `2 times "ab"`
const TEXT_REPEAT_TOPIC: &str = "text-repeat";

impl WasmGcEmitter {
	/// `x as int` of a text held in a variable, like the literal `"12" as int`; no digits is the runtime error invalid_number
	pub(super) fn emit_text_as_int(&mut self) {
		if !self.should_emit_function(TEXT_AS_INT) {
			return;
		}
		let node_ref = Ref(self.node_ref(false));
		let locals = vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32, ValType::I64];
		self.runtime_function(TEXT_AS_INT, vec![node_ref], vec![ValType::I64], locals, |s, f| {
			let (pointer, end, negative, digit, value) = (1, 2, 3, 4, 5);
			let at_end = [I::LocalGet(pointer), I::LocalGet(end), I::I32GeU];
			// a one-character text is a character node: its digit, else invalid_number
			s.emit_field(f, 0, 0);
			Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::Codepoint as i64), I::I64Eq, I::If(BlockType::Empty), I::LocalGet(0)]);
			s.emit_codepoint_of_node(f);
			Self::emit_list(f, &[I::I64Const('0' as i64), I::I64Sub, I::LocalTee(value), I::I64Const(9), I::I64GtU]);
			s.emit_fail_if(f, "invalid_number");
			Self::emit_list(f, &[I::LocalGet(value), I::Return, I::End]);
			s.emit_field(f, 0, 0);
			Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::Text as i64), I::I64Ne, I::If(BlockType::Empty), I::LocalGet(0)]);
			s.call(f, "get_int_value");
			Self::emit_list(f, &[I::Return, I::End]);
			s.emit_text_bounds(f, pointer, end);
			Self::emit_list(f, &at_end);
			s.emit_fail_if(f, "invalid_number");
			// a leading sign: negative = (c == '-'), pointer skips it
			Self::emit_list(f, &[
				I::LocalGet(pointer), I::I32Load8U(BYTE), I::LocalTee(digit), I32Const('-' as i32), I::I32Eq, I::LocalSet(negative),
				I::LocalGet(negative), I::LocalGet(digit), I32Const('+' as i32), I::I32Eq, I::I32Or,
				I::LocalGet(pointer), I::I32Add, I::LocalSet(pointer),
			]);
			Self::emit_list(f, &at_end);
			s.emit_fail_if(f, "invalid_number");
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
			Self::emit_list(f, &at_end);
			Self::emit_list(f, &[I::BrIf(1), I::LocalGet(pointer), I::I32Load8U(BYTE), I32Const('0' as i32), I::I32Sub, I::LocalTee(digit), I32Const(9), I::I32GtU]);
			s.emit_fail_if(f, "invalid_number");
			// value = value*10 + digit, unbounded with the big-int runtime ("99999999999999999999" as int)
			let unbounded = s.int_runtime();
			Self::emit_list(f, &[I::LocalGet(value), I::I64Const(10)]);
			if unbounded { s.call(f, "exact_mul") } else { f.instruction(&I::I64Mul); }
			Self::emit_list(f, &[I::LocalGet(digit), I::I64ExtendI32U]);
			if unbounded { s.call(f, "exact_add") } else { f.instruction(&I::I64Add); }
			Self::emit_list(f, &[
				I::LocalSet(value),
				I::LocalGet(pointer), I32Const(1), I::I32Add, I::LocalSet(pointer), I::Br(0), I::End, I::End,
			]);
			Self::emit_list(f, &[I::I64Const(0), I::LocalGet(value)]);
			if unbounded { s.call(f, "exact_sub") } else { f.instruction(&I::I64Sub); }
			Self::emit_list(f, &[I::LocalGet(value), I::LocalGet(negative), I::Select]);
		});
	}

	/// node_add(a, b) … node_div(a, b): arithmetic on two values whose kinds are known only at run time (fields of a parsed
	/// JSON map): Floats when either is a Float, else exact Ints; anything else is the runtime error not_an_int
	pub(super) fn emit_node_arithmetic(&mut self) {
		let node_ref = Ref(self.node_ref(false));
		let (int_kind, float_kind) = (crate::type_kinds::Kind::Int as i64, crate::type_kinds::Kind::Float as i64);
		for (name, float_op, exact_op) in NODE_ARITHMETIC {
			if !self.should_emit_function(name) {
				continue;
			}
			self.runtime_function(name, vec![node_ref, node_ref], vec![node_ref], vec![ValType::I64], |s, f| {
				let kind = 2;
				// node_add of two lists (ø is the empty list): their concatenation, `out = out + row`
				if name == NODE_ADD {
					let is_list = |f: &mut Function, operand: u32| {
						s.emit_field(f, operand, 0);
						Self::emit_list(f, &[
							I::I64Const(KIND_MASK), I::I64And, I::LocalTee(kind), I::I64Const(Kind::List as i64), I::I64Eq,
							I::LocalGet(kind), I::I64Const(Kind::Block as i64), I::I64Eq, I::I32Or,
							I::LocalGet(kind), I::I64Const(Kind::Empty as i64), I::I64Eq, I::I32Or,
						]);
					};
					is_list(f, 0);
					is_list(f, 1);
					Self::emit_list(f, &[I::I32And, I::If(BlockType::Empty), I::Block(BlockType::Result(node_ref)), I::LocalGet(0), I::LocalGet(1)]);
					s.call(f, "list_concat");
					Self::emit_list(f, &[I::BrOnNonNull(0)]);
					s.call(f, "new_empty");
					Self::emit_list(f, &[I::End, I::Return, I::End]);
					// two texts (or characters): their concatenation, `await a + await b` of two text tasks
					let is_text = |f: &mut Function, operand: u32| {
						s.emit_field(f, operand, 0);
						Self::emit_list(f, &[
							I::I64Const(KIND_MASK), I::I64And, I::LocalTee(kind), I::I64Const(Kind::Text as i64), I::I64Eq,
							I::LocalGet(kind), I::I64Const(Kind::Codepoint as i64), I::I64Eq, I::I32Or,
						]);
					};
					is_text(f, 0);
					is_text(f, 1);
					Self::emit_list(f, &[I::I32And, I::If(BlockType::Empty), I::LocalGet(0), I::LocalGet(1)]);
					s.call(f, super::text_builtins::TEXT_CONCAT);
					Self::emit_list(f, &[I::Return, I::End]);
					// a text and a number: the number's text joined to it, as `1 + "x"` is "1x" when the kinds are known
					is_text(f, 0);
					is_text(f, 1);
					f.instruction(&I::I32Or);
					f.instruction(&I::If(BlockType::Empty));
					let node_type = s.type_manager.node_type;
					f.instruction(&I::I64Const(crate::type_kinds::SQUARE_LIST_KIND));
					f.instruction(&I::LocalGet(0));
					s.call(f, super::text_builtins::TEXT_OF);
					f.instruction(&I::I64Const(crate::type_kinds::SQUARE_LIST_KIND));
					f.instruction(&I::LocalGet(1));
					s.call(f, super::text_builtins::TEXT_OF);
					Self::emit_list(f, &[I::RefNull(HeapType::Concrete(node_type)), I::StructNew(node_type), I::StructNew(node_type)]);
					let (pointer, length) = s.allocate_string("");
					Self::emit_list(f, &[I::I32Const(pointer as i32), I::I32Const(length as i32)]);
					s.call(f, "new_text");
					s.call(f, super::library_ops::LIST_JOIN);
					Self::emit_list(f, &[I::Return, I::End]);
				}
				// an Error value fails with its own message (`x!` of ø is "unwrapped ø"), never as "not a number"
				let trap_detail = s.trap_detail_global();
				for operand in [0, 1] {
					s.emit_field(f, operand, 0);
					Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::Error as i64), I::I64Eq, I::If(BlockType::Empty), I::LocalGet(operand), I::GlobalSet(trap_detail)]);
					s.call(f, RETURNED_ERROR);
					f.instruction(&I::End);
				}
				// a text, a character or a list is no number here: "x" * 2 is no 240
				for operand in [0, 1] {
					s.emit_field(f, operand, 0);
					Self::emit_list(f, &[
						I::I64Const(KIND_MASK), I::I64And, I::LocalTee(kind), I::I64Const(int_kind), I::I64Ne,
						I::LocalGet(kind), I::I64Const(float_kind), I::I64Ne, I::I32And,
					]);
					s.emit_fail_if(f, NOT_A_NUMBER);
				}
				for operand in [0, 1] {
					s.emit_field(f, operand, 0);
					Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(float_kind), I::I64Eq]);
				}
				Self::emit_list(f, &[I::I32Or, I::If(BlockType::Result(node_ref)), I::LocalGet(0)]);
				s.call(f, TEXT_AS_FLOAT);
				f.instruction(&I::LocalGet(1));
				s.call(f, TEXT_AS_FLOAT);
				f.instruction(&float_op);
				s.call(f, "new_float");
				Self::emit_list(f, &[I::Else, I::LocalGet(0)]);
				s.call(f, "get_int_value");
				f.instruction(&I::LocalGet(1));
				s.call(f, "get_int_value");
				s.call(f, exact_op);
				s.call(f, "new_int");
				f.instruction(&I::End);
			});
		}
	}

	pub(super) fn emit_text_as_float(&mut self) {
		if !self.should_emit_function(TEXT_AS_FLOAT) {
			return;
		}
		let node_ref = Ref(self.node_ref(false));
		let float_box = self.type_manager.f64_box_type;
		let mut locals = vec![ValType::I32; 4];
		locals.push(ValType::F64);
		locals.extend([ValType::I32; 4]);
		self.runtime_function(TEXT_AS_FLOAT, vec![node_ref], vec![ValType::F64], locals, |s, f| {
			let (pointer, end, negative, digit, value, scale_digits, digits, exponent, exponent_negative) = (1, 2, 3, 4, 5, 6, 7, 8, 9);
			let at_end = [I::LocalGet(pointer), I::LocalGet(end), I::I32GeU];
			let byte = [I::LocalGet(pointer), I::I32Load8U(BYTE)];
			let advance = [I::LocalGet(pointer), I32Const(1), I::I32Add, I::LocalSet(pointer)];
			let kind = |f: &mut Function| {
				s.emit_field(f, 0, 0);
				Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And]);
			};
			// a number node converts as it is
			kind(f);
			Self::emit_list(f, &[I::I64Const(Kind::Float as i64), I::I64Eq, I::If(BlockType::Empty)]);
			s.emit_field(f, 0, 1);
			Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(float_box)), I::StructGet { struct_type_index: float_box, field_index: 0 }, I::Return, I::End]);
			// an exact number by its value (a ratio too), a digit character by its digit (text_as_int)
			kind(f);
			Self::emit_list(f, &[I::I64Const(Kind::Text as i64), I::I64Ne, I::If(BlockType::Empty), I::LocalGet(0)]);
			s.call(f, TEXT_AS_INT);
			if s.int_runtime() { s.call(f, "exact_to_f64") } else { f.instruction(&I::F64ConvertI64S); }
			Self::emit_list(f, &[I::Return, I::End]);
			s.emit_text_bounds(f, pointer, end);
			// a sign
			Self::emit_list(f, &at_end);
			Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty)]);
			Self::emit_list(f, &byte);
			Self::emit_list(f, &[I::LocalTee(digit), I32Const('-' as i32), I::I32Eq, I::LocalSet(negative), I::LocalGet(negative), I::LocalGet(digit), I32Const('+' as i32), I::I32Eq, I::I32Or]);
			Self::emit_list(f, &[I::LocalGet(pointer), I::I32Add, I::LocalSet(pointer), I::End]);
			// digits, with a point among them: value collects them all, scale_digits counts those after the point
			let digits_loop = |f: &mut Function, after_point: bool| {
				Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
				Self::emit_list(f, &at_end);
				f.instruction(&I::BrIf(1));
				Self::emit_list(f, &byte);
				Self::emit_list(f, &[I32Const('0' as i32), I::I32Sub, I::LocalTee(digit), I32Const(9), I::I32GtU, I::BrIf(1)]);
				Self::emit_list(f, &[I::LocalGet(value), I::F64Const(10.0.into()), I::F64Mul, I::LocalGet(digit), I::F64ConvertI32U, I::F64Add, I::LocalSet(value)]);
				Self::emit_list(f, &[I::LocalGet(digits), I32Const(1), I::I32Add, I::LocalSet(digits)]);
				if after_point {
					Self::emit_list(f, &[I::LocalGet(scale_digits), I32Const(1), I::I32Add, I::LocalSet(scale_digits)]);
				}
				Self::emit_list(f, &advance);
				Self::emit_list(f, &[I::Br(0), I::End, I::End]);
			};
			digits_loop(f, false);
			Self::emit_list(f, &at_end);
			Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty)]);
			Self::emit_list(f, &byte);
			Self::emit_list(f, &[I32Const('.' as i32), I::I32Eq, I::If(BlockType::Empty)]);
			Self::emit_list(f, &advance);
			digits_loop(f, true);
			Self::emit_list(f, &[I::End, I::End]);
			Self::emit_list(f, &[I::LocalGet(digits), I::I32Eqz]);
			s.emit_fail_if(f, "invalid_number");
			// an exponent: e or E, a sign, digits
			Self::emit_list(f, &at_end);
			Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty)]);
			Self::emit_list(f, &byte);
			Self::emit_list(f, &[I32Const(ASCII_LOWERCASE_BIT), I::I32Or, I32Const('e' as i32), I::I32Eq, I::If(BlockType::Empty)]);
			Self::emit_list(f, &advance);
			Self::emit_list(f, &at_end);
			s.emit_fail_if(f, "invalid_number");
			Self::emit_list(f, &byte);
			Self::emit_list(f, &[I::LocalTee(digit), I32Const('-' as i32), I::I32Eq, I::LocalSet(exponent_negative), I::LocalGet(exponent_negative), I::LocalGet(digit), I32Const('+' as i32), I::I32Eq, I::I32Or]);
			Self::emit_list(f, &[I::LocalGet(pointer), I::I32Add, I::LocalSet(pointer), I32Const(0), I::LocalSet(digits)]);
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
			Self::emit_list(f, &at_end);
			f.instruction(&I::BrIf(1));
			Self::emit_list(f, &byte);
			Self::emit_list(f, &[I32Const('0' as i32), I::I32Sub, I::LocalTee(digit), I32Const(9), I::I32GtU, I::BrIf(1)]);
			Self::emit_list(f, &[I::LocalGet(exponent), I32Const(10), I::I32Mul, I::LocalGet(digit), I::I32Add, I::LocalSet(exponent), I::LocalGet(digits), I32Const(1), I::I32Add, I::LocalSet(digits)]);
			Self::emit_list(f, &advance);
			Self::emit_list(f, &[I::Br(0), I::End, I::End]);
			Self::emit_list(f, &[I::LocalGet(digits), I::I32Eqz]);
			s.emit_fail_if(f, "invalid_number");
			Self::emit_list(f, &[I::End, I::End]);
			// nothing may follow
			Self::emit_list(f, &at_end);
			Self::emit_list(f, &[I::I32Eqz]);
			s.emit_fail_if(f, "invalid_number");
			// the power of ten: the written exponent less the digits after the point, applied by repeated * or / 10
			Self::emit_list(f, &[I32Const(0), I::LocalGet(exponent), I::I32Sub, I::LocalGet(exponent), I::LocalGet(exponent_negative), I::Select]);
			Self::emit_list(f, &[I::LocalGet(scale_digits), I::I32Sub, I::LocalSet(exponent)]);
			for (more, step, towards_zero) in [(I::I32GtS, I::F64Mul, I::I32Sub), (I::I32LtS, I::F64Div, I::I32Add)] {
				Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(exponent), I32Const(0), more, I::I32Eqz, I::BrIf(1)]);
				Self::emit_list(f, &[I::LocalGet(value), I::F64Const(10.0.into()), step, I::LocalSet(value)]);
				Self::emit_list(f, &[I::LocalGet(exponent), I32Const(1), towards_zero, I::LocalSet(exponent), I::Br(0), I::End, I::End]);
			}
			Self::emit_list(f, &[I::F64Const(0.0.into()), I::LocalGet(value), I::F64Sub, I::LocalGet(value), I::LocalGet(negative), I::Select]);
		});
	}

	/// zero_fill(count, zero): a square list of `count` times the node `zero`, ø when count is not positive
	fn emit_zero_fill(&mut self) {
		if !self.should_emit_function(crate::analyzer::ZERO_FILL_CALL) {
			return;
		}
		let node = self.type_manager.node_type;
		let node_ref = Ref(self.node_ref(false));
		let nullable_node_ref = Ref(self.node_ref(true));
		self.runtime_function(crate::analyzer::ZERO_FILL_CALL, vec![ValType::I64, node_ref], vec![node_ref], vec![nullable_node_ref], |s, f| {
			let rest = 2;
			s.emit_require_whole(f, 0, COUNT_NOT_INTEGRAL);
			Self::emit_list(f, &[
				I::RefNull(HeapType::Concrete(node)), I::LocalSet(rest),
				I::Block(BlockType::Empty), I::Loop(BlockType::Empty),
				I::LocalGet(0), I::I64Const(0), I::I64LeS, I::BrIf(1),
				I::I64Const(SQUARE_LIST_KIND), I::LocalGet(1), I::LocalGet(rest), I::StructNew(node), I::LocalSet(rest),
				I::LocalGet(0), I::I64Const(1), I::I64Sub, I::LocalSet(0),
				I::Br(0), I::End, I::End,
				I::LocalGet(rest), I::RefIsNull, I::If(BlockType::Result(node_ref)),
			]);
			s.call(f, "new_empty");
			Self::emit_list(f, &[I::Else, I::LocalGet(rest), I::RefAsNonNull, I::End]);
		});
	}

	/// The runtime error functions in id order: the fixed ones, a missing switch case per label, the fixed-width overflows
	pub(super) fn runtime_error_names(&self) -> Vec<&'static str> {
		let no_case_errors = self.ctx.missing_case_labels.iter().map(|label| &*Box::leak(format!("{}{label}", crate::switch::NO_CASE_PREFIX).into_boxed_str()));
		let overflow_errors = crate::fixed_width::FIXED_WIDTHS.iter().map(|width| width.trap);
		RUNTIME_ERRORS.into_iter().chain(no_case_errors).chain(overflow_errors).collect()
	}

	pub(super) fn emit_runtime_errors(&mut self) {
		if self.guards_errors {
			self.declare_error_catching(); // the error functions throw while a `try` runs
		}
		for (error_id, name) in self.runtime_error_names().into_iter().enumerate() {
			self.runtime_function(name, vec![], vec![], vec![], |s, f| s.emit_error_body(f, error_id as i32));
		}
	}

	/// Unconditional runtime error; the stack after it is unreachable, so it fits any expected type
	pub(super) fn emit_runtime_error(&mut self, func: &mut Function, error: &'static str) {
		self.emit_call(func, error);
		func.instruction(&I::Unreachable);
	}

	/// Trap when the i32 condition on the stack is true
	pub(super) fn emit_fail_if(&self, func: &mut Function, error: &'static str) {
		func.instruction(&I::If(BlockType::Empty));
		self.call(func, error);
		func.instruction(&I::End);
	}

	/// An index held in a variable (`m = n/2; xs[m]`, `xs[0:m]`) traps at run time when the Int in i64 local `index` is a
	/// ratio, as `xs[n/2]` does: a value outside the fixnum range is a handle into the number heap (big_int.rs)
	pub(super) fn emit_require_integral(&self, func: &mut Function, index: u32) {
		self.emit_require_whole(func, index, INDEX_NOT_INTEGRAL);
	}

	/// Trap with `error` when the Int in i64 local `local` is a ratio
	pub(super) fn emit_require_whole(&self, func: &mut Function, local: u32, error: &'static str) {
		if !self.int_runtime() {
			return; // without the big-int runtime an Int is always a plain i64
		}
		let fixnum_offset = super::big_int::FIXNUM_OFFSET;
		Self::emit_list(func, &[I::LocalGet(local), I::I64Const(fixnum_offset), I::I64Add, I::I64Const(0), I::I64LtS, I::If(BlockType::Empty)]);
		self.emit_heap_get(func, local);
		func.instruction(&I::RefTestNullable(HeapType::Concrete(self.type_manager.ratio_type)));
		self.emit_fail_if(func, error);
		func.instruction(&I::End);
	}

	/// `xs[n/2]`: an index that divides traps unless the division is exact; floor division (`n//2`) or `n/2 as int` is hinted
	pub(super) fn emit_integral_index_check(&mut self, func: &mut Function, index: &Node) {
		let Some((dividend, divisor)) = divided_index(index) else { return };
		let (dividend_text, divisor_text) = (crate::normalize::operand_text(dividend), crate::normalize::operand_text(divisor));
		crate::normalize::set_position_of(index);
		crate::normalize::hint(&format!("{dividend_text}/{divisor_text}"), &format!("{dividend_text}//{divisor_text}"), "an index must be an integer: floor division, or `as int`");
		let remainder = Node::Key(Box::new(dividend.clone()), crate::operators::Op::Mod, Box::new(divisor.clone()));
		self.emit_numeric_value(func, &remainder);
		func.instruction(&I::I64Const(0));
		func.instruction(&I::I64Ne);
		self.emit_fail_if(func, INDEX_NOT_INTEGRAL);
	}

	fn emit_index_compare(func: &mut Function, compare: Instruction) {
		func.instruction(&I::LocalGet(1));
		func.instruction(&I::I64Const(1));
		func.instruction(&compare);
	}

	pub(super) fn emit_field(&self, func: &mut Function, local: u32, field_index: u32) {
		func.instruction(&I::LocalGet(local));
		func.instruction(&I::StructGet { struct_type_index: self.type_manager.node_type, field_index });
	}

	/// Push field `field_index` (0 ptr, 1 len) of the $String inside text node `local`
	pub(super) fn emit_text_field(&self, func: &mut Function, local: u32, field_index: u32) {
		self.emit_field(func, local, 1);
		func.instruction(&I::RefCastNonNull(HeapType::Concrete(self.type_manager.string_type)));
		func.instruction(&I::StructGet { struct_type_index: self.type_manager.string_type, field_index });
	}

	/// Walk list local 0 to the element at 1-based index local 1 into `current`; trap when out of range
	fn emit_list_walk(&self, func: &mut Function, current: u32) {
		// a number, a character or a text held where a list is indexed (`x=3; x#1`, a parameter): not_a_list, not a cast trap
		for scalar in [Kind::Int, Kind::Float, Kind::Codepoint, Kind::Text, Kind::Symbol] {
			self.emit_field(func, 0, 0);
			Self::emit_list(func, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(scalar as i64), I::I64Eq]);
			self.emit_fail_if(func, "not_a_list");
		}
		// ø is the empty list: it has no item at any index
		self.emit_field(func, 0, 0);
		Self::emit_list(func, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::Empty as i64), I::I64Eq]);
		self.emit_fail_if(func, "index_out_of_range");
		Self::emit_index_compare(func, I::I64LtS);
		self.emit_fail_if(func, "index_out_of_range");
		func.instruction(&I::LocalGet(0));
		func.instruction(&I::LocalSet(current));
		func.instruction(&I::Block(BlockType::Empty));
		func.instruction(&I::Loop(BlockType::Empty));
		func.instruction(&I::LocalGet(current));
		func.instruction(&I::RefIsNull);
		self.emit_fail_if(func, "index_out_of_range");
		Self::emit_index_compare(func, I::I64LeS);
		func.instruction(&I::BrIf(1));
		self.emit_field(func, current, 2);
		func.instruction(&I::LocalSet(current));
		Self::emit_index_compare(func, I::I64Sub);
		func.instruction(&I::LocalSet(1));
		func.instruction(&I::Br(0));
		func.instruction(&I::End);
		func.instruction(&I::End);
		// meta entries `@name:value` sit behind every field (meta_entries.rs, field_with): arriving at one is past the end
		self.emit_field(func, current, 1);
		self.call(func, super::equality::IS_META_ENTRY);
		self.emit_fail_if(func, "index_out_of_range");
	}

	/// The text heap is exported: the host allocates the texts it returns (`read`, `fetch`) from it too
	pub(super) fn emit_text_heap_global(&mut self) {
		if self.text_heap_global.is_some() {
			return;
		}
		self.globals.global(GlobalType { val_type: ValType::I32, mutable: true, shared: false }, &ConstExpr::i32_const(0));
		self.text_heap_global = Some(self.next_global_idx);
		self.exports.export(crate::host::TEXT_HEAP_EXPORT, ExportKind::Global, self.next_global_idx);
		self.next_global_idx += 1;
	}

	/// Bump-allocate `length` bytes into local `address`; fresh pages come from memory.grow,
	/// so runtime texts never overlap the string table at the start of memory
	pub(super) fn emit_text_allocation(&self, func: &mut Function, length: u32, address: u32) {
		const PAGE_BITS: i32 = 16;
		let heap = self.text_heap_global.expect("emit_text_heap_global before allocating texts");
		func.instruction(&I::GlobalGet(heap));
		func.instruction(&I::I32Eqz);
		func.instruction(&I::GlobalGet(heap));
		func.instruction(&I::LocalGet(length));
		func.instruction(&I::I32Add);
		func.instruction(&I::MemorySize(0));
		func.instruction(&I32Const(PAGE_BITS));
		func.instruction(&I::I32Shl);
		func.instruction(&I::I32GtU);
		func.instruction(&I::I32Or);
		func.instruction(&I::If(BlockType::Empty));
		func.instruction(&I::LocalGet(length));
		func.instruction(&I32Const(PAGE_BITS));
		func.instruction(&I::I32ShrU);
		func.instruction(&I32Const(1));
		func.instruction(&I::I32Add);
		func.instruction(&I::MemoryGrow(0));
		func.instruction(&I::LocalTee(address));
		func.instruction(&I32Const(-1));
		func.instruction(&I::I32Eq);
		self.emit_fail_if(func, "out_of_memory");
		func.instruction(&I::LocalGet(address));
		func.instruction(&I32Const(PAGE_BITS));
		func.instruction(&I::I32Shl);
		func.instruction(&I::GlobalSet(heap));
		func.instruction(&I::End);
		func.instruction(&I::GlobalGet(heap));
		func.instruction(&I::LocalTee(address));
		func.instruction(&I::LocalGet(length));
		func.instruction(&I::I32Add);
		func.instruction(&I::GlobalSet(heap));
	}

	/// Grow the memory, when needed, until `length` more bytes fit after the heap global (new pages come at the end)
	pub(super) fn emit_memory_room(&self, func: &mut Function, heap: u32, length: u32) {
		const PAGE_BITS: i32 = 16;
		Self::emit_list(func, &[I::GlobalGet(heap), I::LocalGet(length), I::I32Add, I::MemorySize(0), I32Const(PAGE_BITS), I::I32Shl, I::I32GtU, I::If(BlockType::Empty)]);
		Self::emit_list(func, &[I::GlobalGet(heap), I::LocalGet(length), I::I32Add, I::MemorySize(0), I32Const(PAGE_BITS), I::I32Shl, I::I32Sub]);
		Self::emit_list(func, &[I32Const(PAGE_BITS), I::I32ShrU, I32Const(1), I::I32Add, I::MemoryGrow(0), I32Const(-1), I::I32Eq]);
		self.emit_fail_if(func, "out_of_memory");
		func.instruction(&I::End);
	}

	pub(super) fn emit_is_text(&self, func: &mut Function) {
		for kind in [Kind::Text, Kind::Symbol] {
			self.emit_field(func, 0, 0);
			func.instruction(&I::I64Const(kind as i64));
			func.instruction(&I::I64Eq);
		}
		func.instruction(&I::I32Or);
	}

	/// callee(node, index, value) with the caller's three arguments, the value through `convert` when given
	fn emit_forward_arguments(&mut self, func: &mut Function, callee: u32, convert: Option<&'static str>) {
		for local in 0..3 {
			func.instruction(&I::LocalGet(local));
		}
		if let Some(convert) = convert {
			self.call(func, convert);
		}
		func.instruction(&I::Call(callee));
	}

	/// `target#index = value` leaves the assigned value (i64) on the stack; a variable target gets the updated copy
	pub(super) fn emit_index_assignment(&mut self, func: &mut Function, target: &Node, index: &Node, value: &Node) {
		if let Node::Symbol(name) = target.drop_meta() {
			if self.is_unbound(name) {
				return self.emit_undefined_variable(func, name);
			}
		}
		if self.emit_struct_field_set(func, target, index, value) {
			return;
		}
		if let (Some(slot), Some(key)) = (self.typed_map(target), self.map_key(index)) {
			self.emit_typed_map_set(func, slot, &key, value);
			return self.emit_assigned_entry_value(func, target, &key, value);
		}
		if let Some(key) = self.map_key(index) {
			return self.emit_entry_assignment(func, target, &key, value, crate::library_words::FIELD_WITH);
		}
		if let Some(key) = self.dynamic_key(index) {
			return self.emit_entry_assignment(func, target, &key, value, NODE_WITH_KEY);
		}
		if self.emit_typed_element_assignment(func, target, index, value) {
			return;
		}
		// a value that is no exact Int (a float, a text, a list) is stored as its Node and leaves 0, as an entry
		// assignment of a non-number does
		if !self.is_exact_int_element(target, value) {
			self.emit_node_instructions(func, target);
			self.emit_numeric_value(func, index);
			self.emit_node_instructions(func, value);
			self.emit_call(func, NODE_WITH_NODE_AT);
			self.emit_store_updated(func, target);
			func.instruction(&I::I64Const(0));
			return;
		}
		let assigned = self.scratch(2);
		self.emit_node_instructions(func, target);
		self.emit_numeric_value(func, index);
		self.emit_numeric_value(func, value);
		func.instruction(&I::LocalTee(assigned));
		self.emit_call(func, "node_with_at");
		self.emit_store_updated(func, target);
		func.instruction(&I::LocalGet(assigned));
	}

	/// An element value node_with_at stores as given: an Int, or a character in anything but a list (a text's element)
	pub(super) fn is_exact_int_element(&self, target: &Node, value: &Node) -> bool {
		match self.get_type(value) {
			Kind::Int => true,
			Kind::Codepoint => self.get_type(target) != Kind::List,
			_ => false,
		}
	}

	/// `left = value` with `left` an element `target#index`, valued as `read` emits it: the element read back after the
	/// assignment, so a float or text keeps its value (emit_index_assignment leaves an Int)
	pub(super) fn emit_index_assignment_read_back(&mut self, func: &mut Function, left: &Node, value: &Node, read: fn(&mut Self, &mut Function, &Node)) {
		let Node::Key(target, crate::operators::Op::Hash, index) = left.drop_meta() else { unreachable!("an element target") };
		self.emit_index_assignment(func, target, index, value);
		func.instruction(&I::Drop);
		read(self, func, left);
	}

	/// A variable target (local or declared global) gets the updated copy on the stack, any other target drops it
	fn emit_store_updated(&mut self, func: &mut Function, target: &Node) {
		let Node::Symbol(name) = target.drop_meta() else {
			func.instruction(&I::Drop);
			return;
		};
		let global = self.ctx.user_globals.get(name).filter(|(_, kind)| kind.is_ref());
		match (self.scope.lookup(name), global) {
			(Some(local), _) if local.kind.is_ref() => func.instruction(&I::LocalSet(local.position)),
			(None, Some(&(index, _))) => func.instruction(&I::GlobalSet(index)),
			_ => func.instruction(&I::Drop),
		};
	}

	/// `d[k] = v` with a key read at runtime (`for k in ks {d[k] = 0}`): the copy that `setter` makes with the entry set;
	/// leaves the number of the value (0 for a value that is no number), as `xs[i] = v` does
	fn emit_entry_assignment(&mut self, func: &mut Function, target: &Node, key: &Node, value: &Node, setter: &'static str) {
		self.emit_node_instructions(func, target);
		self.emit_node_instructions(func, key);
		self.emit_node_instructions(func, value);
		self.emit_call(func, setter);
		self.emit_store_updated(func, target);
		self.emit_assigned_entry_value(func, target, key, value);
	}

	/// The value of an entry assignment (i64): an Int value read back from the entry, else 0
	pub(super) fn emit_assigned_entry_value(&mut self, func: &mut Function, target: &Node, key: &Node, value: &Node) {
		if self.get_type(value) != Kind::Int {
			func.instruction(&I::I64Const(0));
			return;
		}
		let one_based_key = Node::Key(Box::new(key.clone()), crate::operators::Op::Add, Box::new(Node::Number(crate::extensions::numbers::Number::Int(1))));
		self.emit_indexed_node(func, target, &one_based_key);
		self.emit_call(func, "get_int_value");
	}

	/// Arithmetic without an implicit conversion: `"5"+3` and `[1 2]*2` are type errors, `list + list` concatenates.
	/// Returns true when it emitted the whole expression (a Node, or a trap after a recorded type error).
	pub(super) fn emit_typed_arithmetic(&mut self, func: &mut Function, left: &Node, op: &crate::operators::Op, right: &Node) -> bool {
		match self.arithmetic_type(left, op, right) {
			Kind::Text if *op == crate::operators::Op::Mul => self.emit_arithmetic_type_error(func, left, op, right, Kind::Text),
			Kind::Text => {
				self.emit_text_concat(func, left, right);
				true
			}
			Kind::List => {
				self.emit_node_instructions(func, left);
				self.emit_node_instructions(func, right);
				self.emit_call(func, "list_concat");
				func.instruction(&I::RefAsNonNull);
				true
			}
			kind => self.emit_arithmetic_type_error(func, left, op, right, kind),
		}
	}

	/// `n times text` and `text * n`: the text n times, joined
	pub(super) fn emit_text_repeat(&mut self, func: &mut Function, text: &Node, count: &Node) {
		let copies = crate::analyzer::filled_list(count.clone(), &Node::List(vec![text.clone()], crate::node::Bracket::Square, crate::node::Separator::Space));
		self.emit_node_instructions(func, &super::join_call(copies.expect("a one-element list"), ""));
	}

	/// `"ab"*2` repeats; the got-it warning names `2 times "ab"`, and for a text spelling a number how to multiply it
	fn educate_text_multiplication(written: &str, text: &Node, count: &Node) {
		// a one-character text parses as a codepoint, but `'5'` would read as a code point (decision #35): keep it a text
		let text_written = match text.drop_meta() {
			Node::Char(character) => format!("\"{character}\""),
			_ => crate::normalize::operand_text(text),
		};
		let count_written = crate::normalize::operand_text(count);
		let spells_a_number = match text.drop_meta() {
			Node::Text(digits) => digits.trim().parse::<f64>().is_ok(),
			Node::Char(digit) => digit.is_ascii_digit(),
			_ => false,
		};
		let reason = if spells_a_number {
			format!("got it: the text repeated; to multiply the number it spells: int({text_written})*{count_written}")
		} else {
			"got it: the text repeated".to_string()
		};
		crate::normalize::set_position_of(count);
		crate::diagnostic::educate_once(TEXT_REPEAT_TOPIC, written, &format!("{count_written} times {text_written}"), &reason);
	}

	/// In a numeric context any collection or text operand is a type error
	pub(super) fn emit_arithmetic_type_error(&mut self, func: &mut Function, left: &Node, op: &crate::operators::Op, right: &Node, kind: Kind) -> bool {
		if !matches!(kind, Kind::Error | Kind::List | Kind::Text) {
			return false;
		}
		// `"clicks " + cont` of an undefined cont: the undefined variable at the name, as in number arithmetic
		let unbound = [left, right].into_iter().find_map(|operand| match operand.drop_meta() {
			Node::Symbol(name) if self.is_unbound(name) => Some((operand, name.clone())),
			_ => None,
		});
		if let Some((operand, name)) = unbound {
			self.note_position(operand);
			self.emit_undefined_variable(func, &name);
			return true;
		}
		let (left_kind, right_kind) = (self.get_type(left), self.get_type(right));
		if crate::analyzer::repeats_text(left_kind, op, right_kind) {
			let (text, count) = if right_kind == Kind::Int { (left, right) } else { (right, left) };
			let fractional = matches!(count.drop_meta(), Node::Number(number) if f64::from(*number).fract() != 0.0);
			if !fractional {
				let written = format!("{}*{}", crate::normalize::operand_text(left), crate::normalize::operand_text(right));
				Self::educate_text_multiplication(&written, text, count);
				self.emit_text_repeat(func, text, count);
				return true;
			}
		}
		let is_number = |kind: Kind| matches!(kind, Kind::Int | Kind::Float);
		let repeats_or_scales = *op == crate::operators::Op::Mul && ((left_kind == Kind::List && is_number(right_kind)) || (is_number(left_kind) && right_kind == Kind::List));
		let repeats_text = match (left_kind, right_kind) {
			(Kind::Text, kind) if is_number(kind) => Some((left, right)),
			(kind, Kind::Text) if is_number(kind) => Some((right, left)),
			_ => None,
		}.filter(|_| *op == crate::operators::Op::Mul);
		let fix = if let Some((text, count)) = repeats_text {
			format!("a text repeats a whole number of times: `{} times {}`", crate::normalize::operand_text(count), crate::normalize::operand_text(text))
		} else if repeats_or_scales {
			"ambiguous: Python repeats the list, NumPy multiplies each element; write `n times [x]` to repeat, or map to multiply".into()
		} else if left_kind == Kind::Codepoint || right_kind == Kind::Codepoint {
			CHARACTER_IS_NO_NUMBER.into()
		} else if left_kind == Kind::List || right_kind == Kind::List {
			"lists only concatenate with lists (+), element-wise arithmetic needs an explicit map".into()
		} else {
			"no implicit conversion, convert explicitly, e.g. int(\"5\") + 3".into()
		};
		self.emit_type_error(func, format!("type error: {left_kind} {op} {right_kind}: {fix}"));
		true
	}

	pub(super) fn arithmetic_type(&self, left: &Node, op: &crate::operators::Op, right: &Node) -> Kind {
		crate::analyzer::arithmetic_kind_of_operands(self.get_type(left), op, self.get_type(right), right)
	}

	/// ø (an Empty node) in local `list` becomes null, the end of a cons list
	pub(super) fn emit_empty_as_null(&self, func: &mut Function, list: u32) {
		func.instruction(&I::LocalGet(list));
		func.instruction(&I::RefIsNull);
		func.instruction(&I::If(BlockType::Empty));
		func.instruction(&I::Else);
		self.emit_field(func, list, 0);
		func.instruction(&I::I64Const(Kind::Empty as i64));
		func.instruction(&I::I64Eq);
		func.instruction(&I::If(BlockType::Empty));
		func.instruction(&I::RefNull(HeapType::Concrete(self.type_manager.node_type)));
		func.instruction(&I::LocalSet(list));
		func.instruction(&I::End);
		func.instruction(&I::End);
	}

	/// list_concat(a, b): copies the cells of a in front of b, which is shared; iteratively (copy_cells)
	fn emit_list_concat(&mut self) {
		if !self.should_emit_function("list_concat") {
			return;
		}
		let node_ref_nullable = Ref(self.node_ref(true));
		let params = vec![node_ref_nullable, node_ref_nullable];
		self.runtime_function("list_concat", params, vec![node_ref_nullable], self.copy_cells_locals(), |s, f| {
			let (first, second) = (0, 1);
			let copy = CopyCells::at(2);
			for list in [first, second] {
				s.emit_empty_as_null(f, list);
			}
			s.emit_collect_cells(f, first, None, &copy);
			Self::emit_list(f, &[I::LocalGet(second), I::LocalSet(copy.result)]);
			s.emit_rebuild_cells(f, &copy);
			f.instruction(&I::LocalGet(copy.result));
		});
	}

	/// The locals copy_cells needs, in CopyCells order
	pub(super) fn copy_cells_locals(&self) -> Vec<ValType> {
		let node_ref_nullable = Ref(self.node_ref(true));
		let cells = Ref(RefType { nullable: true, heap_type: HeapType::Concrete(self.type_manager.node_array_type) });
		vec![node_ref_nullable, cells, ValType::I32, node_ref_nullable, node_ref_nullable]
	}

	/// The first cells of `list` (all, or as many as the i64 local `limit` says; a negative limit is all) into an array,
	/// counted, then filled: no recursion, so a long list never exhausts the call stack. `cell` ends at the first cell
	/// not taken
	pub(super) fn emit_collect_cells(&self, f: &mut Function, list: u32, limit: Option<u32>, copy: &CopyCells) {
		let node_array = self.type_manager.node_array_type;
		let reached_limit = |f: &mut Function| if let Some(limit) = limit {
			Self::emit_list(f, &[I::LocalGet(copy.position), I::I64ExtendI32U, I::LocalGet(limit), I::I64Eq, I::BrIf(1)]);
		};
		let next_cell = |s: &Self, f: &mut Function| {
			s.emit_field(f, copy.cell, 2);
			Self::emit_list(f, &[I::LocalSet(copy.cell), I::LocalGet(copy.position), I::I32Const(1), I::I32Add, I::LocalSet(copy.position), I::Br(0), I::End, I::End]);
		};
		Self::emit_list(f, &[I::LocalGet(list), I::LocalSet(copy.cell), I::I32Const(0), I::LocalSet(copy.position),
			I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(copy.cell), I::RefIsNull, I::BrIf(1)]);
		reached_limit(f);
		next_cell(self, f);
		Self::emit_list(f, &[I::LocalGet(copy.position), I::ArrayNewDefault(node_array), I::LocalSet(copy.cells),
			I::LocalGet(list), I::LocalSet(copy.cell), I::I32Const(0), I::LocalSet(copy.position),
			I::Block(BlockType::Empty), I::Loop(BlockType::Empty),
			I::LocalGet(copy.position), I::LocalGet(copy.cells), I::ArrayLen, I::I32GeU, I::BrIf(1),
			I::LocalGet(copy.cells), I::LocalGet(copy.position), I::LocalGet(copy.cell), I::ArraySet(node_array)]);
		next_cell(self, f);
	}

	/// The collected cells, from the last back, copied in front of `result`, each sharing what follows it
	pub(super) fn emit_rebuild_cells(&self, f: &mut Function, copy: &CopyCells) {
		let node_array = self.type_manager.node_array_type;
		Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty),
			I::LocalGet(copy.position), I::I32Eqz, I::BrIf(1),
			I::LocalGet(copy.position), I::I32Const(1), I::I32Sub, I::LocalSet(copy.position),
			I::LocalGet(copy.cells), I::LocalGet(copy.position), I::ArrayGet(node_array), I::LocalSet(copy.copied)]);
		self.emit_field(f, copy.copied, 0);
		self.emit_field(f, copy.copied, 1);
		Self::emit_list(f, &[I::LocalGet(copy.result), I::StructNew(self.type_manager.node_type), I::LocalSet(copy.result), I::Br(0), I::End, I::End]);
	}

	/// `x#i += v` → `x#i = x#i + v`, leaving the assigned value (i64) on the stack
	pub(super) fn emit_compound_index_assignment(&mut self, func: &mut Function, left: &Node, op: &crate::operators::Op, right: &Node) -> bool {
		let Node::Key(target, crate::operators::Op::Hash, index) = left.drop_meta() else {
			return false;
		};
		let updated = Node::Key(Box::new(left.clone()), op.base_op(), Box::new(right.clone()));
		self.emit_index_assignment(func, target, index, &updated);
		true
	}

	/// Index assignment builds a new value instead of mutating a shared one (value semantics):
	/// `y#i=v` stores `node_with_at(y, i, v)` back into `y`, so aliases and deduplicated literals never change.
	fn emit_with_at_functions(&mut self) {
		if !self.should_emit_function("node_with_at") {
			return;
		}
		self.emit_text_heap_global();
		let node_ref = Ref(self.node_ref(false));
		let node_ref_nullable = Ref(self.node_ref(true));
		let node_type = self.type_manager.node_type;
		let string_type = self.type_manager.string_type;
		let next_index = |s: &Self| s.ctx.func_registry.import_count() + s.ctx.func_registry.code_count();

		// list_with_at(list, index, value node): copies the cells up to index, shares the rest; iterative (the cells before
		// index into an array, rebuilt backwards), so a deep index never exhausts the call stack
		let list_with_at = next_index(self);
		let params = vec![node_ref_nullable, ValType::I64, node_ref];
		let node_array = self.type_manager.node_array_type;
		let cells_ref = Ref(RefType { nullable: true, heap_type: HeapType::Concrete(node_array) });
		let locals = vec![node_ref_nullable, ValType::I64, cells_ref, ValType::I32, node_ref_nullable];
		self.runtime_function("list_with_at", params, vec![node_ref], locals, |s, f| {
			let (list, index, value, cell, countdown, cells, position, result) = (0, 1, 2, 3, 4, 5, 6, 7);
			let kind_test = |s: &Self, f: &mut Function| {
				// a missing cell or ø: lists do not grow by index, the empty list ø included (user, 2026-10-04)
				Self::emit_list(f, &[I::LocalGet(cell), I::RefIsNull]);
				s.emit_fail_if(f, "index_out_of_range");
				s.emit_field(f, cell, 0);
				Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::Empty as i64), I::I64Eq]);
				s.emit_fail_if(f, "index_out_of_range");
			};
			Self::emit_index_compare(f, I::I64LtS);
			s.emit_fail_if(f, "index_out_of_range");
			// the cell at index
			Self::emit_list(f, &[I::LocalGet(list), I::LocalSet(cell), I::LocalGet(index), I::LocalSet(countdown), I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
			kind_test(s, f);
			Self::emit_list(f, &[I::LocalGet(countdown), I::I64Const(1), I::I64Eq, I::BrIf(1)]);
			s.emit_field(f, cell, 2);
			Self::emit_list(f, &[I::LocalSet(cell), I::LocalGet(countdown), I::I64Const(1), I::I64Sub, I::LocalSet(countdown), I::Br(0), I::End, I::End]);
			// it, with the value
			s.emit_field(f, cell, 0);
			Self::emit_list(f, &[I::LocalGet(value)]);
			s.emit_field(f, cell, 2);
			Self::emit_list(f, &[I::StructNew(node_type), I::LocalSet(result)]);
			// the cells before it, in order
			Self::emit_list(f, &[
				I::LocalGet(index), I::I64Const(1), I::I64Sub, I::I32WrapI64, I::ArrayNewDefault(node_array), I::LocalSet(cells),
				I::LocalGet(list), I::LocalSet(cell), I::I32Const(0), I::LocalSet(position),
				I::Block(BlockType::Empty), I::Loop(BlockType::Empty),
				I::LocalGet(position), I::LocalGet(cells), I::ArrayLen, I::I32GeU, I::BrIf(1),
				I::LocalGet(cells), I::LocalGet(position), I::LocalGet(cell), I::ArraySet(node_array),
			]);
			s.emit_field(f, cell, 2);
			Self::emit_list(f, &[I::LocalSet(cell), I::LocalGet(position), I::I32Const(1), I::I32Add, I::LocalSet(position), I::Br(0), I::End, I::End]);
			// rebuilt from the last one back, each sharing what follows
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty),
				I::LocalGet(position), I::I32Eqz, I::BrIf(1),
				I::LocalGet(position), I::I32Const(1), I::I32Sub, I::LocalSet(position),
				I::LocalGet(cells), I::LocalGet(position), I::ArrayGet(node_array), I::LocalSet(cell)]);
			s.emit_field(f, cell, 0);
			s.emit_field(f, cell, 1);
			Self::emit_list(f, &[I::LocalGet(result), I::StructNew(node_type), I::LocalSet(result), I::Br(0), I::End, I::End,
				I::LocalGet(result), I::RefAsNonNull]);
		});
		assert_eq!(self.func_index("list_with_at"), list_with_at, "node_with_at forwards to this index");

		// text_with_char_at(text, index, value): a fresh copy with the grapheme at index replaced by the UTF-8 of value
		let text_with_char_at = next_index(self);
		let params = vec![node_ref, ValType::I64, ValType::I64];
		self.runtime_function("text_with_char_at", params.clone(), vec![node_ref], vec![ValType::I32; 8], |s, f| {
			let (start, stop, pointer, end, width, length, copy, codepoint) = (3, 4, 5, 6, 7, 8, 9, 10);
			Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "text_grapheme_offset");
			f.instruction(&I::LocalSet(start));
			s.emit_text_bounds(f, pointer, end);
			Self::emit_list(f, &[I::LocalGet(start), I::LocalGet(end)]);
			s.call(f, "grapheme_end");
			Self::emit_list(f, &[
				I::LocalSet(stop),
				I::LocalGet(2), I::I32WrapI64, I::LocalTee(codepoint), I32Const(utf8::MAX_CODE_POINT), I::I32GtU,
			]);
			s.emit_fail_if(f, "invalid_number");
			Self::emit_list(f, &[
				// UTF-8 width of the code point: 1 to 4 bytes
				I32Const(1), I32Const(2), I32Const(3), I32Const(4),
				I::LocalGet(codepoint), I32Const(utf8::THREE_BYTE_END), I::I32LtU, I::Select,
				I::LocalGet(codepoint), I32Const(utf8::TWO_BYTE_END), I::I32LtU, I::Select,
				I::LocalGet(codepoint), I32Const(utf8::ONE_BYTE_END), I::I32LtU, I::Select,
				I::LocalSet(width),
				// length = bytes - replaced grapheme + width
				I::LocalGet(end), I::LocalGet(pointer), I::I32Sub,
				I::LocalGet(stop), I::LocalGet(start), I::I32Sub, I::I32Sub,
				I::LocalGet(width), I::I32Add, I::LocalSet(length),
			]);
			s.emit_text_allocation(f, length, copy);
			let copy_bytes = I::MemoryCopy { src_mem: 0, dst_mem: 0 };
			Self::emit_list(f, &[
				// bytes before the grapheme
				I::LocalGet(copy), I::LocalGet(pointer), I::LocalGet(start), I::LocalGet(pointer), I::I32Sub, copy_bytes.clone(),
				// start becomes the destination of the new character, the rest follows it
				I::LocalGet(copy), I::LocalGet(start), I::I32Add, I::LocalGet(pointer), I::I32Sub, I::LocalTee(start),
				I::LocalGet(width), I::I32Add,
				I::LocalGet(stop), I::LocalGet(end), I::LocalGet(stop), I::I32Sub, copy_bytes,
				// lead byte marker by width, stored in stop
				I32Const(0), I32Const(utf8::TWO_BYTE_LEAD), I32Const(utf8::THREE_BYTE_LEAD), I32Const(utf8::FOUR_BYTE_LEAD),
				I::LocalGet(width), I32Const(3), I::I32Eq, I::Select,
				I::LocalGet(width), I32Const(2), I::I32Eq, I::Select,
				I::LocalGet(width), I32Const(1), I::I32Eq, I::Select,
				I::LocalSet(stop),
				// continuation bytes from the last one back, 6 bits each
				I::Block(BlockType::Empty), I::Loop(BlockType::Empty),
				I::LocalGet(width), I32Const(1), I::I32LeU, I::BrIf(1),
				I::LocalGet(width), I32Const(1), I::I32Sub, I::LocalTee(width), I::LocalGet(start), I::I32Add,
				I::LocalGet(codepoint), I32Const(utf8::CONTINUATION_PAYLOAD), I::I32And, I32Const(utf8::CONTINUATION_MARK), I::I32Or, I::I32Store8(BYTE),
				I::LocalGet(codepoint), I32Const(utf8::PAYLOAD_BITS), I::I32ShrU, I::LocalSet(codepoint),
				I::Br(0), I::End, I::End,
				I::LocalGet(start), I::LocalGet(codepoint), I::LocalGet(stop), I::I32Or, I::I32Store8(BYTE),
			]);
			s.emit_field(f, 0, 0);
			f.instruction(&I::LocalGet(copy));
			f.instruction(&I::LocalGet(length));
			f.instruction(&I::StructNew(string_type));
			f.instruction(&I::RefNull(HeapType::Concrete(node_type)));
			f.instruction(&I::StructNew(node_type));
		});

		// node_with_at(node, index, value): texts and symbols by grapheme, everything else as list
		self.runtime_function("node_with_at", params, vec![node_ref], vec![], |s, f| {
			s.emit_is_text(f);
			f.instruction(&I::If(BlockType::Result(node_ref)));
			s.emit_forward_arguments(f, text_with_char_at, None);
			f.instruction(&I::Else);
			s.emit_forward_arguments(f, list_with_at, Some("new_int"));
			f.instruction(&I::End);
		});
		// node_with_node_at(node, index, value node): as node_with_at, with any value in a list (a float stays a float)
		if self.should_emit_function(NODE_WITH_NODE_AT) {
			self.runtime_function(NODE_WITH_NODE_AT, vec![node_ref, ValType::I64, node_ref], vec![node_ref], vec![], |s, f| {
				s.emit_is_text(f);
				f.instruction(&I::If(BlockType::Result(node_ref)));
				s.emit_forward_arguments(f, text_with_char_at, Some("get_int_value"));
				f.instruction(&I::Else);
				s.emit_forward_arguments(f, list_with_at, None);
				f.instruction(&I::End);
			});
		}
	}
}


/// The runtime error of a missing constant field is a function `no_field_<name>`; eval reports it as `no field <name>`
pub const NO_FIELD_PREFIX: &str = "no_field_";
/// struct_body(node): the field list of an instance of a declared type, else the node itself
pub(super) const STRUCT_BODY: &str = "struct_body";
/// instance_field(fields, name_ptr, name_len): the value of the field whose name is that very string of the string
/// table, null when none is (then the general lookup decides)
pub(super) const INSTANCE_FIELD: &str = "instance_field";
/// tagged_field(node, name): the field of the tagged object `tag:{…}` the node is, null for any other node
pub(super) const TAGGED_FIELD: &str = "tagged_field";
const KEY_KIND: i64 = Kind::Key as i64;
pub const MAP_KEY_NAME: &str = "map_key_name";
/// node_at_key(xs, key) and node_with_key(xs, key, value): `xs[key]` read and set with a key known only at runtime
pub const NODE_AT_KEY: &str = "node_at_key";
pub const NODE_WITH_KEY: &str = "node_with_key";
/// node_with_node_at(node, index, value): the copy with a Node value at index (node_with_at takes an Int)
pub const NODE_WITH_NODE_AT: &str = "node_with_node_at";
const IS_MAP: &str = "is_map";
const MAP_PART: &str = "map_part";
const MAP_COLUMN: &str = "map_column";
const MAP_COLUMN_CELLS: &str = "map_column_cells";
/// The parts of an entry `key:value` that `map_part` reads
const ENTRY_PART: i32 = 0;
const KEY_PART: i32 = 1;
const VALUE_PART: i32 = 2;
use crate::library_words::{COLLECTION_CONTAINS, COLLECTION_POSITION, MAP_ENTRIES, MAP_GET_OR, MAP_KEYS, MAP_VALUES, MAP_WITHOUT, MAP_WORD_FUNCTIONS};

impl WasmGcEmitter {
	/// The key of `target[key]`: an index that is a symbol or a text (not a number) selects the entry of that name
	pub(super) fn map_is_indexed_by_key(&self, index: &Node) -> bool {
		self.map_key(index).is_some() || self.dynamic_key(index).is_some()
	}

	/// A key held as a Node (`edge[0]` of a map's value): a number indexes, a name looks up, decided at runtime
	pub(super) fn dynamic_key(&self, index: &Node) -> Option<Node> {
		let key = crate::wasp_parser::subscript_key(index)?;
		(self.get_type(key) == Kind::Empty && !matches!(key.drop_meta(), Node::Empty)).then(|| key.clone())
	}

	pub(super) fn map_key(&self, index: &Node) -> Option<Node> {
		let key = crate::wasp_parser::subscript_key(index)?;
		let is_name = matches!(self.get_type(key), Kind::Symbol | Kind::Text | Kind::Codepoint) || matches!(key.drop_meta(), Node::Char(_)); // "x" is a codepoint
		is_name.then(|| key.clone())
	}

	/// Push the map a lookup searches: a struct instance `point:{x:1 y:2}` is searched by its fields, anything else as it is
	fn emit_lookup_target(&mut self, func: &mut Function, target: &Node) {
		self.emit_node_instructions(func, target);
		if !self.ctx.type_registry.types().is_empty() {
			self.emit_call(func, STRUCT_BODY);
		}
	}

	/// Push the Node at `target#index`: the entry value for a key, else the element at the 1-based position
	pub(super) fn emit_indexed_node(&mut self, func: &mut Function, target: &Node, index: &Node) {
		// a map held as a hash table answers when it can; the generic lookup on its Node gives the rest and the errors
		if let (Some(slot), Some(key)) = (self.typed_map(target), self.map_key(index)) {
			func.instruction(&I::Block(BlockType::Result(Ref(self.node_ref(false)))));
			self.emit_typed_map_lookup(func, slot, &key);
			self.emit_generic_indexed_node(func, target, index);
			func.instruction(&I::End);
			return;
		}
		if self.emit_struct_field_node(func, target, index) {
			return;
		}
		self.emit_generic_indexed_node(func, target, index);
	}

	fn emit_generic_indexed_node(&mut self, func: &mut Function, target: &Node, index: &Node) {
		match self.map_key(index) {
			Some(key) => match crate::analyzer::constant_field_name(&key) {
				// the value of the entry, else of the meta entry `@name`, or the runtime error naming the missing field
				Some(name) => {
					let node_ref = Ref(self.node_ref(false));
					func.instruction(&I::Block(BlockType::Result(node_ref)));
					let meta_name = format!("{}{name}", crate::node::ATTRIBUTE_MARK);
					let fallback = (!name.starts_with(crate::node::ATTRIBUTE_MARK)).then_some(meta_name);
					// the target once: a call in it runs once whichever lookup answers
					let held = self.node_scratch();
					self.emit_lookup_target(func, target);
					func.instruction(&I::LocalSet(held));
					let (pointer, length) = self.allocate_string(&name);
					// an instance's field by the very name the program wrote: no symbol made, no general comparison
					if !self.ctx.type_registry.types().is_empty() {
						Self::emit_list(func, &[I::LocalGet(held), I::RefAsNonNull, I32Const(pointer as i32), I32Const(length as i32)]);
						self.emit_call(func, INSTANCE_FIELD);
						func.instruction(&I::BrOnNonNull(0));
					}
					// the entry, a field of a tagged object `Person{name:…}` (data, D4), else the meta entry `@name`
					let attempts = [(name.clone(), "map_find"), (name.clone(), TAGGED_FIELD)].into_iter().chain(fallback.map(|meta| (meta, "map_find")));
					for (key, finder) in attempts {
						let (pointer, length) = self.allocate_string(&key);
						Self::emit_list(func, &[I::LocalGet(held), I::RefAsNonNull, I32Const(pointer as i32), I32Const(length as i32)]);
						self.emit_call(func, "new_symbol");
						self.emit_call(func, finder);
						func.instruction(&I::BrOnNonNull(0));
					}
					func.instruction(&I::Call(self.func_index(&format!("{NO_FIELD_PREFIX}{name}"))));
					func.instruction(&I::Unreachable);
					func.instruction(&I::End);
				}
				None => {
					self.emit_lookup_target(func, target);
					self.emit_node_instructions(func, &key);
					self.emit_call(func, "map_get");
				}
			},
			None => match self.dynamic_key(index) {
				Some(key) => {
					self.emit_node_instructions(func, target);
					self.emit_node_instructions(func, &key);
					self.emit_call(func, NODE_AT_KEY);
				}
				None => self.emit_list_element_node(func, target, index),
			},
		}
	}

	/// map_get(map: ref $Node, key: ref $Node) -> ref $Node: the value of the first `key:value` entry whose key equals `key`
	pub(super) fn emit_map_get(&mut self) {
		if !self.should_emit_function("map_get") && !self.should_emit_function("map_find") && !self.should_emit_function("field_with") {
			return;
		}
		let node = self.type_manager.node_type;
		let node_ref = Ref(self.node_ref(false));
		let nullable_node_ref = Ref(self.node_ref(true));
		let missing_fields: Vec<&'static str> =
			self.ctx.missing_field_names.iter().map(|name| &*Box::leak(format!("{NO_FIELD_PREFIX}{name}").into_boxed_str())).collect();
		// a missing field is a runtime error a `try` catches, thrown with the id of key_not_found
		let key_not_found = RUNTIME_ERRORS.iter().position(|error| *error == "key_not_found").expect("a runtime error") as i32;
		for name in missing_fields {
			self.runtime_function(name, vec![], vec![], vec![], |s, f| s.emit_error_body(f, key_not_found));
		}
		// map_find(map, key): the value of the first `key:value` entry whose key equals `key`, null when there is none or
		// `map` is no list; a text key equals the symbol of the same letters, so `p["name"]` finds `name:"Joe"`
		// map_entry_has_key(entry, key): whether the node is a `key:value` entry of that key; a text key equals the symbol of the same letters
		// map_key_name(key): the symbol a key is compared by; a text or a character becomes the symbol of the same letters,
		// so `{"A":1}`, `{A:1}`, `m["A"]` and `k="A"; m[k]` all name the same entry
		self.runtime_function(MAP_KEY_NAME, vec![node_ref], vec![node_ref], vec![nullable_node_ref], |s, f| {
			let name = 1;
			f.instruction(&I::LocalGet(0));
			s.call(f, super::text_builtins::TEXT_OF);
			f.instruction(&I::LocalSet(name));
			s.emit_field(f, name, 0);
			Self::emit_list(f, &[I::I64Const(Kind::Text as i64), I::I64Eq, I::If(BlockType::Result(node_ref)), I::I64Const(Kind::Symbol as i64)]);
			s.emit_field(f, name, 1);
			s.emit_field(f, name, 2);
			Self::emit_list(f, &[I::StructNew(node), I::Else, I::LocalGet(name), I::RefAsNonNull, I::End]);
		});
		self.runtime_function("map_entry_has_key", vec![node_ref, node_ref], vec![ValType::I32], vec![nullable_node_ref], |s, f| {
			let (key, entry_key) = (1, 2);
			let is_name = |f: &mut Function, local: u32| {
				for kind in [Kind::Text, Kind::Symbol] {
					s.emit_field(f, local, 0);
					Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(kind as i64), I::I64Eq]);
				}
				f.instruction(&I::I32Or);
			};
			s.emit_field(f, 0, 0);
			Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(KEY_KIND), I::I64Eq, I::If(BlockType::Result(ValType::I32))]);
			s.emit_field(f, 0, 1);
			Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(node)), I::LocalSet(entry_key)]);
			// two names (texts or symbols) compare their letters, without making the symbols
			is_name(f, entry_key);
			is_name(f, key);
			Self::emit_list(f, &[I::I32And, I::If(BlockType::Empty)]);
			s.emit_field(f, entry_key, 1);
			s.emit_field(f, key, 1);
			s.call(f, VALUES_EQUAL);
			Self::emit_list(f, &[I::Return, I::End, I::LocalGet(entry_key), I::RefAsNonNull]);
			s.call(f, MAP_KEY_NAME);
			f.instruction(&I::LocalGet(1));
			s.call(f, MAP_KEY_NAME);
			s.call(f, VALUES_EQUAL);
			Self::emit_list(f, &[I::Else, I::I32Const(0), I::End]);
		});
		let locals = vec![nullable_node_ref, nullable_node_ref, nullable_node_ref, ValType::I64];
		self.runtime_function("map_find", vec![node_ref, node_ref], vec![nullable_node_ref], locals, |s, f| {
			let (cell, entry, tmp, kind) = (2, 3, 4, 5);
			let field = |f: &mut Function, local: u32, index: u32| {
				f.instruction(&I::LocalGet(local));
				f.instruction(&I::StructGet { struct_type_index: node, field_index: index });
			};
			// return the value when the entry in local `entry` is a `key:value` whose key is the wanted one
			let check_entry = |f: &mut Function| {
				Self::emit_list(f, &[I::LocalGet(entry), I::RefAsNonNull, I::LocalGet(1)]);
				s.call(f, "map_entry_has_key");
				f.instruction(&I::If(BlockType::Empty));
				field(f, entry, 2);
				Self::emit_list(f, &[I::LocalTee(tmp), I::RefIsNull, I::If(BlockType::Result(nullable_node_ref))]);
				s.call(f, "new_empty");
				Self::emit_list(f, &[I::Else, I::LocalGet(tmp), I::End, I::Return, I::End]);
			};
			field(f, 0, 0);
			Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::LocalSet(kind)]);
			// a single entry `{a:1}` is the `a:1` node itself
			Self::emit_list(f, &[I::LocalGet(kind), I::I64Const(KEY_KIND), I::I64Eq, I::If(BlockType::Empty), I::LocalGet(0), I::LocalSet(entry)]);
			check_entry(f);
			Self::emit_list(f, &[I::RefNull(HeapType::Concrete(node)), I::Return, I::End]);
			Self::emit_list(f, &[I::LocalGet(kind), I::I64Const(Kind::List as i64), I::I64Ne, I::LocalGet(kind), I::I64Const(Kind::Block as i64), I::I64Ne, I::I32And]);
			Self::emit_list(f, &[I::If(BlockType::Empty), I::RefNull(HeapType::Concrete(node)), I::Return, I::End]);
			Self::emit_list(f, &[I::LocalGet(0), I::LocalSet(cell), I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
			Self::emit_list(f, &[I::LocalGet(cell), I::RefIsNull, I::BrIf(1)]);
			field(f, cell, 1);
			Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(node)), I::LocalSet(entry)]);
			check_entry(f);
			field(f, cell, 2);
			Self::emit_list(f, &[I::LocalSet(cell), I::Br(0), I::End, I::End]);
			f.instruction(&I::RefNull(HeapType::Concrete(node)));
		});
		if self.should_emit_function(TAGGED_FIELD) {
			let list_kind = Kind::List as i64;
			self.runtime_function(TAGGED_FIELD, vec![node_ref, node_ref], vec![nullable_node_ref], vec![nullable_node_ref], |s, f| {
				let value = 2;
				let kind_of = |f: &mut Function, local: u32| Self::emit_list(f, &[I::LocalGet(local), I::RefAsNonNull, I::StructGet { struct_type_index: node, field_index: 0 }, I::I64Const(KIND_MASK), I::I64And]);
				kind_of(f, 0);
				Self::emit_list(f, &[I::I64Const(KEY_KIND), I::I64Ne, I::If(BlockType::Empty), I::RefNull(HeapType::Concrete(node)), I::Return, I::End]);
				Self::emit_list(f, &[I::LocalGet(0), I::StructGet { struct_type_index: node, field_index: 2 }, I::LocalTee(value), I::RefIsNull]);
				Self::emit_list(f, &[I::If(BlockType::Empty), I::RefNull(HeapType::Concrete(node)), I::Return, I::End]);
				// the fields: a list of entries, or the one entry `{b:1}` is
				kind_of(f, value);
				Self::emit_list(f, &[I::I64Const(list_kind), I::I64Ne]);
				kind_of(f, value);
				Self::emit_list(f, &[I::I64Const(KEY_KIND), I::I64Ne, I::I32And, I::If(BlockType::Empty), I::RefNull(HeapType::Concrete(node)), I::Return, I::End]);
				Self::emit_list(f, &[I::LocalGet(value), I::RefAsNonNull, I::LocalGet(1)]);
				s.call(f, "map_find");
			});
		}
		if self.should_emit_function(STRUCT_BODY) {
			let type_names: Vec<String> = self.ctx.type_registry.types().iter().map(|type_def| type_def.name.clone()).collect();
			let type_symbols: Vec<(u32, u32)> = type_names.iter().map(|name| self.allocate_string(name)).collect();
			self.runtime_function(STRUCT_BODY, vec![node_ref], vec![node_ref], vec![], |s, f| {
				let field = |f: &mut Function, index: u32| {
					f.instruction(&I::LocalGet(0));
					f.instruction(&I::StructGet { struct_type_index: node, field_index: index });
				};
				field(f, 0);
				Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(KEY_KIND), I::I64Ne, I::If(BlockType::Empty), I::LocalGet(0), I::Return, I::End]);
				// an instance carries its own op code (D4, type_constructor.rs): its fields, no type name compared
				field(f, 0);
				Self::emit_list(f, &[I::I64Const(8), I::I64ShrU, I::I64Const(crate::operators::op_to_code(&crate::operators::Op::None)), I::I64Eq]);
				Self::emit_list(f, &[I::If(BlockType::Empty)]);
				field(f, 2);
				Self::emit_list(f, &[I::RefAsNonNull, I::Return, I::End]);
				for (pointer, length) in type_symbols {
					field(f, 1);
					Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(node)), I::I32Const(pointer as i32), I::I32Const(length as i32)]);
					s.call(f, "new_symbol");
					s.call(f, VALUES_EQUAL);
					f.instruction(&I::If(BlockType::Empty));
					field(f, 2);
					Self::emit_list(f, &[I::RefAsNonNull, I::Return, I::End]);
				}
				f.instruction(&I::LocalGet(0));
			});
		}
		if self.should_emit_function(INSTANCE_FIELD) {
			let string = self.type_manager.string_type;
			let nullable = Ref(self.node_ref(true));
			let int = ValType::I32;
			self.runtime_function(INSTANCE_FIELD, vec![node_ref, int, int], vec![nullable], vec![nullable, nullable, nullable], |_, f| {
				let (cell, entry, key) = (3, 4, 5);
				let get = |f: &mut Function, local: u32, index: u32| Self::emit_list(f, &[I::LocalGet(local), I::RefAsNonNull, I::StructGet { struct_type_index: node, field_index: index }]);
				let kind_is = |f: &mut Function, local: u32, kind: i64| {
					get(f, local, 0);
					Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(kind), I::I64Eq]);
				};
				Self::emit_list(f, &[I::LocalGet(0), I::LocalSet(cell), I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
				Self::emit_list(f, &[I::LocalGet(cell), I::RefIsNull, I::BrIf(1)]);
				kind_is(f, cell, Kind::List as i64);
				Self::emit_list(f, &[I::I32Eqz, I::BrIf(1)]);
				get(f, cell, 1);
				Self::emit_list(f, &[I::RefCastNullable(HeapType::Concrete(node)), I::LocalTee(entry), I::RefIsNull, I::I32Eqz, I::If(BlockType::Empty)]);
				kind_is(f, entry, Kind::Key as i64);
				f.instruction(&I::If(BlockType::Empty));
				get(f, entry, 1);
				Self::emit_list(f, &[I::RefCastNullable(HeapType::Concrete(node)), I::LocalTee(key), I::RefIsNull, I::I32Eqz, I::If(BlockType::Empty)]);
				kind_is(f, key, Kind::Symbol as i64);
				f.instruction(&I::If(BlockType::Empty));
				// the same string: its offset and length
				get(f, key, 1);
				Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(string)), I::StructGet { struct_type_index: string, field_index: 0 }, I::LocalGet(1), I::I32Eq]);
				get(f, key, 1);
				Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(string)), I::StructGet { struct_type_index: string, field_index: 1 }, I::LocalGet(2), I::I32Eq, I::I32And]);
				f.instruction(&I::If(BlockType::Empty));
				get(f, entry, 2);
				Self::emit_list(f, &[I::Return, I::End, I::End, I::End, I::End, I::End]);
				get(f, cell, 2);
				Self::emit_list(f, &[I::LocalSet(cell), I::Br(0), I::End, I::End, I::RefNull(HeapType::Concrete(node))]);
			});
		}
		// map_get(map, key): the value, or the runtime error `key not found`
		self.runtime_function("map_get", vec![node_ref, node_ref], vec![node_ref], vec![], |s, f| {
			Self::emit_list(f, &[I::Block(BlockType::Result(node_ref)), I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "map_find");
			f.instruction(&I::BrOnNonNull(0));
			s.call(f, "key_not_found");
			Self::emit_list(f, &[I::Unreachable, I::End]);
		});
		self.emit_field_with();
		self.emit_map_words();
		self.emit_dynamic_key_access();
	}

	/// `xs[k]` and `xs[k] = v` with a key held as a Node: an Int indexes from 0, anything else names an entry
	fn emit_dynamic_key_access(&mut self) {
		let node_ref = Ref(self.node_ref(false));
		let is_int = |s: &Self, f: &mut Function| {
			s.emit_field(f, 1, 0);
			Self::emit_list(f, &[I::I64Const(Kind::Int as i64), I::I64Eq]);
		};
		let one_based = |s: &Self, f: &mut Function| {
			f.instruction(&I::LocalGet(1));
			s.call(f, "get_int_value");
			Self::emit_list(f, &[I::I64Const(1), I::I64Add]);
		};
		if self.should_emit_function(NODE_AT_KEY) {
			self.runtime_function(NODE_AT_KEY, vec![node_ref, node_ref], vec![node_ref], vec![], |s, f| {
				is_int(s, f);
				Self::emit_list(f, &[I::If(BlockType::Result(node_ref)), I::LocalGet(0)]);
				one_based(s, f);
				s.call(f, "node_index_at");
				Self::emit_list(f, &[I::Else, I::LocalGet(0), I::LocalGet(1)]);
				s.call(f, "map_get");
				f.instruction(&I::End);
			});
		}
		if self.should_emit_function(NODE_WITH_KEY) {
			self.runtime_function(NODE_WITH_KEY, vec![node_ref, node_ref, node_ref], vec![node_ref], vec![], |s, f| {
				is_int(s, f);
				Self::emit_list(f, &[I::If(BlockType::Result(node_ref)), I::LocalGet(0)]);
				one_based(s, f);
				f.instruction(&I::LocalGet(2));
				s.call(f, NODE_WITH_NODE_AT);
				Self::emit_list(f, &[I::Else, I::LocalGet(0), I::LocalGet(1), I::LocalGet(2)]);
				s.call(f, crate::library_words::FIELD_WITH);
				f.instruction(&I::End);
			});
		}
	}

	/// In `node_index_at` (node local 0, index local 1): a pair `k:v` is the two items k and v
	fn emit_pair_part(&self, func: &mut Function) {
		let node = self.type_manager.node_type;
		self.emit_field(func, 0, 0);
		Self::emit_list(func, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(KEY_KIND), I::I64Eq, I::If(BlockType::Empty)]);
		Self::emit_list(func, &[I::LocalGet(1), I::I64Const(1), I::I64Eq, I::If(BlockType::Empty)]);
		self.emit_field(func, 0, 1);
		Self::emit_list(func, &[I::RefCastNonNull(HeapType::Concrete(node)), I::Return, I::End]);
		Self::emit_list(func, &[I::LocalGet(1), I::I64Const(2), I::I64Ne]);
		self.emit_fail_if(func, "index_out_of_range");
		self.emit_value_or_empty(func, 0);
		Self::emit_list(func, &[I::Return, I::End]);
	}

	/// Push the value of the pair in local `pair`, ø for `k:` without one
	fn emit_value_or_empty(&self, func: &mut Function, pair: u32) {
		func.instruction(&I::Block(BlockType::Result(Ref(self.node_ref(false)))));
		self.emit_field(func, pair, 2);
		func.instruction(&I::BrOnNonNull(0));
		self.call(func, "new_empty");
		func.instruction(&I::End);
	}

	/// The words of maps: `keys`, `values`, the entries a `for (k, v) in m` walks, `x in xs` and `m.get(k, default)`.
	/// A map is a `{…}` list of `key:value` entries, or the one entry itself (`{a:1}`); its keys are read as texts.
	fn emit_map_words(&mut self) {
		if !MAP_WORD_FUNCTIONS.iter().any(|name| self.should_emit_function(name)) {
			return;
		}
		let node = self.type_manager.node_type;
		let (node_ref, nullable) = (Ref(self.node_ref(false)), Ref(self.node_ref(true)));
		let next_index = |s: &Self| s.ctx.func_registry.import_count() + s.ctx.func_registry.code_count();
		let is_entry = |s: &Self, f: &mut Function, local: u32| {
			s.emit_field(f, local, 0);
			Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(KEY_KIND), I::I64Eq]);
		};
		// is_map(node): a single entry, or a `{…}` list that starts with one
		self.runtime_function(IS_MAP, vec![node_ref], vec![ValType::I32], vec![], |s, f| {
			is_entry(s, f, 0);
			Self::emit_list(f, &[I::If(BlockType::Empty), I32Const(1), I::Return, I::End]);
			s.emit_field(f, 0, 0);
			Self::emit_list(f, &[I::I64Const(CURLY_LIST_KIND), I::I64Ne, I::If(BlockType::Empty), I32Const(0), I::Return, I::End]);
			s.emit_field(f, 0, 1);
			Self::emit_list(f, &[I::RefTestNonNull(HeapType::Concrete(node)), I::If(BlockType::Result(ValType::I32))]);
			s.emit_field(f, 0, 1);
			Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(node)), I::StructGet { struct_type_index: node, field_index: 0 }]);
			Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(KEY_KIND), I::I64Eq, I::Else, I32Const(0), I::End]);
		});
		// map_part(entry, part): the entry itself (ENTRY_PART), its key as a text (KEY_PART) or its value (VALUE_PART)
		self.runtime_function(MAP_PART, vec![node_ref, ValType::I32], vec![node_ref], vec![nullable], |s, f| {
			let key = 2;
			is_entry(s, f, 0);
			Self::emit_list(f, &[I::I32Eqz, I::LocalGet(1), I32Const(ENTRY_PART), I::I32Eq, I::I32Or, I::If(BlockType::Empty), I::LocalGet(0), I::Return, I::End]);
			Self::emit_list(f, &[I::LocalGet(1), I32Const(VALUE_PART), I::I32Eq, I::If(BlockType::Empty)]);
			s.emit_value_or_empty(f, 0);
			Self::emit_list(f, &[I::Return, I::End]);
			s.emit_field(f, 0, 1);
			Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(node)), I::LocalSet(key)]);
			s.emit_field(f, key, 0);
			Self::emit_list(f, &[I::I64Const(Kind::Symbol as i64), I::I64Eq, I::If(BlockType::Result(node_ref)), I::I64Const(Kind::Text as i64)]);
			s.emit_field(f, key, 1);
			s.emit_field(f, key, 2);
			Self::emit_list(f, &[I::StructNew(node), I::Else, I::LocalGet(key), I::RefAsNonNull, I::End]);
		});
		// map_column_cells(cells, part): a `[…]` list of that part of every entry
		let column_cells = next_index(self);
		self.runtime_function(MAP_COLUMN_CELLS, vec![nullable, ValType::I32], vec![nullable], vec![], |s, f| {
			Self::emit_list(f, &[I::LocalGet(0), I::RefIsNull, I::If(BlockType::Empty), I::RefNull(HeapType::Concrete(node)), I::Return, I::End]);
			// a meta entry `@name:value` is no key, value or entry of the map
			s.emit_field(f, 0, 1);
			s.call(f, super::equality::IS_META_ENTRY);
			f.instruction(&I::If(BlockType::Empty));
			s.emit_field(f, 0, 2);
			Self::emit_list(f, &[I::LocalGet(1), I::Call(column_cells), I::Return, I::End]);
			f.instruction(&I::I64Const(SQUARE_LIST_KIND));
			s.emit_field(f, 0, 1);
			Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(node)), I::LocalGet(1)]);
			s.call(f, MAP_PART);
			s.emit_field(f, 0, 2);
			Self::emit_list(f, &[I::LocalGet(1), I::Call(column_cells), I::StructNew(node)]);
		});
		assert_eq!(self.func_index(MAP_COLUMN_CELLS), column_cells, "recursive call index");
		// map_column(xs, part): that part of every entry of a map, as a list; anything else unchanged
		self.runtime_function(MAP_COLUMN, vec![node_ref, ValType::I32], vec![node_ref], vec![], |s, f| {
			is_entry(s, f, 0);
			Self::emit_list(f, &[I::If(BlockType::Empty), I::I64Const(SQUARE_LIST_KIND), I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, MAP_PART);
			Self::emit_list(f, &[I::RefNull(HeapType::Concrete(node)), I::StructNew(node), I::Return, I::End, I::LocalGet(0)]);
			s.call(f, IS_MAP);
			Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty), I::LocalGet(0), I::Return, I::End, I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, MAP_COLUMN_CELLS);
			f.instruction(&I::RefAsNonNull);
		});
		for (name, part) in [(MAP_ENTRIES, ENTRY_PART), (MAP_KEYS, KEY_PART), (MAP_VALUES, VALUE_PART)] {
			self.runtime_function(name, vec![node_ref], vec![node_ref], vec![], |s, f| {
				Self::emit_list(f, &[I::LocalGet(0), I32Const(part)]);
				s.call(f, MAP_COLUMN);
			});
		}
		// collection_contains(xs, x): 1 when a map has the key x or a list the element x, else 0;
		// collection_position(xs, x) (`x in xs`): the 1-based position of x in a list instead of the 1 (user decision D14)
		for (name, positional) in [(COLLECTION_CONTAINS, false), (COLLECTION_POSITION, true)] {
			self.runtime_function(name, vec![node_ref, node_ref], vec![node_ref], vec![nullable, ValType::I64, ValType::I64], |s, f| {
				let (wanted, cell, kind, position) = (1, 2, 3, 4);
				let answer = |f: &mut Function, s: &Self, value: i64| {
					f.instruction(&if positional && value == 1 { I::LocalGet(position) } else { I::I64Const(value) });
					s.call(f, "new_int");
					f.instruction(&I::Return);
				};
				f.instruction(&I::LocalGet(0));
				s.call(f, IS_MAP);
				f.instruction(&I::If(BlockType::Empty));
				Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(1)]);
				s.call(f, "map_find");
				Self::emit_list(f, &[I::RefIsNull, I::I64ExtendI32U, I::I64Const(1), I::I64Xor]);
				s.call(f, "new_int");
				Self::emit_list(f, &[I::Return, I::End]);
				s.emit_field(f, 0, 0);
				Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::LocalTee(kind), I::I64Const(Kind::Empty as i64), I::I64Eq, I::If(BlockType::Empty)]);
				answer(f, s, 0);
				f.instruction(&I::End);
				// a text holds a text: its 1-based byte position (in), or 1 (contains)
				Self::emit_list(f, &[I::LocalGet(kind), I::I64Const(Kind::Text as i64), I::I64Eq, I::If(BlockType::Empty), I::LocalGet(0), I::LocalGet(wanted)]);
				s.call(f, super::text_builtins::TEXT_OF);
				s.call(f, super::text_builtins::TEXT_FIND);
				if !positional {
					Self::emit_list(f, &[I::I64Const(0), I::I64Ne, I::I64ExtendI32U]);
				}
				s.call(f, "new_int");
				Self::emit_list(f, &[I::Return, I::End]);
				Self::emit_list(f, &[I::LocalGet(kind), I::I64Const(Kind::List as i64), I::I64Ne, I::LocalGet(kind), I::I64Const(Kind::Block as i64), I::I64Ne, I::I32And]);
				s.emit_fail_if(f, "not_a_list");
				Self::emit_list(f, &[I::LocalGet(0), I::LocalSet(cell), I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
				Self::emit_list(f, &[I::LocalGet(cell), I::RefIsNull, I::BrIf(1)]);
				Self::emit_list(f, &[I::LocalGet(position), I::I64Const(1), I::I64Add, I::LocalSet(position)]);
				s.emit_field(f, cell, 1);
				f.instruction(&I::LocalGet(wanted));
				s.call(f, VALUES_EQUAL);
				f.instruction(&I::If(BlockType::Empty));
				answer(f, s, 1);
				f.instruction(&I::End);
				s.emit_field(f, cell, 2);
				Self::emit_list(f, &[I::LocalSet(cell), I::Br(0), I::End, I::End]);
				answer(f, s, 0);
			});
		}
		// map_get_or(map, key, fallback): the value of the key, else the fallback (ø by default: `m.get(k)`)
		self.runtime_function(MAP_GET_OR, vec![node_ref, node_ref, node_ref], vec![node_ref], vec![], |s, f| {
			Self::emit_list(f, &[I::Block(BlockType::Result(node_ref)), I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "map_find");
			Self::emit_list(f, &[I::BrOnNonNull(0), I::LocalGet(2), I::End]);
		});
		// map_without_cells(cells, key): the cells without the key's entry (or, of a list, the first element equal to the
		// key: `xs.remove(v)`), the cells before it copied, the rest shared
		let without_cells = next_index(self);
		self.runtime_function(MAP_WITHOUT_CELLS, vec![nullable, node_ref], vec![nullable], vec![], |s, f| {
			Self::emit_list(f, &[I::LocalGet(0), I::RefIsNull, I::If(BlockType::Empty), I::RefNull(HeapType::Concrete(node)), I::Return, I::End]);
			s.emit_field(f, 0, 1);
			Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(node)), I::LocalGet(1)]);
			s.call(f, "map_entry_has_key");
			s.emit_field(f, 0, 1);
			f.instruction(&I::LocalGet(1));
			s.call(f, VALUES_EQUAL);
			Self::emit_list(f, &[I::I32Or, I::If(BlockType::Empty)]);
			s.emit_field(f, 0, 2);
			Self::emit_list(f, &[I::Return, I::End]);
			s.emit_field(f, 0, 0);
			s.emit_field(f, 0, 1);
			s.emit_field(f, 0, 2);
			Self::emit_list(f, &[I::LocalGet(1), I::Call(without_cells), I::StructNew(node)]);
		});
		// map_without(map, key): ø for the one entry `{a:1}` of the key, the map itself for another one, else its cells
		// without the key's entry
		self.runtime_function(MAP_WITHOUT, vec![node_ref, node_ref], vec![node_ref], vec![], |s, f| {
			is_entry(s, f, 0);
			Self::emit_list(f, &[I::If(BlockType::Result(node_ref)), I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "map_entry_has_key");
			Self::emit_list(f, &[I::If(BlockType::Result(node_ref))]);
			s.call(f, "new_empty");
			Self::emit_list(f, &[I::Else, I::LocalGet(0), I::End, I::Else, I::Block(BlockType::Result(node_ref)), I::LocalGet(0), I::LocalGet(1), I::Call(without_cells), I::BrOnNonNull(0)]);
			s.call(f, "new_empty");
			Self::emit_list(f, &[I::End, I::End]);
		});
		// removed_value(collection, key): what `collection.remove(key)` gives, of a map the key's value (ø if absent), of
		// a list the list without the first element equal to the key (the static kinds don't tell a map from a list)
		self.runtime_function(crate::analyzer::REMOVED_VALUE_CALL, vec![node_ref, node_ref], vec![node_ref], vec![], |s, f| {
			f.instruction(&I::LocalGet(0));
			s.call(f, IS_MAP);
			Self::emit_list(f, &[I::If(BlockType::Result(node_ref)), I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "new_empty");
			s.call(f, MAP_GET_OR);
			Self::emit_list(f, &[I::Else, I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, MAP_WITHOUT);
			f.instruction(&I::End);
		});
	}
}
