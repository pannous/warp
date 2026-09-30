//! List and string operation functions for WASM

use crate::wasm_emitter::{WasmGcEmitter, VALUES_EQUAL};
use wasm_encoder::*;
use Instruction::I32Const;
use Instruction as I;
use ValType::Ref;
use crate::type_kinds::Kind;
use crate::node::Node;
use crate::extensions::strings::{GRAPHEME_EXTEND, GRAPHEME_PICTOGRAPHIC, REGIONAL_INDICATORS, ZERO_WIDTH_JOINER};

const BYTE: MemArg = MemArg { offset: 0, align: 0, memory_index: 0 };

/// Runtime functions that read text by one of its units (byte, code point, grapheme)
const TEXT_UNIT_USERS: [&str; 7] =
	["node_count", "node_bytes", "string_char_at", "node_with_at", "text_byte_count", "text_codepoint_count", "text_grapheme_count"];

impl WasmGcEmitter {
	/// Emit list and string operation helper functions
	pub(crate) fn emit_list_ops(&mut self) {
		let node_ref = self.node_ref(false);
		let node_ref_nullable = self.node_ref(true);
		self.emit_runtime_errors();
		if TEXT_UNIT_USERS.iter().any(|name| self.should_emit_function(name)) {
			self.emit_text_units();
		}

		// list_at(list: ref $Node, index: i64) -> i64
		// Get the numeric value of the element at index (1-based)
		// Traverses linked list: for index N, follow value pointer N-1 times, return data
		if self.should_emit_function("list_at") {
			let func_type = self.type_manager.types().len();
			self.type_manager.types_mut()
				.ty()
				.function(vec![Ref(node_ref), ValType::I64], vec![ValType::I64]);
			self.functions.function(func_type);

			// Locals: 0=list, 1=index, 2=current (loop variable)
			let mut func = Function::new(vec![(1, Ref(node_ref_nullable))]);

			self.emit_list_walk(&mut func, 2);

			// Get current.data (which is a ref to the element Node, cast from anyref)
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 1, // data field (anyref holding ref $Node)
			});
			// Cast anyref to ref $Node
			func.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(self.type_manager.node_type)));
			// Get the inner node's data field (which holds the i64_box)
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 1, // data field of the element node
			});
			self.emit_int_from_payload(&mut func);

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("list_at");
			self.exports.export("list_at", ExportKind::Func, idx);
		}

		// list_node_at(list: ref $Node, index: i64) -> ref $Node
		// Get the element node at index (1-based), returns the node itself (for symbol/text lists)
		if self.should_emit_function("list_node_at") {
			let func_type = self.type_manager.types().len();
			self.type_manager.types_mut()
				.ty()
				.function(vec![Ref(node_ref), ValType::I64], vec![Ref(node_ref)]);
			self.functions.function(func_type);

			// Locals: 0=list, 1=index, 2=current (loop variable)
			let mut func = Function::new(vec![(1, Ref(node_ref_nullable))]);

			self.emit_list_walk(&mut func, 2);

			// Get current.data (which is a ref to the element Node, cast from anyref)
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 1, // data field (anyref holding ref $Node)
			});
			// Cast anyref to ref $Node and return it directly
			func.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(self.type_manager.node_type)));

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("list_node_at");
			self.exports.export("list_node_at", ExportKind::Func, idx);
		}

		// node_count(node: ref $Node) -> i64
		// Count the number of elements in a list/block by traversing the value chain
		if self.should_emit_function("node_count") {
			let func_type = self.type_manager.types().len();
			self.type_manager.types_mut().ty().function(vec![Ref(node_ref)], vec![ValType::I64]);
			self.functions.function(func_type);

			// Locals: 0=node, 1=count, 2=current
			let mut func = Function::new(vec![(1, ValType::I64), (1, Ref(node_ref_nullable))]);

			// a text counts its user-perceived characters (grapheme clusters)
			self.emit_is_text(&mut func);
			func.instruction(&Instruction::If(BlockType::Empty));
			func.instruction(&Instruction::LocalGet(0));
			self.call(&mut func, "text_grapheme_count");
			func.instruction(&Instruction::Return);
			func.instruction(&Instruction::End);

			// ø is the empty list
			self.emit_field(&mut func, 0, 0);
			func.instruction(&Instruction::I64Const(Kind::Empty as i64));
			func.instruction(&Instruction::I64Eq);
			func.instruction(&Instruction::If(BlockType::Empty));
			func.instruction(&Instruction::I64Const(0));
			func.instruction(&Instruction::Return);
			func.instruction(&Instruction::End);

			// a key:value pair is one item, the one-entry object `{a:1}`
			self.emit_field(&mut func, 0, 0);
			func.instruction(&Instruction::I64Const(KIND_MASK));
			func.instruction(&Instruction::I64And);
			func.instruction(&Instruction::I64Const(KEY_KIND));
			func.instruction(&Instruction::I64Eq);
			func.instruction(&Instruction::If(BlockType::Empty));
			func.instruction(&Instruction::I64Const(1));
			func.instruction(&Instruction::Return);
			func.instruction(&Instruction::End);

			// count = 0
			func.instruction(&Instruction::I64Const(0));
			func.instruction(&Instruction::LocalSet(1));

			// current = node
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalSet(2));

			// Loop: while current is not null, count++ and current = current.value
			func.instruction(&Instruction::Block(BlockType::Empty));
			func.instruction(&Instruction::Loop(BlockType::Empty));

			// if current is null, break
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::RefIsNull);
			func.instruction(&Instruction::BrIf(1)); // break to outer block

			// count = count + 1
			func.instruction(&Instruction::LocalGet(1));
			func.instruction(&Instruction::I64Const(1));
			func.instruction(&Instruction::I64Add);
			func.instruction(&Instruction::LocalSet(1));

			// current = current.value (field 2)
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 2,
			});
			func.instruction(&Instruction::LocalSet(2));

			// continue loop
			func.instruction(&Instruction::Br(0));
			func.instruction(&Instruction::End); // end loop
			func.instruction(&Instruction::End); // end block

			// return count
			func.instruction(&Instruction::LocalGet(1));

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("node_count");
			self.exports.export("node_count", ExportKind::Func, idx);
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

		// string_char_at(node: ref $Node, index: i64) -> ref $Node
		// The character at index (1-based) of a Text/Symbol node as Codepoint node, decoding UTF-8
		if self.should_emit_function("string_char_at") {
			self.emit_string_char_at();
		}

		// node_index_at(node: ref $Node, index: i64) -> ref $Node
		// Runtime dispatch: for Text/Symbol call string_char_at, for List/Block call list_node_at
		if self.should_emit_function("node_index_at") {
			let func_type = self.type_manager.types().len();
			self.type_manager.types_mut()
				.ty()
				.function(vec![Ref(node_ref), ValType::I64], vec![Ref(node_ref)]);
			self.functions.function(func_type);

			let mut func = Function::new(vec![]);

			// a one-character text is held as a Codepoint: its only element is itself
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::StructGet { struct_type_index: self.type_manager.node_type, field_index: 0 });
			func.instruction(&Instruction::I64Const(Kind::Codepoint as i64));
			func.instruction(&Instruction::I64Eq);
			func.instruction(&Instruction::If(BlockType::Empty));
			Self::emit_index_compare(&mut func, Instruction::I64Ne);
			self.emit_fail_if(&mut func, "index_out_of_range");
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::Return);
			func.instruction(&Instruction::End);

			// Get node.kind
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 0, // kind field
			});

			// Check if kind is Text (3) or Symbol (5)
			// kind == 3 || kind == 5 means it's a string
			func.instruction(&Instruction::I64Const(3)); // Kind::Text
			func.instruction(&Instruction::I64Eq);
			func.instruction(&Instruction::If(BlockType::Result(Ref(node_ref))));
			// It's a Text - call string_char_at
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalGet(1));
			self.emit_call(&mut func, "string_char_at");
			func.instruction(&Instruction::Else);
			// Check for Symbol
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 0,
			});
			func.instruction(&Instruction::I64Const(5)); // Kind::Symbol
			func.instruction(&Instruction::I64Eq);
			func.instruction(&Instruction::If(BlockType::Result(Ref(node_ref))));
			// It's a Symbol - call string_char_at
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalGet(1));
			self.emit_call(&mut func, "string_char_at");
			func.instruction(&Instruction::Else);
			// Otherwise it's a list - call list_node_at
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalGet(1));
			self.emit_call(&mut func, "list_node_at");
			func.instruction(&Instruction::End); // end inner if
			func.instruction(&Instruction::End); // end outer if

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("node_index_at");
			self.exports.export("node_index_at", ExportKind::Func, idx);
		}

		self.emit_with_at_functions();
		self.emit_list_concat();
		self.emit_library_ops();
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
				I::LocalGet(next), I32Const(0xF0), I::I32LtU, I::Select,
				I::LocalGet(next), I32Const(0xE0), I::I32LtU, I::Select,
				I::LocalGet(next), I32Const(0x80), I::I32LtU, I::Select,
				I::I32Add, I::LocalTee(next),
				I::LocalGet(1), I::LocalGet(next), I::LocalGet(1), I::I32LtU, I::Select,
			]);
		});

		// utf8_decode(pointer, end): the lead byte keeps 7, 5, 4 or 3 payload bits, each continuation byte adds 6
		self.runtime_function("utf8_decode", vec![int, int], vec![int], vec![int], |_, f| {
			let codepoint = 2;
			Self::emit_list(f, &[
				I::LocalGet(0), I::I32Load8U(BYTE), I::LocalTee(codepoint), I32Const(0x80), I::I32GeU, I::If(BlockType::Empty),
				I::LocalGet(codepoint), I32Const(0x07), I32Const(0x0F), I32Const(0x1F),
				I::LocalGet(codepoint), I32Const(0xE0), I::I32GeU, I::Select,
				I::LocalGet(codepoint), I32Const(0xF0), I::I32GeU, I::Select,
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
				I::LocalGet(previous), I32Const(0x0D), I::I32Eq, I::LocalGet(0), I::LocalGet(1), I::I32LtU, I::I32And,
				I::If(BlockType::Empty),
				I::LocalGet(0), I::I32Load8U(BYTE), I32Const(0x0A), I::I32Eq,
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
				I::LocalGet(pointer), I::I32Load8U(BYTE), I32Const(0xC0), I::I32And, I32Const(0x80), I::I32Ne,
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
		func.instruction(&Instruction::Loop(BlockType::Empty));
		func.instruction(&Instruction::LocalGet(pointer));
		func.instruction(&I32Const(1));
		func.instruction(&Instruction::I32Add);
		func.instruction(&Instruction::LocalTee(pointer));
		func.instruction(&Instruction::LocalGet(end));
		func.instruction(&Instruction::I32LtU);
		func.instruction(&Instruction::If(BlockType::Empty));
		func.instruction(&Instruction::LocalGet(pointer));
		func.instruction(&Instruction::I32Load8U(BYTE));
		func.instruction(&I32Const(0xC0));
		func.instruction(&Instruction::I32And);
		func.instruction(&I32Const(0x80));
		func.instruction(&Instruction::I32Eq);
		func.instruction(&Instruction::If(BlockType::Empty));
		if let Some(accumulator) = accumulator {
			func.instruction(&Instruction::LocalGet(accumulator));
			func.instruction(&I32Const(6));
			func.instruction(&Instruction::I32Shl);
			func.instruction(&Instruction::LocalGet(pointer));
			func.instruction(&Instruction::I32Load8U(BYTE));
			func.instruction(&I32Const(0x3F));
			func.instruction(&Instruction::I32And);
			func.instruction(&Instruction::I32Or);
			func.instruction(&Instruction::LocalSet(accumulator));
		}
		func.instruction(&Instruction::Br(2));
		func.instruction(&Instruction::End);
		func.instruction(&Instruction::End);
		func.instruction(&Instruction::End);
	}
}

/// Runtime errors trap inside a function of that name; eval reports the name as an error value
pub const RUNTIME_ERRORS: [&str; 14] = [
	"index_out_of_range", "invalid_number", "out_of_memory", "key_not_found", "float_out_of_int_range",
	"min_of_an_empty_list", "max_of_an_empty_list",
	"not_a_list", "not_a_text", "not_an_int", "non_ascii_text", "not_a_joinable_item", "empty_separator", "not_an_object",
];

impl WasmGcEmitter {
	fn emit_runtime_errors(&mut self) {
		let no_case_errors = self.ctx.missing_case_labels.iter().map(|label| format!("{}{label}", crate::switch::NO_CASE_PREFIX));
		let no_case_errors: Vec<&'static str> = no_case_errors.map(|name| &*Box::leak(name.into_boxed_str())).collect();
		for name in RUNTIME_ERRORS.into_iter().chain(no_case_errors) {
			self.runtime_function(name, vec![], vec![], vec![], |_, f| {
				f.instruction(&Instruction::Unreachable);
			});
		}
	}

	/// Unconditional runtime error; the stack after it is unreachable, so it fits any expected type
	pub(super) fn emit_runtime_error(&mut self, func: &mut Function, error: &'static str) {
		self.emit_call(func, error);
		func.instruction(&Instruction::Unreachable);
	}

	/// Trap when the i32 condition on the stack is true
	pub(super) fn emit_fail_if(&self, func: &mut Function, error: &'static str) {
		func.instruction(&Instruction::If(BlockType::Empty));
		self.call(func, error);
		func.instruction(&Instruction::End);
	}

	fn emit_index_compare(func: &mut Function, compare: Instruction) {
		func.instruction(&Instruction::LocalGet(1));
		func.instruction(&Instruction::I64Const(1));
		func.instruction(&compare);
	}

	pub(super) fn emit_field(&self, func: &mut Function, local: u32, field_index: u32) {
		func.instruction(&Instruction::LocalGet(local));
		func.instruction(&Instruction::StructGet { struct_type_index: self.type_manager.node_type, field_index });
	}

	/// Push field `field_index` (0 ptr, 1 len) of the $String inside text node `local`
	pub(super) fn emit_text_field(&self, func: &mut Function, local: u32, field_index: u32) {
		self.emit_field(func, local, 1);
		func.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(self.type_manager.string_type)));
		func.instruction(&Instruction::StructGet { struct_type_index: self.type_manager.string_type, field_index });
	}

	/// Walk list local 0 to the element at 1-based index local 1 into `current`; trap when out of range
	fn emit_list_walk(&self, func: &mut Function, current: u32) {
		Self::emit_index_compare(func, Instruction::I64LtS);
		self.emit_fail_if(func, "index_out_of_range");
		func.instruction(&Instruction::LocalGet(0));
		func.instruction(&Instruction::LocalSet(current));
		func.instruction(&Instruction::Block(BlockType::Empty));
		func.instruction(&Instruction::Loop(BlockType::Empty));
		func.instruction(&Instruction::LocalGet(current));
		func.instruction(&Instruction::RefIsNull);
		self.emit_fail_if(func, "index_out_of_range");
		Self::emit_index_compare(func, Instruction::I64LeS);
		func.instruction(&Instruction::BrIf(1));
		self.emit_field(func, current, 2);
		func.instruction(&Instruction::LocalSet(current));
		Self::emit_index_compare(func, Instruction::I64Sub);
		func.instruction(&Instruction::LocalSet(1));
		func.instruction(&Instruction::Br(0));
		func.instruction(&Instruction::End);
		func.instruction(&Instruction::End);
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
		func.instruction(&Instruction::GlobalGet(heap));
		func.instruction(&Instruction::I32Eqz);
		func.instruction(&Instruction::GlobalGet(heap));
		func.instruction(&Instruction::LocalGet(length));
		func.instruction(&Instruction::I32Add);
		func.instruction(&Instruction::MemorySize(0));
		func.instruction(&I32Const(PAGE_BITS));
		func.instruction(&Instruction::I32Shl);
		func.instruction(&Instruction::I32GtU);
		func.instruction(&Instruction::I32Or);
		func.instruction(&Instruction::If(BlockType::Empty));
		func.instruction(&Instruction::LocalGet(length));
		func.instruction(&I32Const(PAGE_BITS));
		func.instruction(&Instruction::I32ShrU);
		func.instruction(&I32Const(1));
		func.instruction(&Instruction::I32Add);
		func.instruction(&Instruction::MemoryGrow(0));
		func.instruction(&Instruction::LocalTee(address));
		func.instruction(&I32Const(-1));
		func.instruction(&Instruction::I32Eq);
		self.emit_fail_if(func, "out_of_memory");
		func.instruction(&Instruction::LocalGet(address));
		func.instruction(&I32Const(PAGE_BITS));
		func.instruction(&Instruction::I32Shl);
		func.instruction(&Instruction::GlobalSet(heap));
		func.instruction(&Instruction::End);
		func.instruction(&Instruction::GlobalGet(heap));
		func.instruction(&Instruction::LocalTee(address));
		func.instruction(&Instruction::LocalGet(length));
		func.instruction(&Instruction::I32Add);
		func.instruction(&Instruction::GlobalSet(heap));
	}

	pub(super) fn emit_is_text(&self, func: &mut Function) {
		for kind in [Kind::Text, Kind::Symbol] {
			self.emit_field(func, 0, 0);
			func.instruction(&Instruction::I64Const(kind as i64));
			func.instruction(&Instruction::I64Eq);
		}
		func.instruction(&Instruction::I32Or);
	}

	fn emit_forward_arguments(func: &mut Function, callee: u32) {
		for local in 0..3 {
			func.instruction(&Instruction::LocalGet(local));
		}
		func.instruction(&Instruction::Call(callee));
	}

	/// `target#index = value` leaves the assigned value (i64) on the stack; a variable target gets the updated copy
	pub(super) fn emit_index_assignment(&mut self, func: &mut Function, target: &Node, index: &Node, value: &Node) {
		let assigned = self.scratch(2);
		self.emit_node_instructions(func, target);
		self.emit_numeric_value(func, index);
		self.emit_numeric_value(func, value);
		func.instruction(&Instruction::LocalTee(assigned));
		self.emit_call(func, "node_with_at");
		let variable = match target.drop_meta() {
			Node::Symbol(name) => self.scope.lookup(name).filter(|local| local.kind.is_ref()),
			_ => None,
		};
		match variable {
			Some(local) => func.instruction(&Instruction::LocalSet(local.position)),
			None => func.instruction(&Instruction::Drop),
		};
		func.instruction(&Instruction::LocalGet(assigned));
	}

	/// Arithmetic without an implicit conversion: `"5"+3` and `[1 2]*2` are type errors, `list + list` concatenates.
	/// Returns true when it emitted the whole expression (a Node, or a trap after a recorded type error).
	pub(super) fn emit_typed_arithmetic(&mut self, func: &mut Function, left: &Node, op: &crate::operators::Op, right: &Node) -> bool {
		match self.arithmetic_type(left, op, right) {
			Kind::Text => {
				self.emit_text_concat(func, left, right);
				true
			}
			Kind::List => {
				self.emit_node_instructions(func, left);
				self.emit_node_instructions(func, right);
				self.emit_call(func, "list_concat");
				func.instruction(&Instruction::RefAsNonNull);
				true
			}
			kind => self.emit_arithmetic_type_error(func, left, op, right, kind),
		}
	}

	/// In a numeric context any collection or text operand is a type error
	pub(super) fn emit_arithmetic_type_error(&mut self, func: &mut Function, left: &Node, op: &crate::operators::Op, right: &Node, kind: Kind) -> bool {
		if !matches!(kind, Kind::Error | Kind::List | Kind::Text) {
			return false;
		}
		let (left_kind, right_kind) = (self.get_type(left), self.get_type(right));
		let fix = if left_kind == Kind::List || right_kind == Kind::List {
			"lists only concatenate with lists (+), element-wise arithmetic needs an explicit map"
		} else {
			"no implicit conversion, convert explicitly, e.g. int(\"5\") + 3"
		};
		self.emit_type_error(func, format!("type error: {left_kind} {op} {right_kind}: {fix}"));
		true
	}

	pub(super) fn arithmetic_type(&self, left: &Node, op: &crate::operators::Op, right: &Node) -> Kind {
		crate::analyzer::arithmetic_kind_of_operands(self.get_type(left), op, self.get_type(right), right)
	}

	/// ø (an Empty node) in local `list` becomes null, the end of a cons list
	pub(super) fn emit_empty_as_null(&self, func: &mut Function, list: u32) {
		func.instruction(&Instruction::LocalGet(list));
		func.instruction(&Instruction::RefIsNull);
		func.instruction(&Instruction::If(BlockType::Empty));
		func.instruction(&Instruction::Else);
		self.emit_field(func, list, 0);
		func.instruction(&Instruction::I64Const(Kind::Empty as i64));
		func.instruction(&Instruction::I64Eq);
		func.instruction(&Instruction::If(BlockType::Empty));
		func.instruction(&Instruction::RefNull(HeapType::Concrete(self.type_manager.node_type)));
		func.instruction(&Instruction::LocalSet(list));
		func.instruction(&Instruction::End);
		func.instruction(&Instruction::End);
	}

	/// list_concat(a, b): copies the cells of a in front of b, which is shared
	fn emit_list_concat(&mut self) {
		if !self.should_emit_function("list_concat") {
			return;
		}
		let node_ref_nullable = Ref(self.node_ref(true));
		let node_type = self.type_manager.node_type;
		let list_concat = self.ctx.func_registry.import_count() + self.ctx.func_registry.code_count();
		let params = vec![node_ref_nullable, node_ref_nullable];
		self.runtime_function("list_concat", params, vec![node_ref_nullable], vec![], |s, f| {
			for list in 0..2 {
				s.emit_empty_as_null(f, list);
			}
			f.instruction(&Instruction::LocalGet(0));
			f.instruction(&Instruction::RefIsNull);
			f.instruction(&Instruction::If(BlockType::Result(node_ref_nullable)));
			f.instruction(&Instruction::LocalGet(1));
			f.instruction(&Instruction::Else);
			s.emit_field(f, 0, 0);
			s.emit_field(f, 0, 1);
			s.emit_field(f, 0, 2);
			f.instruction(&Instruction::LocalGet(1));
			f.instruction(&Instruction::Call(list_concat));
			f.instruction(&Instruction::StructNew(node_type));
			f.instruction(&Instruction::End);
		});
		assert_eq!(self.func_index("list_concat"), list_concat, "recursive call index");
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

		// list_with_at(list, index, value): copies the cells up to index, shares the rest
		let list_with_at = next_index(self);
		let params = vec![node_ref_nullable, ValType::I64, ValType::I64];
		self.runtime_function("list_with_at", params, vec![node_ref], vec![], |s, f| {
			f.instruction(&Instruction::LocalGet(0));
			f.instruction(&Instruction::RefIsNull);
			s.emit_fail_if(f, "index_out_of_range");
			Self::emit_index_compare(f, Instruction::I64LtS);
			s.emit_fail_if(f, "index_out_of_range");
			s.emit_field(f, 0, 0);
			Self::emit_index_compare(f, Instruction::I64Eq);
			f.instruction(&Instruction::If(BlockType::Result(Ref(RefType::ANYREF))));
			f.instruction(&Instruction::LocalGet(2));
			s.call(f, "new_int");
			f.instruction(&Instruction::Else);
			s.emit_field(f, 0, 1);
			f.instruction(&Instruction::End);
			Self::emit_index_compare(f, Instruction::I64GtS);
			f.instruction(&Instruction::If(BlockType::Result(node_ref_nullable)));
			s.emit_field(f, 0, 2);
			Self::emit_index_compare(f, Instruction::I64Sub);
			f.instruction(&Instruction::LocalGet(2));
			f.instruction(&Instruction::Call(list_with_at));
			f.instruction(&Instruction::Else);
			s.emit_field(f, 0, 2);
			f.instruction(&Instruction::End);
			f.instruction(&Instruction::StructNew(node_type));
		});
		assert_eq!(self.func_index("list_with_at"), list_with_at, "recursive call index");

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
				I::LocalGet(2), I::I32WrapI64, I::LocalTee(codepoint), I32Const(0x10FFFF), I::I32GtU,
			]);
			s.emit_fail_if(f, "invalid_number");
			Self::emit_list(f, &[
				// UTF-8 width of the code point: 1 to 4 bytes
				I32Const(1), I32Const(2), I32Const(3), I32Const(4),
				I::LocalGet(codepoint), I32Const(0x10000), I::I32LtU, I::Select,
				I::LocalGet(codepoint), I32Const(0x800), I::I32LtU, I::Select,
				I::LocalGet(codepoint), I32Const(0x80), I::I32LtU, I::Select,
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
				I32Const(0), I32Const(0xC0), I32Const(0xE0), I32Const(0xF0),
				I::LocalGet(width), I32Const(3), I::I32Eq, I::Select,
				I::LocalGet(width), I32Const(2), I::I32Eq, I::Select,
				I::LocalGet(width), I32Const(1), I::I32Eq, I::Select,
				I::LocalSet(stop),
				// continuation bytes from the last one back, 6 bits each
				I::Block(BlockType::Empty), I::Loop(BlockType::Empty),
				I::LocalGet(width), I32Const(1), I::I32LeU, I::BrIf(1),
				I::LocalGet(width), I32Const(1), I::I32Sub, I::LocalTee(width), I::LocalGet(start), I::I32Add,
				I::LocalGet(codepoint), I32Const(0x3F), I::I32And, I32Const(0x80), I::I32Or, I::I32Store8(BYTE),
				I::LocalGet(codepoint), I32Const(6), I::I32ShrU, I::LocalSet(codepoint),
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
			f.instruction(&Instruction::If(BlockType::Result(node_ref)));
			Self::emit_forward_arguments(f, text_with_char_at);
			f.instruction(&Instruction::Else);
			Self::emit_forward_arguments(f, list_with_at);
			f.instruction(&Instruction::End);
		});
	}
}


