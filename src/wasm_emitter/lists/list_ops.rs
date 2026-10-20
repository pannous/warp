//! List and string operation functions for WASM

use crate::extensions::strings::{GRAPHEME_EXTEND, GRAPHEME_PICTOGRAPHIC, REGIONAL_INDICATORS, ZERO_WIDTH_JOINER};
use crate::node::Node;
use crate::operators::Op;
use crate::type_kinds::{Kind, CURLY_LIST_KIND, KIND_MASK, SQUARE_LIST_KIND, UNMARKED_KIND};
use crate::wasm_emitter::layout::{utf8, BYTE};
use crate::wasm_emitter::lists::list_dispatch::Slot;
use crate::wasm_emitter::{
	WasmGcEmitter, CONSTANT_TEXT, NEW_BOOL, NEW_TEXT_OF, TEXT_ADDRESS, TEXT_BYTES_EQUAL, TEXT_BYTES_JOIN, TEXT_FLAT,
	TEXT_FROM_MEMORY, TEXT_LITERAL, TEXT_PIECE, TEXT_SEGMENT, VALUES_EQUAL,
};
use crate::words::MAP_WORD;
use wasm_encoder::*;
use Instruction as I;
use Instruction::I32Const;
use ValType::Ref;

mod runtime_errors;
/// The code points from which UTF-8 needs 2, 3 and 4 bytes
mod text_units;
pub use runtime_errors::*;
mod node_arithmetic;
pub use node_arithmetic::*;
mod list_changes;
pub use list_changes::*;
mod fields_and_keys;
mod swizzles;
pub use fields_and_keys::*;
pub use swizzles::*;

/// The run-time type tests tell a map by node_is_map
pub(in crate::wasm_emitter) fn type_test_dependencies() -> Vec<crate::wasm_emitter::runtime_dependencies::Dependency> {
	use crate::type_tests::{NODE_IS_MAP, NODE_KIND_IN, NODE_TYPE_NAME};
	vec![crate::wasm_emitter::runtime_dependencies::any(&[NODE_KIND_IN, NODE_TYPE_NAME], &[NODE_IS_MAP])]
}

const UTF8_LENGTH_STEPS: [i64; 3] = [0x80, 0x800, 0x10000];
/// The kinds a value of unknown static type may have at run time, named by node_type_name as Kind's Display does
const RUN_TIME_TYPE_KINDS: [Kind; 12] = [
	Kind::Empty,
	Kind::Int,
	Kind::Float,
	Kind::Text,
	Kind::Codepoint,
	Kind::Symbol,
	Kind::Key,
	Kind::Block,
	Kind::List,
	Kind::Error,
	Kind::TypeDef,
	Kind::Function,
];

/// An ASCII letter with this bit set is lowercase: `byte | ASCII_LOWERCASE_BIT == 'e'` takes e and E
const ASCII_LOWERCASE_BIT: i32 = 0x20;
pub(in crate::wasm_emitter) const ELEMENT_CHILDREN: &str = "element_children";
/// text_slice(text, from, to): the graphemes from…to-1 (0-based) of a text as a text sharing its bytes
pub(in crate::wasm_emitter) const TEXT_SLICE: &str = "text_slice";
pub(in crate::wasm_emitter) const ELEMENT_BODY: &str = "element_body";
/// Runtime functions that read text by one of its units (byte, code point, grapheme)
const TEXT_UNIT_USERS: [&str; 10] = [
	crate::library_words::TEXT_GRAPHEMES,
	// grapheme_end walks the characters it reverses
	"text_reverse",
	"node_count",
	"node_bytes",
	"string_char_at",
	TEXT_SLICE,
	"node_with_at",
	"text_byte_count",
	"text_codepoint_count",
	"text_grapheme_count",
];
/// Runtime functions that read text code point by code point (utf8_next, utf8_decode) without its unit cursors
const CODE_POINT_READERS: [&str; 4] =
	[crate::library_words::TEXT_CODE_POINTS, "text_upper", "text_lower", super::super::text::similarity::TEXT_FOLD];

/// The globals of the list cursor (emit_list_cursor_globals): the list indexed last, the cell and index found there, the
/// list counted last and its count, the list extended last and its last cell
#[derive(Clone, Copy)]
enum CursorPart {
	List,
	Cell,
	Index,
	Counted,
	Count,
	Extended,
	Last,
}
const LIST_CURSOR_PARTS: [CursorPart; 7] = [
	CursorPart::List,
	CursorPart::Cell,
	CursorPart::Index,
	CursorPart::Counted,
	CursorPart::Count,
	CursorPart::Extended,
	CursorPart::Last,
];
/// An index or count up to this leaves the cursor where it is: a short list read inside a loop (a row's `row#4`, its
/// `#row`) takes few links from its head anyway, and the loop's long list keeps its cursor, or each step walks it again
const SHORT_WALK: i64 = 8;
/// Lists whose read cursors are kept at once: a loop reading several long lists by turns (`room#i for room in rooms`,
/// lib/sound.warp's four reverb combs) walks on in each instead of from the head of all but one (quadratic)
const READ_CURSORS: u32 = 4;
/// Texts whose grapheme cursors are kept at once (emit_text_cursor_globals): a lexer comparing its source with a word
/// (`src#i == word#j`) walks on in both instead of from the start of the source at each index (quadratic)
const TEXT_CURSORS: u32 = 2;
/// How far (bytes) text_grapheme_back looks back from a cursor; further back, an index walks from the start of the text
const BACK_REACH: i32 = 256;
/// The globals of a text cursor: the text's bytes, a 1-based grapheme index in them, that grapheme's byte index and
/// when the cursor was used last (a stamp of text_cache_clock)
const TEXT_CURSOR_PARTS: [&str; 4] = ["text", "index", "at", "used"];
/// Texts whose grapheme counts are kept at once (text_grapheme_count), so a loop's `i < #text` counts once
const TEXT_COUNTS: u32 = 2;
/// The globals of a counted text: its bytes, its count and when it was used last
const TEXT_COUNTED_PARTS: [&str; 3] = ["text", "count", "used"];
/// Texts below this many bytes stay out of the text cursors and counts: walking them is cheap, and a lexer's words
/// (`starts_at(source, word, at)`) would take the slot of its long source by turns (card text-cache-lru)
pub(super) const SHORT_TEXT: i32 = 64;

/// The locals of a copy of a list's first cells (emit_collect_cells, emit_rebuild_cells), consecutive from `cell`
pub(in crate::wasm_emitter) struct CopyCells {
	cell: u32,
	cells: u32,
	position: u32,
	copied: u32,
	/// what the copied cells go in front of; the copy when rebuilt
	result: u32,
}

