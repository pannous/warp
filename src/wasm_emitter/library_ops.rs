//! Runtime functions of the library words (`reverse`, `sort`, `upper`, `lower`, `split`, `join`), see library_words.rs

use crate::node::{Bracket, Node};
use crate::operators::{op_to_code, Op};
use crate::type_kinds::{Kind, KIND_MASK};
use crate::wasm_emitter::WasmGcEmitter;
use wasm_encoder::*;
use Instruction as I;
use Instruction::I32Const;
use ValType::Ref;
use crate::type_kinds::{CURLY_BRACKET_INFO, CURLY_LIST_KIND, KIND_BITS, SQUARE_LIST_KIND};
/// The bracket info in a list's kind, above its KIND_BITS
const BRACKET_INFO_MASK: i64 = 0xff;
/// The operator code in a key's kind, above its KIND_BITS; a type instance `point{x:1}` has none
const OP_INFO_MASK: i64 = 0xff;
/// An operator's text in list_text's table: its pointer and length, two i32 words
const OPERATOR_ENTRY_BYTES: i32 = 2;
use crate::wasm_emitter::layout::BYTE;
use crate::wasm_emitter::text_unicode::CaseMapping;

const KEY_KIND: i64 = Kind::Key as i64;
/// An i64 has at most 19 digits, plus the sign
const MAX_INT_BYTES: i32 = 20;
const DECIMAL_BASE: i64 = 10;
const ZERO_DIGIT: i32 = b'0' as i32;
const MINUS_SIGN: i32 = b'-' as i32;
const COPY_BYTES: I<'static> = I::MemoryCopy { src_mem: 0, dst_mem: 0 };
/// The text of a ø item in the text of a list (`str([1, ø])` is "[1 ø]")
const EMPTY_TEXT: &str = "ø";

/// Names of the runtime functions, keyed by the library word
pub const LIBRARY_FUNCTIONS: [(&str, &str); 18] = [
	(crate::library_words::MAP_KEYS, crate::library_words::MAP_KEYS),
	(crate::library_words::MAP_VALUES, crate::library_words::MAP_VALUES),
	(crate::library_words::MAP_ENTRIES, crate::library_words::MAP_ENTRIES),
	(crate::library_words::COLLECTION_CONTAINS, crate::library_words::COLLECTION_CONTAINS),
	(crate::library_words::COLLECTION_POSITION, crate::library_words::COLLECTION_POSITION),
	(crate::library_words::MAP_GET_OR, crate::library_words::MAP_GET_OR),
	(crate::library_words::MAP_WITHOUT, crate::library_words::MAP_WITHOUT),
	(crate::library_words::ORD, CODEPOINT_OF),
	("chars", "text_chars"),
	("field_with", "field_with"),
	(crate::library_words::INSTANCE_COPY, crate::library_words::INSTANCE_COPY),
	("reverse", "list_reverse"),
	("sort", "list_sort"),
	("upper", "text_upper"),
	("lower", "text_lower"),
	("split", "text_split"),
	("join", "list_join"),
	(crate::library_words::SLICE, NODE_SLICE),
];
pub const NODE_SLICE: &str = "node_slice";
/// field_set_in_place(fields, name, value): an object's field set in place (field_with)
const FIELD_SET_IN_PLACE: &str = "field_set_in_place";
/// fields_grow(fields, entry): a new field added to an object in place (field_with)
const FIELDS_GROW: &str = "fields_grow";
/// text_quoted(text) -> text: how a text inside a container prints (P126)
pub const TEXT_QUOTED: &str = "text_quoted";
const QUOTE_BYTE: i32 = b'"' as i32;
const BACKSLASH_BYTE: i32 = b'\\' as i32;
pub const LIST_JOIN: &str = "list_join";
/// list_text(list, separator): list_join that also takes nested lists, for the text of a list (`str(xs)`)
pub const LIST_TEXT: &str = "list_text";
/// codepoint_of(node) -> Int node: the code point of a character, the runtime error not_a_character for anything else
pub const CODEPOINT_OF: &str = "codepoint_of";
pub const NODE_ORDER: &str = "node_order";

impl WasmGcEmitter {
	pub(crate) fn emit_library_ops(&mut self) {
		self.emit_text_reverse(); // list_reverse hands a text to it
		self.emit_list_reverse();
		self.emit_text_chars(); // and it ends with list_reverse
		if self.should_emit_function(NODE_ORDER) {
			self.emit_node_order();
		}
		self.emit_text_case("text_upper", CaseMapping::Upper);
		self.emit_text_case("text_lower", CaseMapping::Lower);
		self.emit_text_split();
		self.emit_list_join();
		self.emit_list_sort(); // after text_chars and list_join: a text sorts its characters
		self.emit_print_value(); // after list_join, which gives the text
		self.emit_node_slice(); // after list_reverse, text_chars and list_join, which it calls
		self.emit_codepoint_of();
	}