/// The runtime error of a missing constant field is a function `no_field_<name>`; eval reports it as `no field <name>`
pub const NO_FIELD_PREFIX: &str = "no_field_";
/// struct_body(node): the field list of an instance of a declared type, else the node itself
const STRUCT_BODY: &str = "struct_body";
const KEY_KIND: i64 = 6;
const KIND_MASK: i64 = 0xFF;

impl WasmGcEmitter {
	/// The key of `target[key]`: an index that is a symbol or a text (not a number) selects the entry of that name
	pub(super) fn map_is_indexed_by_key(&self, index: &Node) -> bool {
		self.map_key(index).is_some()
	}

	fn map_key(&self, index: &Node) -> Option<Node> {
		let key = crate::wasp_parser::subscript_key(index)?;
		let is_name = matches!(self.get_type(key), Kind::Symbol | Kind::Text) || matches!(key.drop_meta(), Node::Char(_)); // "x" is a codepoint
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
		match self.map_key(index) {
			Some(key) => match crate::analyzer::constant_field_name(&key) {
				// the value of the entry, or the runtime error naming the missing field
				Some(name) => {
					let node_ref = Ref(self.node_ref(false));
					func.instruction(&I::Block(BlockType::Result(node_ref)));
					self.emit_lookup_target(func, target);
					let (pointer, length) = self.allocate_string(&name);
					Self::emit_list(func, &[I32Const(pointer as i32), I32Const(length as i32)]);
					self.emit_call(func, "new_symbol");
					self.emit_call(func, "map_find");
					func.instruction(&I::BrOnNonNull(0));
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
			None => {
				self.emit_node_instructions(func, target);
				self.emit_numeric_value(func, index);
				self.emit_call(func, "node_index_at");
			}
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
		for name in missing_fields {
			self.runtime_function(name, vec![], vec![], vec![], |_, f| {
				f.instruction(&I::Unreachable);
			});
		}
		// map_find(map, key): the value of the first `key:value` entry whose key equals `key`, null when there is none or
		// `map` is no list; a text key equals the symbol of the same letters, so `p["name"]` finds `name:"Joe"`
		// map_entry_has_key(entry, key): whether the node is a `key:value` entry of that key; a text key equals the symbol of the same letters
		self.runtime_function("map_entry_has_key", vec![node_ref, node_ref], vec![ValType::I32], vec![nullable_node_ref], |s, f| {
			let tmp = 2;
			let field = |f: &mut Function, local: u32, index: u32| {
				f.instruction(&I::LocalGet(local));
				f.instruction(&I::StructGet { struct_type_index: node, field_index: index });
			};
			// the node on the stack, with the kind Symbol when it was Text
			let as_symbol = |f: &mut Function| {
				f.instruction(&I::LocalSet(tmp));
				field(f, tmp, 0);
				Self::emit_list(f, &[I::I64Const(Kind::Text as i64), I::I64Eq, I::If(BlockType::Result(nullable_node_ref)), I::I64Const(Kind::Symbol as i64)]);
				field(f, tmp, 1);
				Self::emit_list(f, &[I::RefNull(HeapType::Concrete(node)), I::StructNew(node), I::Else, I::LocalGet(tmp), I::End]);
			};
			field(f, 0, 0);
			Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(KEY_KIND), I::I64Eq, I::If(BlockType::Result(ValType::I32))]);
			field(f, 0, 1);
			Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(node))]);
			as_symbol(f);
			f.instruction(&I::LocalGet(1));
			as_symbol(f);
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
		// map_get(map, key): the value, or the runtime error `key not found`
		self.runtime_function("map_get", vec![node_ref, node_ref], vec![node_ref], vec![], |s, f| {
			Self::emit_list(f, &[I::Block(BlockType::Result(node_ref)), I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "map_find");
			f.instruction(&I::BrOnNonNull(0));
			s.call(f, "key_not_found");
			Self::emit_list(f, &[I::Unreachable, I::End]);
		});
		self.emit_field_with();
	}
}