impl CopyCells {
	pub(in crate::wasm_emitter) fn at(first: u32) -> Self {
		CopyCells { cell: first, cells: first + 1, position: first + 2, copied: first + 3, result: first + 4 }
	}
}

/// The locals of a walk over a list's items (emit_item_walk), consecutive from `items`: the $NodeList, the next index, the
/// item
#[derive(Clone, Copy)]
pub(in crate::wasm_emitter) struct ItemWalk {
	pub(in crate::wasm_emitter) items: u32,
	pub(in crate::wasm_emitter) index: u32,
	pub(in crate::wasm_emitter) item: u32,
}

impl ItemWalk {
	pub(in crate::wasm_emitter) fn at(first: u32) -> Self {
		ItemWalk { items: first, index: first + 1, item: first + 2 }
	}
}

impl WasmGcEmitter {
	/// Emit list and string operation helper functions
	pub(crate) fn emit_list_ops(&mut self) {
		if TEXT_UNIT_USERS.iter().chain(&CODE_POINT_READERS).any(|name| self.should_emit_function(name)) {
			self.emit_utf8_steps();
		}
		if TEXT_UNIT_USERS.iter().any(|name| self.should_emit_function(name)) {
			self.emit_text_units();
		}
		self.emit_list_cell_access();
		self.emit_node_counting();
		self.emit_node_is_map();
		self.emit_node_kind_test();
		self.emit_list_marks();
		self.emit_node_indexing();
		self.emit_with_at_functions();
		self.emit_typed_list_runtime();
		self.emit_list_concat();
		self.emit_list_extend();
		self.emit_list_become();
		self.emit_element_children();
		self.emit_element_body();
		self.emit_swizzled();
	}

	/// The cursor of list_at, list_node_at and node_count: the list, cell and index found last, so a walk on from it (a
	/// loop's `items#(i+1)` after `items#i`) takes the links between, not all from the head; and the list counted last
	/// with its count (a loop's `index < #items`). A function changing a list's links in place forgets both
	/// (forget_list_cursor). The list extended last and its last cell, so a loop's `out += [x]` appends there instead of
	/// walking the list. Globals in the order of LIST_CURSOR_PARTS.
	pub(in crate::wasm_emitter) fn emit_list_cursor_globals(&mut self) {
		if self.list_cursor_global.is_some()
			|| !["list_at", "list_node_at", "node_count", LIST_EXTEND]
				.iter()
				.any(|function| self.should_emit_function(function))
		{
			return;
		}
		let node_type = self.type_manager.node_type;
		self.list_cursor_global = Some(self.next_global_idx);
		for part in LIST_CURSOR_PARTS {
			let (val_type, initial) = match part {
				CursorPart::Index | CursorPart::Count => (ValType::I64, ConstExpr::i64_const(0)),
				_ => (Ref(self.node_ref(true)), ConstExpr::ref_null(HeapType::Concrete(node_type))),
			};
			self.declare_global_of(val_type, initial);
		}
		// the read cursors after the first: list, cell, index each
		for _ in 1..READ_CURSORS {
			for val_type in [Ref(self.node_ref(true)), Ref(self.node_ref(true)), ValType::I64] {
				let initial = if val_type == ValType::I64 {
					ConstExpr::i64_const(0)
				} else {
					ConstExpr::ref_null(HeapType::Concrete(node_type))
				};
				self.declare_global_of(val_type, initial);
			}
		}
	}

	/// The text cursors of text_grapheme_offset, newest first, and the counted texts of text_grapheme_count, each keyed
	/// by the text's bytes ($Text, never changed in place: an append builds a new text)
	pub(in crate::wasm_emitter) fn emit_text_cursor_globals(&mut self) {
		if self.text_cursor_global.is_some() || !TEXT_UNIT_USERS.iter().any(|name| self.should_emit_function(name)) {
			return;
		}
		let text_type = self.type_manager.text_type;
		let global_of = |part: &str| match part {
			"text" => (self.text_ref_nullable(), ConstExpr::ref_null(HeapType::Concrete(text_type))),
			"at" => (ValType::I32, ConstExpr::i32_const(0)),
			_ => (ValType::I64, ConstExpr::i64_const(0)),
		};
		let cursors = (0..TEXT_CURSORS)
			.flat_map(|slot| TEXT_CURSOR_PARTS.map(|part| (format!("text_cursor_{slot}_{part}"), global_of(part))));
		let counted = (0..TEXT_COUNTS)
			.flat_map(|slot| TEXT_COUNTED_PARTS.map(|part| (format!("text_counted_{slot}_{part}"), global_of(part))));
		let clock = std::iter::once(("text_cache_clock".to_string(), global_of("used")));
		let globals: Vec<_> = cursors.chain(counted).chain(clock).collect();
		self.text_cursor_global = Some(self.next_global_idx);
		for (name, (val_type, initial)) in globals {
			self.declare_named_global(val_type, initial, name);
		}
	}

	/// The globals (text, count, used) of each counted text
	fn texts_counted(&self) -> Vec<(u32, u32, u32)> {
		let Some(cursors) = self.text_cursor_global else {
			return vec![];
		};
		let first = cursors + TEXT_CURSORS * TEXT_CURSOR_PARTS.len() as u32;
		(0..TEXT_COUNTS)
			.map(|slot| first + slot * TEXT_COUNTED_PARTS.len() as u32)
			.map(|text| (text, text + 1, text + 2))
			.collect()
	}

	/// The globals (text, index, at, used) of each text cursor
	fn text_cursors(&self) -> Vec<(u32, u32, u32, u32)> {
		let Some(first) = self.text_cursor_global else {
			return vec![];
		};
		(0..TEXT_CURSORS)
			.map(|slot| first + slot * TEXT_CURSOR_PARTS.len() as u32)
			.map(|text| (text, text + 1, text + 2, text + 3))
			.collect()
	}

	/// The global counting the uses of the text cursors and counts, after them
	fn text_cache_clock(&self) -> Option<u32> {
		let cursors = self.text_cursor_global?;
		Some(cursors + TEXT_CURSORS * TEXT_CURSOR_PARTS.len() as u32 + TEXT_COUNTS * TEXT_COUNTED_PARTS.len() as u32)
	}

	/// The globals (list, cell, index) of read cursor `slot`: the first is the list cursor's own
	fn read_cursor(&self, slot: u32) -> Option<(u32, u32, u32)> {
		let first = self.list_cursor_global?;
		if slot == 0 {
			return Some((
				first + CursorPart::List as u32,
				first + CursorPart::Cell as u32,
				first + CursorPart::Index as u32,
			));
		}
		let list = first + LIST_CURSOR_PARTS.len() as u32 + (slot - 1) * 3;
		Some((list, list + 1, list + 2))
	}