	fn emit_codepoint_of(&mut self) {
		if !self.should_emit_function(CODEPOINT_OF) {
			return;
		}
		let node_ref = Ref(self.node_ref(false));
		self.runtime_function(CODEPOINT_OF, vec![node_ref], vec![node_ref], vec![], |s, f| {
			// ord("a"): a one-character text, as an untyped parameter holds it (card char-text), is its character
			s.emit_field(f, 0, 0);
			Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::Text as i64), I::I64Eq, I::If(BlockType::Empty), I::LocalGet(0)]);
			s.call(f, "text_grapheme_count");
			Self::emit_list(f, &[I::I64Const(1), I::I64Eq, I::If(BlockType::Empty), I::LocalGet(0), I::I64Const(1)]);
			s.call(f, "string_char_at");
			Self::emit_list(f, &[I::LocalSet(0), I::End, I::End]);
			s.emit_require_kind(f, 0, Kind::Codepoint, "not_a_character");
			f.instruction(&I::LocalGet(0));
			s.emit_codepoint_of_node(f);
			s.call(f, "new_int");
		});
	}

	/// Push the i64 of the Int node in local `node`
	fn emit_int_value_of(&self, func: &mut Function, node: u32) {
		self.emit_field(func, node, 1);
		if self.int_runtime() {
			self.call(func, "int_from_payload");
		} else {
			func.instruction(&I::RefCastNonNull(HeapType::Concrete(self.type_manager.i64_box_type)));
			func.instruction(&I::StructGet { struct_type_index: self.type_manager.i64_box_type, field_index: 0 });
		}
	}

	/// Trap unless the node in local `local` has kind `expected`
	pub(super) fn emit_require_kind(&self, func: &mut Function, local: u32, expected: Kind, error: &'static str) {
		self.emit_field(func, local, 0);
		Self::emit_list(func, &[I::I64Const(expected as i64), I::I64Ne]);
		self.emit_fail_if(func, error);
	}

	/// A one-character string is a Codepoint node: local `text` becomes the equivalent Text node of its UTF-8 bytes
	pub(super) fn emit_codepoint_as_text(&self, func: &mut Function, text: u32) {
		Self::emit_list(func, &[I::LocalGet(text), I::RefAsNonNull]);
		self.call(func, crate::wasm_emitter::text_builtins::TEXT_OF);
		func.instruction(&I::LocalSet(text));
	}

	/// Trap unless local 0 is a list (or ø, which `emit_empty_as_null` turned into null)
	fn emit_require_list(&self, func: &mut Function) {
		Self::emit_list(func, &[I::LocalGet(0), I::RefIsNull, I::If(BlockType::Empty), I::Else]);
		self.emit_field(func, 0, 0);
		Self::emit_list(func, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::List as i64), I::I64Ne]);
		self.emit_fail_if(func, "not_a_list");
		func.instruction(&I::End);
	}

	/// The list in local `acc` (null when empty) as a node: ø for the empty list
	fn emit_list_result(&self, func: &mut Function, acc: u32) {
		let node_ref = Ref(self.node_ref(false));
		Self::emit_list(func, &[I::LocalGet(acc), I::RefIsNull, I::If(BlockType::Result(node_ref))]);
		self.call(func, "new_empty");
		Self::emit_list(func, &[I::Else, I::LocalGet(acc), I::RefAsNonNull, I::End]);
	}

	/// list_reverse(list): a new list with the cells in the opposite order
	fn emit_list_reverse(&mut self) {
		if !self.should_emit_function("list_reverse") && !self.should_emit_function("text_split") {
			return;
		}
		let (nullable, node_ref) = (Ref(self.node_ref(true)), Ref(self.node_ref(false)));
		let node_type = self.type_manager.node_type;
		self.runtime_function("list_reverse", vec![nullable], vec![node_ref], vec![nullable], |s, f| {
			let reversed = 1;
			// a text reverses by characters
			Self::emit_list(f, &[I::LocalGet(0), I::RefIsNull, I::I32Eqz, I::If(BlockType::Empty)]);
			for kind in [Kind::Text, Kind::Codepoint] {
				s.emit_field(f, 0, 0);
				Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(kind as i64), I::I64Eq, I::If(BlockType::Empty)]);
				Self::emit_list(f, &[I::LocalGet(0), I::RefAsNonNull]);
				s.call(f, "text_reverse");
				Self::emit_list(f, &[I::Return, I::End]);
			}
			f.instruction(&I::End);
			s.emit_empty_as_null(f, 0);
			s.emit_require_list(f);
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(0), I::RefIsNull, I::BrIf(1)]);
			s.emit_field(f, 0, 0);
			s.emit_field(f, 0, 1);
			Self::emit_list(f, &[I::LocalGet(reversed), I::StructNew(node_type), I::LocalSet(reversed)]);
			s.emit_field(f, 0, 2);
			Self::emit_list(f, &[I::LocalSet(0), I::Br(0), I::End, I::End]);
			s.emit_list_result(f, reversed);
		});
	}

	/// node_order(a, b): negative, 0 or positive as a is before, with or after b. Numbers by value (as f64 when either is a
	/// float), texts and characters by code points (the order of their UTF-8 bytes), instances by the Comparable witness of
	/// their type (witness.rs); anything else is not comparable.
	fn emit_node_order(&mut self) {
		let witness = self.compare_witness_table();
		let node_ref = Ref(self.node_ref(false));
		let float_box = self.type_manager.f64_box_type;
		let exact = self.int_runtime();
		let mut locals = vec![ValType::I64, ValType::I64, ValType::F64, ValType::F64];
		locals.extend([ValType::I32; 6]);
		self.runtime_function(NODE_ORDER, vec![node_ref, node_ref], vec![ValType::I32], locals, |s, f| {
			let (first, second, kinds) = (0, 1, [2, 3]);
			let (first_float, second_float) = (4, 5);
			let (first_pointer, second_pointer, first_length, second_length, index, difference) = (6, 7, 8, 9, 10, 11);
			let is_kind = |f: &mut Function, local: u32, kind: Kind| Self::emit_list(f, &[I::LocalGet(local), I::I64Const(kind as i64), I::I64Eq]);
			let both = |f: &mut Function, kind: Kind, either: I<'static>| {
				is_kind(f, kinds[0], kind);
				is_kind(f, kinds[1], kind);
				f.instruction(&either);
			};
			s.emit_codepoint_as_text(f, first);
			s.emit_codepoint_as_text(f, second);
			for (node, kind) in [(first, kinds[0]), (second, kinds[1])] {
				s.emit_field(f, node, 0);
				Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::LocalSet(kind)]);
			}
			// texts: the first differing byte, else the shorter one first
			both(f, Kind::Text, I::I32And);
			f.instruction(&I::If(BlockType::Empty));
			for (node, pointer, length) in [(first, first_pointer, first_length), (second, second_pointer, second_length)] {
				s.emit_text_field(f, node, 0);
				f.instruction(&I::LocalSet(pointer));
				s.emit_text_field(f, node, 1);
				f.instruction(&I::LocalSet(length));
			}
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
			Self::emit_list(f, &[I::LocalGet(index), I::LocalGet(first_length), I::I32GeU, I::LocalGet(index), I::LocalGet(second_length), I::I32GeU, I::I32Or, I::BrIf(1)]);
			Self::emit_list(f, &[I::LocalGet(first_pointer), I::LocalGet(index), I::I32Add, I::I32Load8U(BYTE)]);
			Self::emit_list(f, &[I::LocalGet(second_pointer), I::LocalGet(index), I::I32Add, I::I32Load8U(BYTE), I::I32Sub, I::LocalTee(difference)]);
			Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(difference), I::Return, I::End]);
			Self::emit_list(f, &[I::LocalGet(index), I32Const(1), I::I32Add, I::LocalSet(index), I::Br(0), I::End, I::End]);
			Self::emit_list(f, &[I::LocalGet(first_length), I::LocalGet(second_length), I::I32Sub, I::Return, I::End]);
			if let Some(table) = witness {
				s.emit_instance_order(f, table, kinds, first, second);
			}
			// numbers only from here
			for kind in kinds {
				is_kind(f, kind, Kind::Int);
				is_kind(f, kind, Kind::Float);
				Self::emit_list(f, &[I::I32Or, I::I32Eqz]);
				s.emit_fail_if(f, "not_comparable");
			}
			both(f, Kind::Int, I::I32And);
			f.instruction(&I::If(BlockType::Empty));
			s.emit_int_value_of(f, first);
			s.emit_int_value_of(f, second);
			if exact {
				s.call(f, "exact_cmp");
			} else {
				// both kinds are known to be Int here, so their locals hold the values
				let (a, b) = (kinds[0], kinds[1]);
				Self::emit_list(f, &[I::LocalSet(b), I::LocalSet(a), I::LocalGet(a), I::LocalGet(b), I::I64GtS, I::LocalGet(a), I::LocalGet(b), I::I64LtS, I::I32Sub]);
			}
			Self::emit_list(f, &[I::Return, I::End]);
			// a float on either side compares both as f64
			for (node, kind, float) in [(first, kinds[0], first_float), (second, kinds[1], second_float)] {
				is_kind(f, kind, Kind::Float);
				f.instruction(&I::If(BlockType::Result(ValType::F64)));
				s.emit_field(f, node, 1);
				Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(float_box)), I::StructGet { struct_type_index: float_box, field_index: 0 }, I::Else]);
				s.emit_int_value_of(f, node);
				if exact {
					s.call(f, "exact_to_f64");
				} else {
					f.instruction(&I::F64ConvertI64S);
				}
				Self::emit_list(f, &[I::End, I::LocalSet(float)]);
			}
			Self::emit_list(f, &[I::LocalGet(first_float), I::LocalGet(second_float), I::F64Gt, I::LocalGet(first_float), I::LocalGet(second_float), I::F64Lt, I::I32Sub]);
		});
	}

	/// list_sort(list): a new list of comparable values in ascending order (insertion into a sorted list, see node_order)
	fn emit_list_sort(&mut self) {
		if !self.should_emit_function("list_sort") {
			return;
		}
		let (nullable, node_ref) = (Ref(self.node_ref(true)), Ref(self.node_ref(false)));
		let node_type = self.type_manager.node_type;
		let next_index = |s: &Self| s.ctx.func_registry.import_count() + s.ctx.func_registry.code_count();

		// insert_sorted(sorted, element, kind): the cells of sorted before the first value not before element, the element, the rest shared
		let insert_sorted = next_index(self);
		let params = vec![nullable, node_ref, ValType::I64];
		self.runtime_function("list_insert_sorted", params, vec![node_ref], vec![nullable], |s, f| {
			let (sorted, element, kind, head) = (0, 1, 2, 3);
			Self::emit_list(f, &[I::LocalGet(sorted), I::RefIsNull, I::If(BlockType::Empty), I::LocalGet(kind), I::LocalGet(element)]);
			Self::emit_list(f, &[I::RefNull(HeapType::Concrete(node_type)), I::StructNew(node_type), I::Return, I::End]);
			s.emit_field(f, sorted, 1);
			Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(node_type)), I::LocalSet(head), I::LocalGet(head), I::RefAsNonNull, I::LocalGet(element)]);
			s.call(f, NODE_ORDER);
			Self::emit_list(f, &[I32Const(0), I::I32GeS, I::If(BlockType::Empty), I::LocalGet(kind), I::LocalGet(element)]);
			Self::emit_list(f, &[I::LocalGet(sorted), I::StructNew(node_type), I::Return, I::End]);
			s.emit_field(f, sorted, 0);
			s.emit_field(f, sorted, 1);
			s.emit_field(f, sorted, 2);
			Self::emit_list(f, &[I::LocalGet(element), I::LocalGet(kind), I::Call(insert_sorted), I::StructNew(node_type)]);
		});
		assert_eq!(self.func_index("list_insert_sorted"), insert_sorted, "recursive call index");

		self.runtime_function("list_sort", vec![nullable], vec![node_ref], vec![nullable, nullable, ValType::I32], |s, f| {
			let (sorted, element, is_text) = (1, 2, 3);
			// a text sorts its characters: `sorted "hello"` is "ehllo"
			s.emit_text_as_characters(f, 0, is_text);
			s.emit_empty_as_null(f, 0);
			s.emit_require_list(f);
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(0), I::RefIsNull, I::BrIf(1)]);
			s.emit_field(f, 0, 1);
			Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(node_type)), I::LocalSet(element)]);
			Self::emit_list(f, &[I::LocalGet(sorted), I::LocalGet(element), I::RefAsNonNull]);
			s.emit_field(f, 0, 0);
			Self::emit_list(f, &[I::Call(insert_sorted), I::LocalSet(sorted)]);
			s.emit_field(f, 0, 2);
			Self::emit_list(f, &[I::LocalSet(0), I::Br(0), I::End, I::End]);
			s.emit_list_result(f, sorted);
			f.instruction(&I::LocalSet(sorted));
			s.emit_characters_as_text(f, sorted, is_text);
		});
	}

	/// text_split(text, separator): the pieces between the separators as a list of texts that share the bytes of the text
	fn emit_text_split(&mut self) {
		if !self.should_emit_function("text_split") {
			return;
		}
		self.emit_text_heap_global();
		let (node_ref, nullable) = (Ref(self.node_ref(false)), Ref(self.node_ref(true)));
		let node_type = self.type_manager.node_type;
		let mut locals = vec![nullable];
		locals.extend([ValType::I32; 8]);
		self.runtime_function("text_split", vec![node_ref, node_ref], vec![node_ref], locals, |s, f| {
			let (pieces, pointer, end, separator, separator_end, start, index, matched, offset) = (2, 3, 4, 5, 6, 7, 8, 9, 10);
			s.emit_codepoint_as_text(f, 0);
			s.emit_codepoint_as_text(f, 1);
			s.emit_require_kind(f, 0, Kind::Text, "not_a_text");
			s.emit_require_kind(f, 1, Kind::Text, "not_a_text");
			s.emit_text_bounds(f, pointer, end);
			s.emit_text_field(f, 1, 0);
			f.instruction(&I::LocalTee(separator));
			s.emit_text_field(f, 1, 1);
			Self::emit_list(f, &[I::I32Add, I::LocalSet(separator_end)]);
			Self::emit_list(f, &[I::LocalGet(separator), I::LocalGet(separator_end), I::I32Eq]);
			s.emit_fail_if(f, "empty_separator");
			Self::emit_list(f, &[I::LocalGet(pointer), I::LocalTee(start), I::LocalSet(index)]);
			// the bytes from `start` up to the local `stop` as a text at the front of `pieces`
			let push_piece = |f: &mut Function, stop: u32| {
				Self::emit_list(f, &[I::I64Const(SQUARE_LIST_KIND), I::LocalGet(start), I::LocalGet(stop), I::LocalGet(start), I::I32Sub]);
				s.call(f, "new_text");
				Self::emit_list(f, &[I::LocalGet(pieces), I::StructNew(node_type), I::LocalSet(pieces)]);
			};
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
			// stop when the separator no longer fits before the end
			Self::emit_list(f, &[I::LocalGet(index), I::LocalGet(separator_end), I::I32Add, I::LocalGet(separator), I::I32Sub, I::LocalGet(end), I::I32GtU, I::BrIf(1)]);
			// compare the separator with the bytes at index
			Self::emit_list(f, &[I32Const(1), I::LocalSet(matched), I32Const(0), I::LocalSet(offset)]);
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
			Self::emit_list(f, &[I::LocalGet(separator), I::LocalGet(offset), I::I32Add, I::LocalGet(separator_end), I::I32GeU, I::BrIf(1)]);
			Self::emit_list(f, &[I::LocalGet(index), I::LocalGet(offset), I::I32Add, I::I32Load8U(BYTE)]);
			Self::emit_list(f, &[I::LocalGet(separator), I::LocalGet(offset), I::I32Add, I::I32Load8U(BYTE), I::I32Ne]);
			Self::emit_list(f, &[I::If(BlockType::Empty), I32Const(0), I::LocalSet(matched), I::Br(2), I::End]);
			Self::emit_list(f, &[I::LocalGet(offset), I32Const(1), I::I32Add, I::LocalSet(offset), I::Br(0), I::End, I::End]);
			Self::emit_list(f, &[I::LocalGet(matched), I::If(BlockType::Empty)]);
			push_piece(f, index);
			Self::emit_list(f, &[I::LocalGet(index), I::LocalGet(separator_end), I::I32Add, I::LocalGet(separator), I::I32Sub, I::LocalTee(index), I::LocalSet(start)]);
			Self::emit_list(f, &[I::Else, I::LocalGet(index), I32Const(1), I::I32Add, I::LocalSet(index), I::End, I::Br(0), I::End, I::End]);
			push_piece(f, end);
			Self::emit_list(f, &[I::LocalGet(pieces)]);
			s.call(f, "list_reverse");
		});
	}

	/// int_to_decimal(position, value): writes the decimal digits of the i64 at `position`, returns how many bytes it wrote
	fn emit_int_to_decimal(&mut self) {
		let locals = vec![ValType::I32, ValType::I32, ValType::I64, ValType::I32];
		self.runtime_function("int_to_decimal", vec![ValType::I32, ValType::I64], vec![ValType::I32], locals, |_, f| {
			let (position, value, negative, digits, rest, offset) = (0, 1, 2, 3, 4, 5);
			Self::emit_list(f, &[I::LocalGet(value), I::I64Const(0), I::I64LtS, I::LocalTee(negative), I::If(BlockType::Empty)]);
			Self::emit_list(f, &[I::LocalGet(position), I32Const(MINUS_SIGN), I::I32Store8(BYTE), I::I64Const(0), I::LocalGet(value), I::I64Sub, I::LocalSet(value), I::End]);
			// count the digits
			Self::emit_list(f, &[I32Const(1), I::LocalSet(digits), I::LocalGet(value), I::LocalSet(rest)]);
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(rest), I::I64Const(DECIMAL_BASE), I::I64LtU, I::BrIf(1)]);
			Self::emit_list(f, &[I::LocalGet(rest), I::I64Const(DECIMAL_BASE), I::I64DivU, I::LocalSet(rest), I::LocalGet(digits), I32Const(1), I::I32Add, I::LocalSet(digits), I::Br(0), I::End, I::End]);
			// write them from the last one back
			Self::emit_list(f, &[I::LocalGet(digits), I::LocalSet(offset)]);
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(offset), I::I32Eqz, I::BrIf(1)]);
			Self::emit_list(f, &[I::LocalGet(offset), I32Const(1), I::I32Sub, I::LocalSet(offset)]);
			Self::emit_list(f, &[I::LocalGet(position), I::LocalGet(negative), I::I32Add, I::LocalGet(offset), I::I32Add]);
			Self::emit_list(f, &[I::LocalGet(value), I::I64Const(DECIMAL_BASE), I::I64RemU, I::I32WrapI64, I32Const(ZERO_DIGIT), I::I32Add, I::I32Store8(BYTE)]);
			Self::emit_list(f, &[I::LocalGet(value), I::I64Const(DECIMAL_BASE), I::I64DivU, I::LocalSet(value), I::Br(0), I::End, I::End]);
			Self::emit_list(f, &[I::LocalGet(digits), I::LocalGet(negative), I::I32Add]);
		});
	}

	/// list_join(list, separator): the items (texts, ints, ASCII characters) as one text with the separator between them;
	/// list_text(list, separator), the text of a list (`str(xs)`), also takes nested lists, each as "[…]" ("{…}" for a
	/// map), entries as "key:value", symbols as their names and texts quoted (P126: `["a" "b"]`, `p{name:"a"}`)
	fn emit_list_join(&mut self) {
		for (name, nested) in [(LIST_JOIN, false), (LIST_TEXT, true)] {
			if self.should_emit_function(name) {
				self.emit_joining(name, nested);
			}
		}
	}

	/// text_quoted(text): the text in double quotes, a `"` or `\` in it escaped by a `\` (P126: `["say \"hi\""]`);
	/// byte-wise, as neither byte occurs inside a multi-byte UTF-8 character
	fn emit_text_quoted(&mut self) {
		if !self.should_emit_function(TEXT_QUOTED) {
			return;
		}
		self.emit_text_heap_global();
		let node_ref = Ref(self.node_ref(false));
		self.runtime_function(TEXT_QUOTED, vec![node_ref], vec![node_ref], vec![ValType::I32; 6], |s, f| {
			let (pointer, end, capacity, destination, out, byte) = (1, 2, 3, 4, 5, 6);
			let write = |f: &mut Function, value: I<'static>| {
				Self::emit_list(f, &[I::LocalGet(out), value, I::I32Store8(BYTE), I::LocalGet(out), I32Const(1), I::I32Add, I::LocalSet(out)]);
			};
			s.emit_text_bounds(f, pointer, end);
			// at worst every byte is escaped, plus the two quotes
			Self::emit_list(f, &[I::LocalGet(end), I::LocalGet(pointer), I::I32Sub, I32Const(2), I::I32Mul, I32Const(2), I::I32Add, I::LocalSet(capacity)]);
			s.emit_text_allocation(f, capacity, destination);
			Self::emit_list(f, &[I::LocalGet(destination), I::LocalSet(out)]);
			write(f, I32Const(QUOTE_BYTE));
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(pointer), I::LocalGet(end), I::I32GeU, I::BrIf(1)]);
			Self::emit_list(f, &[I::LocalGet(pointer), I::I32Load8U(BYTE), I::LocalTee(byte), I32Const(QUOTE_BYTE), I::I32Eq]);
			Self::emit_list(f, &[I::LocalGet(byte), I32Const(BACKSLASH_BYTE), I::I32Eq, I::I32Or, I::If(BlockType::Empty)]);
			write(f, I32Const(BACKSLASH_BYTE));
			f.instruction(&I::End);
			write(f, I::LocalGet(byte));
			Self::emit_list(f, &[I::LocalGet(pointer), I32Const(1), I::I32Add, I::LocalSet(pointer), I::Br(0), I::End, I::End]);
			write(f, I32Const(QUOTE_BYTE));
			Self::emit_list(f, &[I::LocalGet(destination), I::LocalGet(out), I::LocalGet(destination), I::I32Sub]);
			s.call(f, "new_text");
		});
	}

	/// The table list_text joins an entry by: the written texts (operators::written_operator) of the operators up to
	/// the bound the program's Keys need, then `:` for any code above, in one run of bytes, and for each its offset in
	/// the run and length as two bytes; returns (run, table). A hello world carries `:` alone (web::test_bundle_budget)
	fn allocate_operator_texts(&mut self) -> (u32, u32) {
		let bound = self.written_operator_bound as usize;
		let texts: Vec<String> = (0..=bound).map(crate::operators::written_operator).chain([crate::operators::Op::Colon.as_str().to_string()]).collect();
		let mut table = vec![];
		let mut offset = 0;
		for text in &texts {
			table.extend([u8::try_from(offset).expect("operator texts within 255 bytes"), text.len() as u8]);
			offset += text.len();
		}
		(self.allocate_bytes("operator_texts", texts.concat().as_bytes()), self.allocate_bytes("operator_table", &table))
	}

	fn emit_joining(&mut self, name: &'static str, nested: bool) {
		self.emit_text_quoted();
		self.emit_text_heap_global();
		self.emit_int_to_decimal();
		self.emit_exact_text();
		self.emit_float_text(); // after int_to_decimal, which it calls
		let float_box = self.type_manager.f64_box_type;
		let exact_numbers = self.should_emit_function(crate::wasm_emitter::exact::EXACT_TEXT);
		let texts = [self.allocate_string("["), self.allocate_string("]"), self.allocate_string(" ")];
		let map_texts = [self.allocate_string("{"), self.allocate_string("}")];
		// only a program whose Keys hold operators beyond `:` carries their texts (web::test_bundle_budget)
		let operator_texts = (self.written_operator_bound > op_to_code(&Op::Colon)).then(|| self.allocate_operator_texts());
		let instance_texts = [self.allocate_string(""), self.allocate_string(Op::Colon.as_str())];
		let empty_text = self.allocate_string(EMPTY_TEXT);
		let bool_texts = [self.allocate_string(crate::node::NO), self.allocate_string(crate::node::YES)];
		let i64_box = self.type_manager.i64_box_type;
		let own_index = self.next_func_idx; // list_text joins a nested list by calling itself
		let (node_ref, nullable) = (Ref(self.node_ref(false)), Ref(self.node_ref(true)));
		let node_type = self.type_manager.node_type;
		let mut locals = vec![nullable, nullable];
		locals.extend([ValType::I32; 4]);
		locals.extend([ValType::I64, nullable, ValType::I32]);
		self.runtime_function(name, vec![nullable, node_ref], vec![node_ref], locals, |s, f| {
			let (cell, element) = (2, 3);
			let (bound, address, position, is_first, number, entry_value, operator_entry) = (4, 5, 6, 7, 8, 9, 10);
			let is_kind = |f: &mut Function, kind: Kind| {
				s.emit_field(f, element, 0);
				Self::emit_list(f, &[I::I64Const(kind as i64), I::I64Eq]);
			};
			let next_element = |f: &mut Function| {
				s.emit_field(f, cell, 1);
				Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(node_type)), I::LocalSet(element)]);
				s.emit_codepoint_as_text(f, element); // a character joins as its UTF-8 bytes
				if nested { // a text or a character in a container is quoted (P126): `["a" "bc"]`
					s.emit_field(f, element, 0);
					Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::Text as i64), I::I64Eq]);
					Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(element), I::RefAsNonNull]);
					s.call(f, TEXT_QUOTED);
					Self::emit_list(f, &[I::LocalSet(element), I::End]);
				}
				// a bool joins as true or false (card bool-type)
				s.emit_field(f, element, 0);
				Self::emit_list(f, &[I::I64Const(crate::type_kinds::BOOL_KIND), I::I64Eq, I::If(BlockType::Empty)]);
				s.emit_field(f, element, 1);
				Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(i64_box)), I::StructGet { struct_type_index: i64_box, field_index: 0 }, I::I64Eqz, I::If(BlockType::Result(node_ref))]);
				Self::emit_list(f, &[I32Const(bool_texts[0].0 as i32), I32Const(bool_texts[0].1 as i32)]);
				s.call(f, "new_text");
				f.instruction(&I::Else);
				Self::emit_list(f, &[I32Const(bool_texts[1].0 as i32), I32Const(bool_texts[1].1 as i32)]);
				s.call(f, "new_text");
				Self::emit_list(f, &[I::End, I::LocalSet(element), I::End]);
				// an Error joins as its message (card catch-message): `print e`
				is_kind(f, Kind::Error);
				Self::emit_list(f, &[I::If(BlockType::Empty), I::I64Const(Kind::Text as i64)]);
				s.emit_field(f, element, 1);
				Self::emit_list(f, &[I::RefNull(HeapType::Concrete(node_type)), I::StructNew(node_type), I::LocalSet(element), I::End]);
				// list_text: a nested list as its literal, "[" + list_text(item, " ") + "]", a map in braces
				if nested {
					let [open, close, space] = texts;
					let [open_map, close_map] = map_texts;
					let new_text = |f: &mut Function, (pointer, length): (u32, u32)| {
						Self::emit_list(f, &[I32Const(pointer as i32), I32Const(length as i32)]);
						s.call(f, "new_text");
					};
					let masked_kind_is = |f: &mut Function, kind: Kind| {
						s.emit_field(f, element, 0);
						Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(kind as i64), I::I64Eq]);
					};
					// a symbol (a map's key) as its name
					is_kind(f, Kind::Symbol);
					Self::emit_list(f, &[I::If(BlockType::Empty), I::I64Const(Kind::Text as i64)]);
					s.emit_field(f, element, 1);
					Self::emit_list(f, &[I::RefNull(HeapType::Concrete(node_type)), I::StructNew(node_type), I::LocalSet(element), I::End]);
					// an entry key:value as list_text([key value], ":"), ø for a missing value
					masked_kind_is(f, Kind::Key);
					f.instruction(&I::If(BlockType::Empty));
					Self::emit_list(f, &[I::I64Const(SQUARE_LIST_KIND)]);
					s.emit_field(f, element, 1);
					Self::emit_list(f, &[I::I64Const(SQUARE_LIST_KIND)]);
					s.emit_field(f, element, 2);
					Self::emit_list(f, &[I::RefIsNull, I::If(BlockType::Result(node_ref))]);
					s.call(f, "new_empty");
					f.instruction(&I::Else);
					s.emit_field(f, element, 2);
					f.instruction(&I::LocalSet(entry_value));
					s.emit_entry_in_braces(f, entry_value);
					Self::emit_list(f, &[I::End, I::RefNull(HeapType::Concrete(node_type)), I::StructNew(node_type), I::StructNew(node_type)]);
					// joined by its operator as written: `x+1`, an instance `point{x:1}` without one (P123)
					s.emit_field(f, element, 0);
					Self::emit_list(f, &[I::I64Const(KIND_BITS), I::I64ShrU, I::I64Const(OP_INFO_MASK), I::I64And]);
					if let Some((run, table)) = operator_texts {
						let last_code = I32Const(s.written_operator_bound as i32 + 1);
						Self::emit_list(f, &[I::I32WrapI64, I::LocalTee(operator_entry), last_code.clone(), I::LocalGet(operator_entry), last_code, I::I32LtU, I::Select]);
						Self::emit_list(f, &[I32Const(OPERATOR_ENTRY_BYTES), I::I32Mul, I32Const(table as i32), I::I32Add, I::LocalTee(operator_entry)]);
						Self::emit_list(f, &[I::I32Load8U(BYTE), I32Const(run as i32), I::I32Add, I::LocalGet(operator_entry), I::I32Load8U(MemArg { offset: 1, ..BYTE })]);
						s.call(f, "new_text");
					} else {
						let [none, colon] = instance_texts;
						Self::emit_list(f, &[I::I64Const(op_to_code(&Op::None)), I::I64Eq, I::If(BlockType::Result(node_ref))]);
						new_text(f, none);
						f.instruction(&I::Else);
						new_text(f, colon);
						f.instruction(&I::End);
					}
					Self::emit_list(f, &[I::Call(own_index), I::LocalSet(element), I::End]);
					// a curly list is a map: its bracket info, above the kind (other info may sit higher still)
					let is_map = |f: &mut Function| {
						s.emit_field(f, element, 0);
						Self::emit_list(f, &[I::I64Const(KIND_BITS), I::I64ShrU, I::I64Const(BRACKET_INFO_MASK), I::I64And, I::I64Const(CURLY_BRACKET_INFO), I::I64Eq]);
					};
					masked_kind_is(f, Kind::List);
					f.instruction(&I::If(BlockType::Empty));
					is_map(f);
					f.instruction(&I::If(BlockType::Result(node_ref)));
					new_text(f, open_map);
					f.instruction(&I::Else);
					new_text(f, open);
					f.instruction(&I::End);
					f.instruction(&I::LocalGet(element));
					new_text(f, space);
					f.instruction(&I::Call(own_index));
					s.call(f, super::text_builtins::TEXT_CONCAT);
					is_map(f);
					f.instruction(&I::If(BlockType::Result(node_ref)));
					new_text(f, close_map);
					f.instruction(&I::Else);
					new_text(f, close);
					f.instruction(&I::End);
					s.call(f, super::text_builtins::TEXT_CONCAT);
					Self::emit_list(f, &[I::LocalSet(element), I::End]);
					// a ø item reads as ø, as Node::serialize writes it
					is_kind(f, Kind::Empty);
					f.instruction(&I::If(BlockType::Empty));
					Self::emit_list(f, &[I32Const(empty_text.0 as i32), I32Const(empty_text.1 as i32)]);
					s.call(f, "new_text");
					Self::emit_list(f, &[I::LocalSet(element), I::End]);
				}
				is_kind(f, Kind::Float);
				f.instruction(&I::If(BlockType::Empty));
				s.emit_field(f, element, 1);
				Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(float_box)), I::StructGet { struct_type_index: float_box, field_index: 0 }]);
				s.call(f, super::float_text::FLOAT_TEXT);
				Self::emit_list(f, &[I::LocalSet(element), I::End]);
				if exact_numbers { // a big integer or a ratio joins as its exact text, a fixnum by int_to_decimal below
					is_kind(f, Kind::Int);
					f.instruction(&I::If(BlockType::Empty));
					s.emit_int_value_of(f, element);
					f.instruction(&I::LocalSet(number));
					s.emit_fixnum_test(f, &[number]);
					Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty), I::LocalGet(number)]);
					s.call(f, crate::wasm_emitter::exact::EXACT_TEXT);
					Self::emit_list(f, &[I::LocalSet(element), I::End, I::End]);
				}
			};
			let loop_over_cells = |f: &mut Function| {
				Self::emit_list(f, &[I::LocalGet(0), I::LocalSet(cell), I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(cell), I::RefIsNull, I::BrIf(1)]);
				next_element(f);
			};
			let end_loop = |f: &mut Function| {
				s.emit_field(f, cell, 2);
				Self::emit_list(f, &[I::LocalSet(cell), I::Br(0), I::End, I::End]);
			};
			// append the bytes of a text node: memory.copy(position, ptr, len); position += len
			let append_text = |f: &mut Function, text: u32| {
				f.instruction(&I::LocalGet(position));
				s.emit_text_field(f, text, 0);
				s.emit_text_field(f, text, 1);
				Self::emit_list(f, &[COPY_BYTES, I::LocalGet(position)]);
				s.emit_text_field(f, text, 1);
				Self::emit_list(f, &[I::I32Add, I::LocalSet(position)]);
			};
			s.emit_empty_as_null(f, 0);
			s.emit_require_list(f);
			s.emit_codepoint_as_text(f, 1);
			s.emit_require_kind(f, 1, Kind::Text, "not_a_text");
			// pass 1: an upper bound of the bytes: every item, and a separator after it
			loop_over_cells(f);
			is_kind(f, Kind::Text);
			f.instruction(&I::If(BlockType::Result(ValType::I32)));
			s.emit_text_field(f, element, 1);
			f.instruction(&I::Else);
			is_kind(f, Kind::Int);
			Self::emit_list(f, &[I::If(BlockType::Result(ValType::I32)), I32Const(MAX_INT_BYTES), I::Else]);
			s.call(f, "not_a_joinable_item");
			Self::emit_list(f, &[I32Const(0), I::End, I::End]);
			Self::emit_list(f, &[I::LocalGet(bound), I::I32Add]);
			s.emit_text_field(f, 1, 1);
			Self::emit_list(f, &[I::I32Add, I::LocalSet(bound)]);
			end_loop(f);
			s.emit_text_allocation(f, bound, address);
			Self::emit_list(f, &[I::LocalGet(address), I::LocalSet(position), I32Const(1), I::LocalSet(is_first)]);
			// pass 2: write
			loop_over_cells(f);
			Self::emit_list(f, &[I::LocalGet(is_first), I::I32Eqz, I::If(BlockType::Empty)]);
			append_text(f, 1);
			Self::emit_list(f, &[I::End, I32Const(0), I::LocalSet(is_first)]);
			is_kind(f, Kind::Text);
			f.instruction(&I::If(BlockType::Empty));
			append_text(f, element);
			f.instruction(&I::Else);
			Self::emit_list(f, &[I::LocalGet(position), I::LocalGet(position)]);
			s.emit_int_value_of(f, element);
			Self::emit_list(f, &[I::Call(s.func_index("int_to_decimal")), I::I32Add, I::LocalSet(position), I::End]);
			end_loop(f);
			Self::emit_list(f, &[I::LocalGet(address), I::LocalGet(position), I::LocalGet(address), I::I32Sub]);
			s.call(f, "new_text");
		});
	}

	/// node_slice(x, start, end): the items start…end-1 of a list, or the characters of a text as a text. Indexes are 0-based
	/// and clamped to the length; a negative one is out of range; ø is the start or the end itself.
	fn emit_node_slice(&mut self) {
		if !self.should_emit_function(NODE_SLICE) {
			return;
		}
		let (node_ref, nullable) = (Ref(self.node_ref(false)), Ref(self.node_ref(true)));
		let node_type = self.type_manager.node_type;
		let locals = vec![ValType::I32, ValType::I64, ValType::I64, ValType::I64, ValType::I64, nullable, nullable];
		self.runtime_function(NODE_SLICE, vec![nullable, nullable, nullable], vec![node_ref], locals, |s, f| {
			let (list, start, end) = (0, 1, 2);
			let (is_text, length, from, to, index, cell, sliced) = (3, 4, 5, 6, 7, 8, 9);
			let walk_cells = |f: &mut Function| {
				Self::emit_list(f, &[I::LocalGet(list), I::LocalSet(cell), I::I64Const(0), I::LocalSet(index)]);
				Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(cell), I::RefIsNull, I::BrIf(1)]);
			};
			let next_cell = |f: &mut Function| {
				s.emit_field(f, cell, 2);
				Self::emit_list(f, &[I::LocalSet(cell), I::LocalGet(index), I::I64Const(1), I::I64Add, I::LocalSet(index), I::Br(0), I::End, I::End]);
			};
			// bound = the int in `bound`, or `default` for ø; never negative (no wrap around, Footguns.md); at most length
			let clamped_bound = |f: &mut Function, bound: u32, default: u32, target: u32| {
				s.emit_empty_as_null(f, bound);
				Self::emit_list(f, &[I::LocalGet(bound), I::RefIsNull, I::If(BlockType::Result(ValType::I64)), I::LocalGet(default), I::Else]);
				s.emit_int_value_of(f, bound);
				Self::emit_list(f, &[I::End, I::LocalSet(target)]);
				s.emit_require_integral(f, target);
				Self::emit_list(f, &[I::LocalGet(target), I::I64Const(0), I::I64LtS]);
				s.emit_fail_if(f, "index_out_of_range");
				Self::emit_list(f, &[I::LocalGet(target), I::LocalGet(length), I::LocalGet(target), I::LocalGet(length), I::I64LtS, I::Select, I::LocalSet(target)]);
			};
			s.emit_text_as_characters(f, list, is_text);
			s.emit_empty_as_null(f, list);
			s.emit_require_list(f);
			walk_cells(f);
			next_cell(f);
			Self::emit_list(f, &[I::LocalGet(index), I::LocalSet(length), I::I64Const(0), I::LocalSet(from)]);
			clamped_bound(f, start, from, from);
			clamped_bound(f, end, length, to);
			// the cells from…to-1, collected backwards
			walk_cells(f);
			Self::emit_list(f, &[I::LocalGet(index), I::LocalGet(to), I::I64GeS, I::BrIf(1)]);
			Self::emit_list(f, &[I::LocalGet(index), I::LocalGet(from), I::I64GeS, I::If(BlockType::Empty)]);
			s.emit_field(f, cell, 0);
			s.emit_field(f, cell, 1);
			Self::emit_list(f, &[I::LocalGet(sliced), I::StructNew(node_type), I::LocalSet(sliced), I::End]);
			next_cell(f);
			f.instruction(&I::LocalGet(sliced));
			s.call(f, "list_reverse");
			f.instruction(&I::LocalSet(sliced));
			s.emit_characters_as_text(f, sliced, is_text);
		});
	}

	/// A text or character in `local` replaced by the list of its characters, `is_text` set: words on lists work on texts
	fn emit_text_as_characters(&self, f: &mut Function, local: u32, is_text: u32) {
		Self::emit_list(f, &[I::LocalGet(local), I::RefIsNull, I::I32Eqz, I::If(BlockType::Empty)]);
		for kind in [Kind::Text, Kind::Codepoint] {
			self.emit_field(f, local, 0);
			Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(kind as i64), I::I64Eq, I::If(BlockType::Empty)]);
			Self::emit_list(f, &[I::LocalGet(local), I::RefAsNonNull]);
			self.call(f, "text_chars");
			Self::emit_list(f, &[I::LocalSet(local), I::I32Const(1), I::LocalSet(is_text), I::End]);
		}
		f.instruction(&I::End);
	}

	/// The list in `local` (not null), joined back into a text when `is_text` (emit_text_as_characters)
	fn emit_characters_as_text(&self, f: &mut Function, local: u32, is_text: u32) {
		let node_ref = Ref(self.node_ref(false));
		Self::emit_list(f, &[I::LocalGet(is_text), I::If(BlockType::Result(node_ref)), I::LocalGet(local), I::I32Const(0), I::I32Const(0)]);
		self.call(f, "new_text");
		self.call(f, "list_join");
		Self::emit_list(f, &[I::Else, I::LocalGet(local), I::RefAsNonNull, I::End]);
	}

	/// Push the node in `local` (not null), a `key:value` entry as the one-entry map `{key:value}` it stands for, so its
	/// text keeps the braces: `{a:{b:1}}` is the entry a:(b:1) at run time and would write a:b:1. A tag, an entry whose
	/// value is a map (`data point{x:1}`), writes as an instance does, its name before its braces (card data-tag)
	pub(super) fn emit_entry_in_braces(&self, f: &mut Function, local: u32) {
		let node_type = self.type_manager.node_type;
		let entry_kind_mask = (OP_INFO_MASK << KIND_BITS) | KIND_MASK;
		let colon_entry_kind = (crate::operators::op_to_code(&crate::operators::Op::Colon) << KIND_BITS) | KEY_KIND;
		self.emit_field(f, local, 0);
		Self::emit_list(f, &[I::I64Const(entry_kind_mask), I::I64And, I::I64Const(colon_entry_kind), I::I64Eq, I::If(BlockType::Result(Ref(self.node_ref(false))))]);
		if !self.writes_tags {
			Self::emit_list(f, &[I::I64Const(CURLY_LIST_KIND), I::LocalGet(local), I::RefNull(HeapType::Concrete(node_type)), I::StructNew(node_type)]);
			return Self::emit_list(f, &[I::Else, I::LocalGet(local), I::RefAsNonNull, I::End]);
		}
		self.emit_field(f, local, 2);
		Self::emit_list(f, &[I::RefIsNull, I::If(BlockType::Result(ValType::I64)), I::I64Const(0), I::Else]);
		self.emit_field(f, local, 2);
		Self::emit_list(f, &[I::StructGet { struct_type_index: node_type, field_index: 0 }, I::End]);
		let list_kind_mask = (BRACKET_INFO_MASK << KIND_BITS) | KIND_MASK;
		Self::emit_list(f, &[I::I64Const(list_kind_mask), I::I64And, I::I64Const(CURLY_LIST_KIND), I::I64Eq, I::If(BlockType::Result(Ref(self.node_ref(false))))]);
		Self::emit_list(f, &[I::I64Const(KEY_KIND)]);
		self.emit_field(f, local, 1);
		self.emit_field(f, local, 2);
		Self::emit_list(f, &[I::StructNew(node_type), I::Else]);
		Self::emit_list(f, &[I::I64Const(CURLY_LIST_KIND), I::LocalGet(local), I::RefNull(HeapType::Concrete(node_type)), I::StructNew(node_type), I::End]);
		Self::emit_list(f, &[I::Else, I::LocalGet(local), I::RefAsNonNull, I::End]);
	}

	/// instance_copy(node, shallow): a copy of any object, keeping its class (P205/P207). Deep unless `shallow` is true:
	/// every instance, map, list and entry inside is new, each copied once however often it is held (cycles too);
	/// numbers, texts and resources (closures, data) are shared, they never change. Shallow: the object's own entries
	/// and cells are new, the values in them shared
	pub(super) fn emit_instance_copy(&mut self) {
		if !self.should_emit_function(crate::library_words::INSTANCE_COPY) {
			return;
		}
		let (node_ref, nullable) = (Ref(self.node_ref(false)), Ref(self.node_ref(true)));
		let node_type = self.type_manager.node_type;
		let node_heap = HeapType::Concrete(node_type);
		let has_instances = !self.ctx.type_registry.types().is_empty();
		let next_index = |s: &Self| s.ctx.func_registry.import_count() + s.ctx.func_registry.code_count();
		// is the node in `local` an entry or cell (a Key or List), else leave the function giving it back
		let return_unless_structure = |f: &mut Function, local: u32, kind: u32| {
			Self::emit_list(f, &[I::LocalGet(local), I::StructGet { struct_type_index: node_type, field_index: 0 }, I::I64Const(KIND_MASK), I::I64And, I::LocalTee(kind)]);
			Self::emit_list(f, &[I::I64Const(KEY_KIND), I::I64Ne, I::LocalGet(kind), I::I64Const(Kind::List as i64), I::I64Ne, I::I32And]);
			Self::emit_list(f, &[I::LocalGet(kind), I::I64Const(Kind::Empty as i64), I::I64Ne, I::I32And, I::If(BlockType::Empty), I::LocalGet(local), I::Return, I::End]);
		};
		let entry_copy = |s: &Self, f: &mut Function, local: u32| {
			for field_index in 0..3 {
				s.emit_field(f, local, field_index);
			}
			f.instruction(&I::StructNew(node_type));
		};

		// copy_spine(fields): new entries and cells, the values in them shared
		let spine = next_index(self);
		self.runtime_function("copy_spine", vec![nullable], vec![nullable], vec![ValType::I64, Self::any_ref(), nullable], |s, f| {
			let (node, kind, item, entry) = (0, 1, 2, 3);
			Self::emit_list(f, &[I::LocalGet(node), I::RefIsNull, I::If(BlockType::Empty), I::RefNull(node_heap), I::Return, I::End]);
			return_unless_structure(f, node, kind);
			Self::emit_list(f, &[I::LocalGet(kind), I::I64Const(Kind::List as i64), I::I64Ne, I::If(BlockType::Empty)]);
			entry_copy(s, f, node);
			Self::emit_list(f, &[I::Return, I::End]);
			// a cell: its item new when it is an entry
			s.emit_field(f, node, 1);
			Self::emit_list(f, &[I::LocalTee(item), I::RefTestNonNull(node_heap), I::If(BlockType::Empty), I::LocalGet(item), I::RefCastNonNull(node_heap), I::LocalTee(entry)]);
			Self::emit_list(f, &[I::StructGet { struct_type_index: node_type, field_index: 0 }, I::I64Const(KIND_MASK), I::I64And, I::I64Const(KEY_KIND), I::I64Eq, I::If(BlockType::Empty)]);
			entry_copy(s, f, entry);
			Self::emit_list(f, &[I::LocalSet(item), I::End, I::End]);
			s.emit_field(f, node, 0);
			f.instruction(&I::LocalGet(item));
			s.emit_field(f, node, 2);
			Self::emit_list(f, &[I::Call(spine), I::StructNew(node_type)]);
		});
		assert_eq!(self.func_index("copy_spine"), spine, "recursive call index");

		// deep_copy(node, memo): memo.value lists `original:copy` pairs, so a part held twice is copied once
		let deep = next_index(self);
		self.runtime_function("deep_copy", vec![nullable, node_ref], vec![nullable], vec![ValType::I64, nullable, nullable, nullable, Self::any_ref()], |s, f| {
			let (node, memo, kind, cell, pair, copy, data) = (0, 1, 2, 3, 4, 5, 6);
			Self::emit_list(f, &[I::LocalGet(node), I::RefIsNull, I::If(BlockType::Empty), I::RefNull(node_heap), I::Return, I::End]);
			return_unless_structure(f, node, kind);
			s.emit_field(f, memo, 2);
			Self::emit_list(f, &[I::LocalSet(cell), I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(cell), I::RefIsNull, I::BrIf(1)]);
			s.emit_field(f, cell, 1);
			Self::emit_list(f, &[I::RefCastNonNull(node_heap), I::LocalTee(pair)]);
			Self::emit_list(f, &[I::StructGet { struct_type_index: node_type, field_index: 1 }, I::RefCastNonNull(node_heap), I::LocalGet(node), I::RefEq, I::If(BlockType::Empty)]);
			s.emit_field(f, pair, 2);
			Self::emit_list(f, &[I::Return, I::End]);
			s.emit_field(f, cell, 2);
			Self::emit_list(f, &[I::LocalSet(cell), I::Br(0), I::End, I::End]);
			s.emit_field(f, node, 0);
			Self::emit_list(f, &[I::RefNull(HeapType::Abstract { shared: false, ty: AbstractHeapType::Any }), I::RefNull(node_heap), I::StructNew(node_type), I::LocalSet(copy)]);
			Self::emit_list(f, &[I::LocalGet(memo), I::I64Const(Kind::List as i64), I::I64Const(KEY_KIND), I::LocalGet(node), I::LocalGet(copy), I::StructNew(node_type)]);
			s.emit_field(f, memo, 2);
			Self::emit_list(f, &[I::StructNew(node_type), I::StructSet { struct_type_index: node_type, field_index: 2 }]);
			s.emit_field(f, node, 1);
			Self::emit_list(f, &[I::LocalSet(data), I::LocalGet(copy), I::LocalGet(data), I::RefTestNonNull(node_heap)]);
			Self::emit_list(f, &[I::If(BlockType::Result(Self::any_ref())), I::LocalGet(data), I::RefCastNonNull(node_heap), I::LocalGet(memo), I::Call(deep), I::Else, I::LocalGet(data), I::End]);
			Self::emit_list(f, &[I::StructSet { struct_type_index: node_type, field_index: 1 }, I::LocalGet(copy)]);
			s.emit_field(f, node, 2);
			Self::emit_list(f, &[I::LocalGet(memo), I::Call(deep), I::StructSet { struct_type_index: node_type, field_index: 2 }, I::LocalGet(copy)]);
		});
		assert_eq!(self.func_index("deep_copy"), deep, "recursive call index");

		self.runtime_function(crate::library_words::INSTANCE_COPY, vec![node_ref, node_ref], vec![node_ref], vec![nullable], |s, f| {
			let (object, shallow, body) = (0, 1, 2);
			f.instruction(&I::LocalGet(shallow));
			s.call(f, super::equality::IS_TRUTHY);
			f.instruction(&I::If(BlockType::Empty));
			if has_instances {
				f.instruction(&I::LocalGet(object));
				s.call(f, super::list_ops::STRUCT_BODY);
				Self::emit_list(f, &[I::LocalTee(body), I::LocalGet(object), I::RefEq, I::I32Eqz, I::If(BlockType::Empty)]);
				s.emit_field(f, object, 0);
				s.emit_field(f, object, 1);
				Self::emit_list(f, &[I::LocalGet(body), I::Call(spine), I::StructNew(node_type), I::Return, I::End]);
			}
			Self::emit_list(f, &[I::LocalGet(object), I::Call(spine), I::RefAsNonNull, I::Return, I::End]);
			Self::emit_list(f, &[I::LocalGet(object), I::I64Const(Kind::Empty as i64), I::RefNull(HeapType::Abstract { shared: false, ty: AbstractHeapType::Any }), I::RefNull(node_heap), I::StructNew(node_type), I::Call(deep), I::RefAsNonNull]);
		});
	}

	/// field_set_in_place(fields, name, value) -> found: the entry `name:…` among an object's fields (one entry, or a
	/// cons list of them) gets the value in place
	fn emit_field_set_in_place(&mut self) {
		let (node_ref, nullable) = (Ref(self.node_ref(false)), Ref(self.node_ref(true)));
		let node_type = self.type_manager.node_type;
		self.runtime_function(FIELD_SET_IN_PLACE, vec![nullable, node_ref, node_ref], vec![ValType::I32], vec![nullable, ValType::I64], |s, f| {
			let (cells, name, value, entry, kind) = (0, 1, 2, 3, 4);
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
			Self::emit_list(f, &[I::LocalGet(cells), I::RefIsNull, I::BrIf(1)]);
			s.emit_field(f, cells, 0);
			Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::LocalTee(kind), I::I64Const(KEY_KIND), I::I64Eq]);
			// one field: the fields are that entry; else the entry is the cell's item
			Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(cells), I::LocalSet(entry), I::Else]);
			s.emit_field(f, cells, 1);
			Self::emit_list(f, &[I::RefCastNullable(HeapType::Concrete(node_type)), I::LocalSet(entry), I::End]);
			Self::emit_list(f, &[I::LocalGet(entry), I::RefIsNull, I::I32Eqz, I::If(BlockType::Empty)]);
			Self::emit_list(f, &[I::LocalGet(entry), I::RefAsNonNull, I::LocalGet(name)]);
			s.call(f, "map_entry_has_key");
			Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(entry), I::LocalGet(value)]);
			Self::emit_list(f, &[I::StructSet { struct_type_index: node_type, field_index: 2 }, I32Const(1), I::Return, I::End, I::End]);
			Self::emit_list(f, &[I::LocalGet(kind), I::I64Const(KEY_KIND), I::I64Eq, I::BrIf(1)]);
			s.emit_field(f, cells, 2);
			Self::emit_list(f, &[I::LocalSet(cells), I::Br(0), I::End, I::End, I32Const(0)]);
		});
	}

	/// fields_grow(fields, entry): the entry added to an object's fields in place, so every holder sees it (P200b). The
	/// empty object becomes the entry itself, a single entry a cons list of the two; meta entries `@name:value` stay
	/// behind every field, so a walk by position never meets one
	fn emit_fields_grow(&mut self) {
		let node_ref = Ref(self.node_ref(false));
		let nullable = Ref(self.node_ref(true));
		let node_type = self.type_manager.node_type;
		let set = |field_index: u32| I::StructSet { struct_type_index: node_type, field_index };
		self.runtime_function(FIELDS_GROW, vec![node_ref, node_ref], vec![], vec![ValType::I64, nullable], |s, f| {
			let (fields, entry, kind, cells) = (0, 1, 2, 3);
			s.emit_field(f, fields, 0);
			Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::LocalSet(kind)]);
			Self::emit_list(f, &[I::LocalGet(kind), I::I64Const(Kind::Empty as i64), I::I64Eq, I::If(BlockType::Empty)]);
			for field_index in 0..3 {
				f.instruction(&I::LocalGet(fields));
				s.emit_field(f, entry, field_index);
				f.instruction(&set(field_index));
			}
			Self::emit_list(f, &[I::Return, I::End]);
			// a single entry moves into a new first cell
			Self::emit_list(f, &[I::LocalGet(kind), I::I64Const(KEY_KIND), I::I64Eq, I::If(BlockType::Empty), I::LocalGet(fields)]);
			for field_index in 0..3 {
				s.emit_field(f, fields, field_index);
			}
			Self::emit_list(f, &[I::StructNew(node_type), set(1), I::LocalGet(fields), I::I64Const(CURLY_LIST_KIND), set(0)]);
			Self::emit_list(f, &[I::LocalGet(fields), I::RefNull(HeapType::Concrete(node_type)), set(2), I::I64Const(Kind::List as i64), I::LocalSet(kind), I::End]);
			Self::emit_list(f, &[I::LocalGet(kind), I::I64Const(Kind::List as i64), I::I64Ne, I::If(BlockType::Empty)]);
			s.call(f, "not_an_object");
			Self::emit_list(f, &[I::End, I::LocalGet(fields), I::LocalSet(cells), I::Loop(BlockType::Empty)]);
			// in front of the first meta entry: the cell keeps the new field, a new cell after it the meta entry
			s.emit_field(f, cells, 1);
			s.call(f, super::equality::IS_META_ENTRY);
			f.instruction(&I::LocalGet(entry));
			s.call(f, super::equality::IS_META_ENTRY);
			Self::emit_list(f, &[I::I32Eqz, I::I32And, I::If(BlockType::Empty), I::LocalGet(cells)]);
			for field_index in 0..3 {
				s.emit_field(f, cells, field_index);
			}
			Self::emit_list(f, &[I::StructNew(node_type), set(2), I::LocalGet(cells), I::LocalGet(entry), set(1), I::Return, I::End]);
			// at the end: a new last cell
			s.emit_field(f, cells, 2);
			Self::emit_list(f, &[I::RefIsNull, I::If(BlockType::Empty), I::LocalGet(cells)]);
			s.emit_field(f, cells, 0);
			Self::emit_list(f, &[I::LocalGet(entry), I::RefNull(HeapType::Concrete(node_type)), I::StructNew(node_type), set(2), I::Return, I::End]);
			s.emit_field(f, cells, 2);
			Self::emit_list(f, &[I::LocalSet(cells), I::Br(0), I::End]);
		});
	}

	/// field_with(object, name, value): the field `name` of the object set to `value` in place, added at the end when it
	/// is new; gives back the object itself (objects are references, P200/P200b). A one-entry object `{a:1}` is the entry
	/// node itself; an instance `T:{fields}` changes its fields.
	pub(super) fn emit_field_with(&mut self) {
		if !self.should_emit_function("field_with") {
			return;
		}
		let node_ref = Ref(self.node_ref(false));
		let colon = crate::operators::op_to_code(&crate::operators::Op::Colon);
		self.emit_field_set_in_place();
		self.emit_fields_grow();
		let has_instances = !self.ctx.type_registry.types().is_empty();
		self.runtime_function("field_with", vec![node_ref, node_ref, node_ref], vec![node_ref], vec![node_ref, ValType::I64], |s, f| {
			let (object, name, value, fields, kind) = (0, 1, 2, 3, 4);
			f.instruction(&I::LocalGet(object));
			if has_instances {
				s.call(f, super::list_ops::STRUCT_BODY);
			}
			f.instruction(&I::LocalSet(fields));
			s.emit_field(f, fields, 0);
			Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::LocalSet(kind)]);
			for object_kind in [Kind::Empty, Kind::Key, Kind::List] {
				Self::emit_list(f, &[I::LocalGet(kind), I::I64Const(object_kind as i64), I::I64Ne]);
			}
			Self::emit_list(f, &[I::I32And, I::I32And, I::If(BlockType::Empty)]);
			s.call(f, "not_an_object");
			f.instruction(&I::End);
			// the name as a symbol like the names of an object literal
			f.instruction(&I::LocalGet(name));
			s.call(f, super::list_ops::MAP_KEY_NAME);
			f.instruction(&I::LocalSet(name));
			s.emit_require_kind(f, name, Kind::Symbol, "not_a_text");
			Self::emit_list(f, &[I::LocalGet(fields), I::LocalGet(name), I::LocalGet(value)]);
			s.call(f, FIELD_SET_IN_PLACE);
			Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty), I::LocalGet(fields), I::LocalGet(name), I::LocalGet(value), I::I64Const(colon)]);
			s.call(f, "new_key");
			f.instruction(&I::RefAsNonNull);
			s.call(f, FIELDS_GROW);
			Self::emit_list(f, &[I::End, I::LocalGet(object)]);
		});
	}
}

/// A tag quoted as data anywhere in the program (`p = data point{x:1}`): an entry whose value is a map. Markup
/// (`p{ "hello" }`) has the same shape but is no data, so a page's list_text needs no tag check
pub(super) fn mentions_quoted_tag(node: &Node) -> bool {
	match node.drop_meta() {
		Node::List(items, _, _) if crate::lowering::run_time_blocks::is_data(node) => items.iter().skip(1).any(mentions_tag),
		Node::Key(left, _, right) => mentions_quoted_tag(left) || mentions_quoted_tag(right),
		Node::List(items, _, _) => items.iter().any(mentions_quoted_tag),
		_ => false,
	}
}

fn mentions_tag(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(_, Op::Colon, value) if matches!(value.drop_meta(), Node::List(_, Bracket::Curly, _)) => true,
		Node::Key(left, _, right) => mentions_tag(left) || mentions_tag(right),
		Node::List(items, _, _) => items.iter().any(mentions_tag),
		_ => false,
	}
}