	fn read_cursors(&self) -> Vec<(u32, u32, u32)> {
		(0..READ_CURSORS).filter_map(|slot| self.read_cursor(slot)).collect()
	}

	/// The global of a part of the list cursor
	fn list_cursor(&self, part: CursorPart) -> Option<u32> {
		self.list_cursor_global.map(|first| first + part as u32)
	}

	/// The instructions forgetting the list cursor, before a list's links change in place
	pub(in crate::wasm_emitter) fn forget_list_cursor(&self) -> Vec<Instruction<'static>> {
		[self.forget_list_cursor_keeping_map_indexes(), self.forget_map_indexes()].concat()
	}

	/// The instructions forgetting the list cursor, in field_with and fields_grow, which keep the map indexes right
	pub(in crate::wasm_emitter) fn forget_list_cursor_keeping_map_indexes(&self) -> Vec<Instruction<'static>> {
		self.forget_cursor_parts(&[CursorPart::List, CursorPart::Counted, CursorPart::Extended])
	}

	fn forget_cursor_parts(&self, parts: &[CursorPart]) -> Vec<Instruction<'static>> {
		let forget = |global: u32| [I::RefNull(HeapType::Concrete(self.type_manager.node_type)), I::GlobalSet(global)];
		let read_lists: Vec<u32> = if parts.iter().any(|part| matches!(part, CursorPart::List)) {
			self.read_cursors().into_iter().skip(1).map(|(list, _, _)| list).collect()
		} else {
			vec![]
		};
		parts.iter().filter_map(|part| self.list_cursor(*part)).chain(read_lists).flat_map(forget).collect()
	}

	/// list_at and list_node_at: the element of a cons list at a 1-based index
	fn emit_list_cell_access(&mut self) {
		let node_ref = self.node_ref(false);
		if self.should_emit_function("list_at") || self.should_emit_function("list_node_at") {
			self.emit_index_out_of_range_in();
		}
		// list_at(list, index) -> i64: the number at a 1-based index; a character is its code point, any other non-Int not_an_int
		self.emit_list_cell_function("list_at", ValType::I64, |s, func| s.emit_call(func, "get_int_value"));
		// list_node_at(list, index) -> node: the element itself
		self.emit_list_cell_function("list_node_at", Ref(node_ref), |_, _| {});
	}

	/// `name(list, index)`: the data of the cell at a 1-based index, as a node, then `finish`
	fn emit_list_cell_function(
		&mut self,
		name: &'static str,
		result: ValType,
		finish: impl FnOnce(&mut Self, &mut Function),
	) {
		if !self.should_emit_function(name) {
			return;
		}
		let (node_ref, node_ref_nullable, node_type) =
			(self.node_ref(false), self.node_ref(true), self.type_manager.node_type);
		self.exported_function(
			name,
			vec![Ref(node_ref), ValType::I64],
			vec![result],
			vec![Ref(node_ref_nullable), ValType::I64],
			|s, func| {
				let cell = 2;
				s.emit_require_integral(func, 1);
				s.emit_list_walk(func, cell);
				s.emit_field(func, cell, 1);
				func.instruction(&I::RefCastNonNull(HeapType::Concrete(node_type)));
				finish(s, func);
			},
		);
	}

	/// node_kind_in
	fn emit_node_kind_test(&mut self) {
		let node_ref = self.node_ref(false);
		// node_kind_in(node, mask) -> i64: 1 when bit `kind` of the mask is set (type tests of values of unknown static type);
		// a bool's bit is BOOL_MASK_BIT, not its Int's
		if self.should_emit_function(crate::type_tests::NODE_KIND_IN) {
			self.runtime_function(
				crate::type_tests::NODE_KIND_IN,
				vec![Ref(node_ref), ValType::I64],
				vec![ValType::I64],
				vec![ValType::I64],
				|s, f| {
					let kind = 2;
					f.instruction(&I::LocalGet(1));
					s.emit_field(f, 0, 0);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalTee(kind), I::I64Const(crate::type_kinds::BOOL_KIND), I::I64Eq,
						I::If(BlockType::Result(ValType::I64)),
							I::I64Const(crate::type_kinds::BOOL_MASK_BIT),
						I::Else,
					]);
					Self::emit_is_operation(f, kind);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::If(BlockType::Result(ValType::I64)),
							I::I64Const(crate::type_kinds::OPERATION_MASK_BIT),
						I::Else,
							I::LocalGet(kind), I::I64Const(KIND_MASK), I::I64And,
						I::End,
						I::End,
						I::I64ShrU,
						// code tests as code and as its form
						I::LocalGet(1), I::I64Const(crate::type_kinds::CODE_MASK_BIT), I::I64ShrU,
						I::LocalGet(kind), I::I64Const(crate::type_kinds::CODE_MARK), I::I64And, I::I64Const(0), I::I64Ne,
						I::I64ExtendI32U, I::I64And, I::I64Or,
						I::I64Const(1), I::I64And,
					]);
				},
			);
		}
	}

	/// i32 1 when the kind in local `kind` is an operation: a key whose operator code is one of Op::is_operation
	/// (`1+1` as data), no pair `a:1` or instance entry; the code picks its bit in one of two 64-bit masks
	fn emit_is_operation(f: &mut Function, kind: u32) {
		let [low, high] = crate::operators::operation_code_masks();
		let code = || {
			[
				I::LocalGet(kind),
				I::I64Const(UNMARKED_KIND),
				I::I64And,
				I::I64Const(crate::type_kinds::KIND_BITS),
				I::I64ShrU,
			]
		};
		let mut instructions = vec![I::I64Const(low), I::I64Const(high)];
		instructions.extend(code());
		instructions.extend([I::I64Const(64), I::I64LtU, I::Select]);
		// i64.shr_u takes the shift modulo 64: the code's bit in the mask chosen
		instructions.extend(code());
		instructions.extend([
			I::I64ShrU,
			I::I64Const(1),
			I::I64And,
			I::I32WrapI64,
			I::LocalGet(kind),
			I::I64Const(KIND_MASK),
			I::I64And,
			I::I64Const(Kind::Key as i64),
			I::I64Eq,
			I::I32And,
		]);
		Self::emit_list(f, &instructions);
	}

	/// node on the stack → i32 1 when it is a map (node_is_map)
	pub(in crate::wasm_emitter) fn emit_is_map(&mut self, func: &mut Function) {
		func.instruction(&I::RefAsNonNull);
		self.emit_call(func, crate::type_tests::NODE_IS_MAP);
	}

	/// node_is_map(node) -> i32: 1 when its kind carries the curly bracket and it is empty or begins with a pair
	/// (`{}`, `{a: 1}`); a quoted block `{print 1}` is a curly list too, but code, and so is a parse() result marked as
	/// code (`parse("{a: b}")`)
	fn emit_node_is_map(&mut self) {
		if !self.should_emit_function(crate::type_tests::NODE_IS_MAP) {
			return;
		}
		let node_type = self.type_manager.node_type;
		self.runtime_function(
			crate::type_tests::NODE_IS_MAP,
			vec![Ref(self.node_ref(false))],
			vec![ValType::I32],
			vec![],
			|s, f| {
				s.emit_field(f, 0, 0);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::I64Const(UNMARKED_KIND | crate::type_kinds::CODE_MARK), I::I64And, I::I64Const(CURLY_LIST_KIND),
					I::I64Ne, I::If(BlockType::Empty), I32Const(0), I::Return, I::End,
				]);
				s.emit_field(f, 0, 1);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::RefIsNull, I::If(BlockType::Empty), I32Const(1), I::Return, I::End,
				]);
				s.emit_field(f, 0, 1);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::RefCastNonNull(HeapType::Concrete(node_type)), I::StructGet { struct_type_index: node_type, field_index: 0 },
					I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::Key as i64), I::I64Eq,
				]);
			},
		);
	}

	/// node_type_name(node) -> ref $Node: `type(x)` of a value whose static type is unknown (held as a Node), the symbol
	/// naming its run-time kind as the static names do: bool, rational, int, text, …
	pub(crate) fn emit_node_type_name(&mut self) {
		// only when a program asks type(): emitting all runtimes must not allocate its type names
		if !self.ctx.required_functions.contains(crate::type_tests::NODE_TYPE_NAME) {
			return;
		}
		let node_ref = self.node_ref(false);
		let (kind, pointer, end, byte, function_type, bytes) = (1, 2, 3, 4, 5, 6);
		self.runtime_function(
			crate::type_tests::NODE_TYPE_NAME,
			vec![Ref(node_ref)],
			vec![Ref(node_ref)],
			vec![ValType::I64, ValType::I32, ValType::I32, ValType::I32, Ref(self.node_ref(true)), self.text_ref()],
			|s, f| {
				// a function value: its kind and signature (function_type is ø for any other value)
				if s.types_functions() {
					f.instruction(&I::LocalGet(0));
					s.emit_call(f, crate::wasm_emitter::functions::FUNCTION_TYPE);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalTee(function_type), I::RefIsNull, I::I32Eqz, I::If(BlockType::Empty),
						I::LocalGet(function_type), I::RefAsNonNull, I::Return, I::End,
					]);
				}
				s.emit_field(f, 0, 0);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalTee(kind), I::I64Const(crate::type_kinds::CODE_MARK), I::I64And, I::I64Const(0), I::I64Ne,
					I::If(BlockType::Empty),
				]);
				s.emit_type_value(f, crate::words::CODE_WORD);
				Self::emit_list(f, &[I::Return, I::End]);
				Self::emit_list(
					f,
					&[I::LocalGet(kind), I::I64Const(crate::type_kinds::BOOL_KIND), I::I64Eq, I::If(BlockType::Empty)],
				);
				s.emit_type_value(f, crate::analyzer::BOOL_TYPE);
				Self::emit_list(f, &[I::Return, I::End]);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(kind), I::I64Const(crate::type_kinds::BUILTIN_FUNCTION_MARK), I::I64And, I::I64Const(0), I::I64Ne,
					I::If(BlockType::Empty),
				]);
				s.emit_type_value(f, crate::type_tests::BUILTIN_TYPE);
				Self::emit_list(f, &[I::Return, I::End]);
				// the type of a type value is type (type(int) is type)
				f.instruction(&I::LocalGet(kind));
				Self::emit_type_value_test(f);
				f.instruction(&I::If(BlockType::Empty));
				s.emit_type_value(f, crate::type_tests::TYPE_WORD);
				Self::emit_list(f, &[I::Return, I::End]);
				if s.int_runtime() {
					// an exact number that is no integer holds a $Ratio (exact.rs)
					Self::emit_list(
						f,
						&[I::LocalGet(kind), I::I64Const(Kind::Int as i64), I::I64Eq, I::If(BlockType::Empty)],
					);
					s.emit_field(f, 0, 1);
					Self::emit_list(
						f,
						&[I::RefTestNonNull(HeapType::Concrete(s.type_manager.ratio_type)), I::If(BlockType::Empty)],
					);
					s.emit_type_value(f, crate::analyzer::RATIONAL_WORD);
					Self::emit_list(f, &[I::Return, I::End, I::End]);
				}
				// an instance of a declared type is a Key whose data is the type's name
				let type_names: Vec<String> =
					s.ctx.type_registry.types().iter().map(|type_def| type_def.name.clone()).collect();
				if !type_names.is_empty() {
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(kind), I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::Key as i64), I::I64Eq,
						I::If(BlockType::Empty),
					]);
					for type_name in &type_names {
						s.emit_field(f, 0, 1);
						f.instruction(&I::RefCastNonNull(HeapType::Concrete(s.type_manager.node_type)));
						s.emit_type_value(f, type_name);
						s.emit_call(f, VALUES_EQUAL);
						f.instruction(&I::If(BlockType::Empty));
						s.emit_type_value(f, type_name);
						Self::emit_list(f, &[I::Return, I::End]);
					}
					f.instruction(&I::End);
				}
				Self::emit_list(f, &[I::LocalGet(0)]);
				s.emit_is_map(f);
				f.instruction(&I::If(BlockType::Empty));
				s.emit_type_value(f, MAP_WORD);
				Self::emit_list(f, &[I::Return, I::End]);
				for form in crate::time::TIME_FORMS {
					s.emit_time_form(f, 0);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::I64Const(form as i64), I::I64Eq, I::LocalGet(kind), I::I64Const(KIND_MASK), I::I64And,
						I::I64Const(Kind::Time as i64), I::I64Eq, I::I32And, I::If(BlockType::Empty),
					]);
					s.emit_type_value(f, form.name());
					Self::emit_list(f, &[I::Return, I::End]);
				}
				// an operator node `1+1` is an operation, a pair `a:1` (or an instance's entry) stays a key
				Self::emit_is_operation(f, kind);
				f.instruction(&I::If(BlockType::Empty));
				s.emit_type_value(f, crate::words::OPERATION_WORD);
				Self::emit_list(f, &[I::Return, I::End]);
				// a one-character text is a character (chars-codepoints): what a parameter given "a" holds
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(kind), I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::Text as i64), I::I64Eq,
					I::If(BlockType::Empty),
				]);
				s.emit_text_bounds(f, (bytes, pointer, end));
				s.emit_is_one_character(f, (bytes, pointer, end), byte);
				f.instruction(&I::If(BlockType::Empty));
				s.emit_type_value(f, &Kind::Codepoint.to_string());
				Self::emit_list(f, &[I::Return, I::End, I::End]);
				for named in RUN_TIME_TYPE_KINDS {
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(kind), I::I64Const(KIND_MASK), I::I64And, I::I64Const(named as i64), I::I64Eq,
						I::If(BlockType::Empty),
					]);
					s.emit_type_value(f, &named.to_string());
					Self::emit_list(f, &[I::Return, I::End]);
				}
				s.emit_type_value(f, &Kind::Data.to_string());
			},
		);
	}

	fn emit_node_counting(&mut self) {
		let node_ref = self.node_ref(false);
		// node_count(node) -> i64: the items of a list or block, the characters of a text
		if self.should_emit_function("node_count") {
			self.exported_function(
				"node_count",
				vec![Ref(node_ref)],
				vec![ValType::I64],
				vec![ValType::I64],
				|s, func| {
					s.emit_fail_if_error(func, 0); // `(try 10/0).count` is the Error, not 1 (card error-field)
									// a text counts its user-perceived characters (grapheme clusters)
					s.emit_is_text(func);
					Self::emit_list(func, &[I::If(BlockType::Empty), I::LocalGet(0)]);
					s.call(func, "text_grapheme_count");
					Self::emit_list(func, &[I::Return, I::End]);

					// ø is the empty list
					s.emit_field(func, 0, 0);
					s.emit_unmarked(func);
					#[rustfmt::skip]
					Self::emit_list(func, &[
						I::I64Const(Kind::Empty as i64), I::I64Eq, I::If(BlockType::Empty),
							I::I64Const(0), I::Return,
						I::End,
					]);

					// a key:value pair is one item, the one-entry object `{a:1}`
					s.emit_field(func, 0, 0);
					Self::emit_kind_test(func, KEY_KIND, I::I64Eq);
					Self::emit_list(func, &[I::If(BlockType::Empty), I::I64Const(1), I::Return, I::End]);

					// the list counted last, unchanged since
					if let (Some(counted), Some(count)) =
						(s.list_cursor(CursorPart::Counted), s.list_cursor(CursorPart::Count))
					{
						#[rustfmt::skip]
						Self::emit_list(func, &[
							I::GlobalGet(counted), I::LocalGet(0), I::RefEq, I::If(BlockType::Empty),
								I::GlobalGet(count), I::Return,
							I::End,
						]);
					}

					// the items, a meta entry `@name:value` counting none
					func.instruction(&I::LocalGet(0));
					s.emit_items_length(func);
					func.instruction(&I::LocalSet(1));

					if let (Some(counted), Some(count)) =
						(s.list_cursor(CursorPart::Counted), s.list_cursor(CursorPart::Count))
					{
						#[rustfmt::skip]
						Self::emit_list(func, &[
							I::LocalGet(1), I::I64Const(SHORT_WALK), I::I64GtS, I::If(BlockType::Empty),
								I::LocalGet(0), I::GlobalSet(counted),
								I::LocalGet(1), I::GlobalSet(count),
							I::End,
						]);
					}
					func.instruction(&I::LocalGet(1));
				},
			);
		}

		// node_bytes(node) -> i64: the bytes of a text, 8 bytes per element otherwise
		if self.should_emit_function("node_bytes") {
			self.runtime_function("node_bytes", vec![Ref(node_ref)], vec![ValType::I64], vec![], |s, f| {
				s.emit_fail_if_error(f, 0); // the bytes of an Error are none of its message's (card byte-slice)
								// a one-character text, held as a codepoint, counts the bytes of its UTF-8 encoding
				s.emit_field(f, 0, 0);
				Self::emit_kind_test(f, Kind::Codepoint as i64, I::I64Eq);
				Self::emit_list(f, &[I::If(BlockType::Empty), I::I64Const(1)]);
				for first_code_point_of_longer_encoding in UTF8_LENGTH_STEPS {
					f.instruction(&I::LocalGet(0));
					s.emit_codepoint_of_node(f);
					Self::emit_list(
						f,
						&[I::I64Const(first_code_point_of_longer_encoding), I::I64GeU, I::I64ExtendI32U, I::I64Add],
					);
				}
				Self::emit_list(f, &[I::Return, I::End]);
				s.emit_is_text(f);
				Self::emit_list(f, &[I::If(BlockType::Result(ValType::I64)), I::LocalGet(0)]);
				s.call(f, "text_byte_count");
				Self::emit_list(f, &[I::Else, I::LocalGet(0)]);
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
		if self.should_emit_function(TEXT_SLICE) {
			self.emit_text_slice();
		}

		// node_index_at(node: ref $Node, index: i64) -> ref $Node
		// Runtime dispatch: for Text/Symbol call string_char_at, for List/Block call list_node_at
		if self.should_emit_function("node_index_at") {
			self.exported_function(
				"node_index_at",
				vec![Ref(node_ref), ValType::I64],
				vec![Ref(node_ref)],
				vec![],
				|s, func| {
					s.emit_require_integral(func, 1);

					// a one-character text is held as a Codepoint: its only element is itself
					#[rustfmt::skip]
					Self::emit_list(func, &[
						I::LocalGet(0), I::StructGet { struct_type_index: s.type_manager.node_type, field_index: 0 },
						I::I64Const(Kind::Codepoint as i64), I::I64Eq, I::If(BlockType::Empty),
					]);
					Self::emit_index_compare(func, I::I64Ne);
					s.emit_fail_if(func, "index_out_of_range");
					Self::emit_list(func, &[I::LocalGet(0), I::Return, I::End]);
					s.emit_pair_part(func);

					// a Text or Symbol indexes its characters, anything else its items
					for kind in [Kind::Text, Kind::Symbol] {
						s.emit_field(func, 0, 0);
						Self::emit_list(func, &[I::I64Const(kind as i64), I::I64Eq]);
					}
					Self::emit_list(
						func,
						&[I::I32Or, I::If(BlockType::Result(Ref(node_ref))), I::LocalGet(0), I::LocalGet(1)],
					);
					s.emit_call(func, "string_char_at");
					Self::emit_list(func, &[I::Else, I::LocalGet(0), I::LocalGet(1)]);
					s.emit_call(func, "list_node_at");
					func.instruction(&I::End);
				},
			);
		}
	}

	/// The grapheme at a 1-based index: a Codepoint when it is one code point (`'héllo'#2` → 'é'),
	/// else the cluster as Text sharing the bytes (`'👍🏽'#1` keeps its skin tone modifier)
	fn emit_string_char_at(&mut self) {
		let node_ref = Ref(self.node_ref(false));
		let params = vec![node_ref, ValType::I64];
		let locals = vec![ValType::I32, ValType::I32, ValType::I32, self.text_ref()];
		self.runtime_function("string_char_at", params, vec![node_ref], locals, |s, f| {
			let (start, end, stop, bytes) = (2, 3, 4, 5);
			Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "text_grapheme_offset");
			f.instruction(&I::LocalSet(start));
			s.emit_text_bounds(f, (bytes, stop, end));
			s.emit_grapheme_node(f, (bytes, start, end), stop);
		});
	}

	/// The grapheme at byte index local `start` of the $Text in local `bytes` (ending at `end`) as a node, its end left in
	/// `stop`: a Codepoint when it is one code point, else the cluster as Text (string_char_at, text_graphemes)
	pub(in crate::wasm_emitter) fn emit_grapheme_node(
		&self,
		f: &mut Function,
		(bytes, start, end): (u32, u32, u32),
		stop: u32,
	) {
		let node_ref = Ref(self.node_ref(false));
		let at_start = [I::LocalGet(bytes), I::LocalGet(start), I::LocalGet(end)];
		Self::emit_list(f, &at_start);
		self.call(f, "grapheme_end");
		// a grapheme of one byte is an ASCII character, its byte the code point (card text-costs)
		#[rustfmt::skip]
		Self::emit_list(f, &[
			I::LocalTee(stop), I::LocalGet(start), I::I32Sub, I32Const(1), I::I32Eq, I::If(BlockType::Result(node_ref)),
				I::LocalGet(bytes), I::LocalGet(start), I::ArrayGetU(self.type_manager.text_type),
		]);
		self.call(f, "new_codepoint");
		f.instruction(&I::Else);
		Self::emit_list(f, &at_start);
		self.call(f, "utf8_next");
		Self::emit_list(f, &[I::LocalGet(stop), I::I32Eq, I::If(BlockType::Result(node_ref))]);
		Self::emit_list(f, &at_start);
		self.call(f, "utf8_decode");
		self.call(f, "new_codepoint");
		f.instruction(&I::Else);
		self.emit_text_piece(f, (bytes, start, stop));
		Self::emit_list(f, &[I::End, I::End]);
	}
}

impl WasmGcEmitter {
	/// TEXT_SLICE with `from` and `to` in 0…count: both ends found by text_grapheme_offset, whose cursors make a scan of
	/// consecutive slices linear (`t[a:b]` used to turn the whole text into characters and join them back)
	fn emit_text_slice(&mut self) {
		let node_ref = Ref(self.node_ref(false));
		let (int, long) = (ValType::I32, ValType::I64);
		let params = vec![node_ref, long, long];
		let locals = vec![int, int, int, long, self.text_ref()];
		self.runtime_function(TEXT_SLICE, params, vec![node_ref], locals, |s, f| {
			let (from, to, start, end, stop, count, bytes) = (1, 2, 3, 4, 5, 6, 7);
			s.emit_text_bounds(f, (bytes, start, end));
			f.instruction(&I::LocalGet(0));
			s.call(f, "text_grapheme_count");
			f.instruction(&I::LocalSet(count));
			// the address of the grapheme at 0-based `index`, the end of the text past its last
			let grapheme_address = |f: &mut Function, index: u32| {
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(index), I::LocalGet(count), I::I64LtS, I::If(BlockType::Result(int)),
						I::LocalGet(0), I::LocalGet(index), I::I64Const(1), I::I64Add,
				]);
				s.call(f, "text_grapheme_offset");
				Self::emit_list(f, &[I::Else, I::LocalGet(end), I::End]);
			};
			grapheme_address(f, from);
			f.instruction(&I::LocalSet(start));
			// an end before the start slices nothing
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(to), I::LocalGet(from), I::LocalGet(to), I::LocalGet(from), I::I64GtS, I::Select,
				I::LocalSet(to),
			]);
			grapheme_address(f, to);
			f.instruction(&I::LocalSet(stop));
			s.emit_text_piece(f, (bytes, start, stop));
		});
	}

	fn emit_index_compare(func: &mut Function, compare: Instruction) {
		Self::emit_list(func, &[I::LocalGet(1), I::I64Const(1), compare]);
	}

	pub(in crate::wasm_emitter) fn emit_field(&self, func: &mut Function, local: u32, field_index: u32) {
		Self::emit_list(
			func,
			&[I::LocalGet(local), I::StructGet { struct_type_index: self.type_manager.node_type, field_index }],
		);
	}

	/// `body` for each cell of the list in local `cell`, which steps to the rest after it; `BrIf(1)` in the body ends the walk
	pub(in crate::wasm_emitter) fn emit_cell_walk(
		&self,
		func: &mut Function,
		cell: u32,
		body: impl FnOnce(&mut Function),
	) {
		Self::emit_list(
			func,
			&[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(cell), I::RefIsNull, I::BrIf(1)],
		);
		body(func);
		self.emit_field(func, cell, 2);
		Self::emit_list(func, &[I::LocalSet(cell), I::Br(0), I::End, I::End]);
	}

	/// `body` for each item of the list node in local `list` (list_items), the item in local `walk.item`; `BrIf(1)` in the
	/// body ends the walk
	pub(in crate::wasm_emitter) fn emit_item_walk(
		&self,
		func: &mut Function,
		list: u32,
		walk: ItemWalk,
		body: impl FnOnce(&mut Function),
	) {
		Self::emit_list(func, &[I::LocalGet(list)]);
		self.call(func, super::list_dispatch::LIST_ITEMS);
		func.instruction(&I::LocalSet(walk.items));
		self.emit_items_again(func, walk, body);
	}

	/// `body` for each item in `walk.items` again (emit_item_walk set it), the item in local `walk.item`
	pub(in crate::wasm_emitter) fn emit_items_again(
		&self,
		func: &mut Function,
		walk: ItemWalk,
		body: impl FnOnce(&mut Function),
	) {
		let list_type = self.type_manager.node_list_type;
		let field = |field_index| I::StructGet { struct_type_index: list_type, field_index };
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::I32Const(0), I::LocalSet(walk.index),
			I::Block(BlockType::Empty), I::Loop(BlockType::Empty),
				I::LocalGet(walk.index), I::LocalGet(walk.items), field(0), I::I32GeU, I::BrIf(1),
				I::LocalGet(walk.items), field(1), I::LocalGet(walk.index), I::ArrayGet(self.type_manager.node_array_type),
				I::LocalSet(walk.item),
				I::LocalGet(walk.index), I::I32Const(1), I::I32Add, I::LocalSet(walk.index),
		]);
		body(func);
		Self::emit_list(func, &[I::Br(0), I::End, I::End]);
	}

	/// Store local `value` as the item the walk just took (list_items' array is the walk's own)
	pub(in crate::wasm_emitter) fn emit_item_replaced(&self, func: &mut Function, walk: ItemWalk, value: u32) {
		let field = I::StructGet { struct_type_index: self.type_manager.node_list_type, field_index: 1 };
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::LocalGet(walk.items), field, I::LocalGet(walk.index), I::I32Const(1), I::I32Sub, I::LocalGet(value),
			I::ArraySet(self.type_manager.node_array_type),
		]);
	}

	/// The number of items (i64) of the list node on the stack, as list_items finds them
	pub(in crate::wasm_emitter) fn emit_items_length(&self, func: &mut Function) {
		self.call(func, super::list_dispatch::LIST_ITEMS);
		let length = I::StructGet { struct_type_index: self.type_manager.node_list_type, field_index: 0 };
		Self::emit_list(func, &[length, I::I64ExtendI32U]);
	}

	/// The locals of an item walk, in the order of ItemWalk::at
	pub(in crate::wasm_emitter) fn item_walk_locals(&self) -> [ValType; 3] {
		[self.node_list_type(), ValType::I32, Ref(self.node_ref(true))]
	}

	/// Push the address (0) of a copy in memory or the byte length (1) of the text in node `local`
	pub(in crate::wasm_emitter) fn emit_text_field(&self, func: &mut Function, local: u32, field_index: u32) {
		self.emit_text_bytes(func, local);
		match field_index {
			0 => self.call(func, TEXT_ADDRESS),
			_ => {
				func.instruction(&I::ArrayLen);
			}
		}
	}

	/// Walk list local 0 to the element at 1-based index local 1 into `current`; trap when out of range, naming the index
	/// asked for (local `current + 1`) and the list's range
	fn emit_list_walk(&mut self, func: &mut Function, current: u32) {
		let asked = current + 1;
		Self::emit_list(func, &[I::LocalGet(1), I::LocalSet(asked)]);
		// a stored error indexed (`r = error "x"; r#1`) raises itself, as any operation on it does
		self.emit_fail_if_error(func, 0);
		// a number, a character or a text held where a list is indexed (`x=3; x#1`, a parameter): not_a_list, not a cast trap
		for scalar in [Kind::Int, Kind::Float, Kind::Codepoint, Kind::Text, Kind::Symbol] {
			self.emit_field(func, 0, 0);
			Self::emit_kind_test(func, scalar as i64, I::I64Eq);
			self.emit_fail_if(func, "not_a_list");
		}
		// ø is the empty list: it has no item at any index
		self.emit_field(func, 0, 0);
		Self::emit_kind_test(func, Kind::Empty as i64, I::I64Eq);
		self.emit_fail_out_of_range_if(func, asked);
		Self::emit_index_compare(func, I::I64LtS);
		self.emit_fail_out_of_range_if(func, asked);
		Self::emit_list(func, &[I::LocalGet(0), I::LocalSet(current)]);
		// at or past a cursor of this list: walk on from its cell, the rest of the index
		for (list, cell, index) in self.read_cursors() {
			#[rustfmt::skip]
			Self::emit_list(func, &[
				I::GlobalGet(list), I::LocalGet(0), I::RefEq, I::LocalGet(asked), I::GlobalGet(index), I::I64GeS,
				I::I32And, I::If(BlockType::Empty),
					I::GlobalGet(cell), I::LocalSet(current),
					I::LocalGet(asked), I::GlobalGet(index), I::I64Sub, I::I64Const(1), I::I64Add, I::LocalSet(1),
				I::End,
			]);
		}
		Self::emit_list(
			func,
			&[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(current), I::RefIsNull],
		);
		self.emit_fail_out_of_range_if(func, asked);
		Self::emit_index_compare(func, I::I64LeS);
		func.instruction(&I::BrIf(1));
		self.emit_field(func, current, 2);
		func.instruction(&I::LocalSet(current));
		Self::emit_index_compare(func, I::I64Sub);
		Self::emit_list(func, &[I::LocalSet(1), I::Br(0), I::End, I::End]);
		// meta entries `@name:value` sit behind every field (meta_entries.rs, field_with): arriving at one is past the end
		self.emit_field(func, current, 1);
		self.call(func, super::super::reflection::equality::IS_META_ENTRY);
		self.emit_fail_out_of_range_if(func, asked);
		let cursors = self.read_cursors();
		if !cursors.is_empty() {
			Self::emit_list(func, &[I::LocalGet(asked), I::I64Const(SHORT_WALK), I::I64GtS, I::If(BlockType::Empty)]);
			Self::emit_cursor_store(func, &cursors, current, asked);
			func.instruction(&I::End);
		}
	}

	/// Store the cell found in the cursor of list local 0, or, a list without one, in the first cursor with the others
	/// moved one on (the oldest dropped): `if list == cursor 0 {…} else if list == cursor 1 {…} … else {shift; …}`
	fn emit_cursor_store(func: &mut Function, cursors: &[(u32, u32, u32)], current: u32, asked: u32) {
		let store = |(list, cell, index): (u32, u32, u32)| {
			[
				I::LocalGet(0),
				I::GlobalSet(list),
				I::LocalGet(current),
				I::GlobalSet(cell),
				I::LocalGet(asked),
				I::GlobalSet(index),
			]
		};
		for &(list, _, _) in cursors {
			Self::emit_list(func, &[I::GlobalGet(list), I::LocalGet(0), I::RefEq, I::If(BlockType::Empty)]);
			Self::emit_list(func, &store(cursors[cursors.iter().position(|c| c.0 == list).unwrap()]));
			func.instruction(&I::Else);
		}
		for pair in cursors.windows(2).rev() {
			let ((list, cell, index), (next_list, next_cell, next_index)) = (pair[0], pair[1]);
			#[rustfmt::skip]
			Self::emit_list(func, &[
				I::GlobalGet(list), I::GlobalSet(next_list),
				I::GlobalGet(cell), I::GlobalSet(next_cell),
				I::GlobalGet(index), I::GlobalSet(next_index),
			]);
		}
		Self::emit_list(func, &store(cursors[0]));
		for _ in cursors {
			func.instruction(&I::End);
		}
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
	pub(in crate::wasm_emitter) fn emit_index_assignment(
		&mut self,
		func: &mut Function,
		target: &Node,
		index: &Node,
		value: &Node,
	) {
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

	/// An element value node_with_at stores as given: an Int, or a character in a text known as one. A character into
	/// a target of unknown type goes as its Node: a list read from a field would otherwise hold its codepoint as an Int
	pub(in crate::wasm_emitter) fn is_exact_int_element(&self, target: &Node, value: &Node) -> bool {
		match self.get_type(value) {
			Kind::Int => true,
			Kind::Codepoint => matches!(self.get_type(target), Kind::Text | Kind::Symbol),
			_ => false,
		}
	}

	/// `left = value` with `left` an element `target#index`, valued as `read` emits it: the element read back after the
	/// assignment, so a float or text keeps its value (emit_index_assignment leaves an Int)
	pub(in crate::wasm_emitter) fn emit_index_assignment_read_back(
		&mut self,
		func: &mut Function,
		left: &Node,
		value: &Node,
		read: fn(&mut Self, &mut Function, &Node),
	) {
		let Node::Key(target, crate::operators::Op::Hash, index) = left.drop_meta() else {
			unreachable!("an element target")
		};
		self.emit_index_assignment(func, target, index, value);
		func.instruction(&I::Drop);
		read(self, func, left);
	}

	/// A variable target (local or declared global) gets the updated copy on the stack, any other target drops it
	fn emit_store_updated(&mut self, func: &mut Function, target: &Node) {
		let slot = match target.drop_meta() {
			Node::Symbol(name) => self.node_variable_slot(name),
			_ => None,
		};
		func.instruction(&match slot {
			Some(Slot::Local(index)) => I::LocalSet(index),
			Some(Slot::Global(index)) => I::GlobalSet(index),
			None => I::Drop,
		});
	}

	/// The local or declared global a variable holding a Node (reference) lives in
	pub(in crate::wasm_emitter) fn node_variable_slot(&self, name: &str) -> Option<Slot> {
		match self.scope.lookup(name) {
			Some(local) => local.kind.is_ref().then_some(Slot::Local(local.position)),
			None => {
				self.ctx.user_globals.get(name).filter(|(_, kind)| kind.is_ref()).map(|&(index, _)| Slot::Global(index))
			}
		}
	}

	/// `d[k] = v` with a key read at runtime (`for k in ks {d[k] = 0}`): the copy that `setter` makes with the entry set;
	/// leaves the number of the value (0 for a value that is no number), as `xs[i] = v` does
	fn emit_entry_assignment(
		&mut self,
		func: &mut Function,
		target: &Node,
		key: &Node,
		value: &Node,
		setter: &'static str,
	) {
		self.emit_node_instructions(func, target);
		self.emit_node_instructions(func, key);
		self.emit_node_instructions(func, value);
		self.emit_call(func, setter);
		self.emit_store_updated(func, target);
		self.emit_assigned_entry_value(func, target, key, value);
	}

	/// The value of an entry assignment (i64): an Int value read back from the entry, else 0
	pub(in crate::wasm_emitter) fn emit_assigned_entry_value(
		&mut self,
		func: &mut Function,
		target: &Node,
		key: &Node,
		value: &Node,
	) {
		if self.get_type(value) != Kind::Int {
			func.instruction(&I::I64Const(0));
			return;
		}
		let one_based_key = Node::Key(
			Box::new(key.clone()),
			crate::operators::Op::Add,
			Box::new(Node::Number(crate::extensions::numbers::Number::Int(1))),
		);
		self.emit_indexed_node(func, target, &one_based_key);
		self.emit_call(func, "get_int_value");
	}

	/// ø (an Empty node) in local `list` becomes null, the end of a cons list
	pub(in crate::wasm_emitter) fn emit_empty_as_null(&self, func: &mut Function, list: u32) {
		Self::emit_list(func, &[I::LocalGet(list), I::RefIsNull, I::If(BlockType::Empty), I::Else]);
		self.emit_field(func, list, 0);
		self.emit_unmarked(func);
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::I64Const(Kind::Empty as i64), I::I64Eq, I::If(BlockType::Empty),
				I::RefNull(HeapType::Concrete(self.type_manager.node_type)), I::LocalSet(list),
			I::End,
			I::End,
		]);
	}

	/// The kind on the stack without its element mark, when the program's lists carry marks (declared_values LIST_MARK)
	pub(in crate::wasm_emitter) fn emit_unmarked(&self, func: &mut Function) {
		if self.marks_lists() {
			Self::emit_list(func, &[I::I64Const(KIND_MASK), I::I64And]);
		}
	}

	/// The kind on the stack with the element mark of the node in local `node`, when the program's lists carry marks
	pub(in crate::wasm_emitter) fn emit_or_element_mark(&self, func: &mut Function, node: u32) {
		if self.marks_lists() {
			self.emit_field(func, node, 0);
			Self::emit_list(func, &[I::I64Const(!UNMARKED_KIND), I::I64And, I::I64Or]);
		}
	}

	/// The run-time error when the item in local `item` (`check` LIST_ITEM_CHECK) or an item of the list in local `item`
	/// (LIST_ITEMS_CHECK) does not fit the element mark of the list in local `list`
	pub(in crate::wasm_emitter) fn emit_item_check(&self, func: &mut Function, check: &str, list: u32, item: u32) {
		if self.marks_lists() {
			Self::emit_list(func, &[I::LocalGet(list), I::LocalGet(item)]);
			self.call(func, check);
		}
	}

	/// The node in local `node` (a lone cell or entry) becomes ø in place, so every holder of it sees the empty list;
	/// a declared list keeps its element mark
	pub(in crate::wasm_emitter) fn emit_become_empty(&self, func: &mut Function, node: u32) {
		let node_type = self.type_manager.node_type;
		let set = |field_index: u32| I::StructSet { struct_type_index: node_type, field_index };
		Self::emit_list(func, &self.forget_list_cursor());
		Self::emit_list(func, &[I::LocalGet(node), I::I64Const(Kind::Empty as i64)]);
		self.emit_or_element_mark(func, node);
		Self::emit_list(
			func,
			&[
				set(0),
				I::LocalGet(node),
				I::RefNull(HeapType::Abstract { shared: false, ty: AbstractHeapType::Any }),
				set(1),
				I::LocalGet(node),
				I::RefNull(HeapType::Concrete(node_type)),
				set(2),
			],
		);
	}
}
