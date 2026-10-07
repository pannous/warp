//! Runtime functions of the library words (`reverse`, `sort`, `upper`, `lower`, `split`, `join`), see library_words.rs

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
const INSTANCE_OP_CODE: i64 = 0;
use crate::wasm_emitter::layout::BYTE;

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
pub const LIBRARY_FUNCTIONS: [(&str, &str); 17] = [
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
	("reverse", "list_reverse"),
	("sort", "list_sort"),
	("upper", "text_upper"),
	("lower", "text_lower"),
	("split", "text_split"),
	("join", "list_join"),
	(crate::library_words::SLICE, NODE_SLICE),
];
pub const NODE_SLICE: &str = "node_slice";
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
		self.emit_list_sort();
		self.emit_text_case("text_upper", true);
		self.emit_text_case("text_lower", false);
		self.emit_text_split();
		self.emit_list_join();
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

		self.runtime_function("list_sort", vec![nullable], vec![node_ref], vec![nullable, nullable], |s, f| {
			let (sorted, element) = (1, 2);
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

	fn emit_joining(&mut self, name: &'static str, nested: bool) {
		self.emit_text_quoted();
		self.emit_text_heap_global();
		self.emit_int_to_decimal();
		self.emit_exact_text();
		self.emit_float_text(); // after int_to_decimal, which it calls
		let float_box = self.type_manager.f64_box_type;
		let exact_numbers = self.should_emit_function(crate::wasm_emitter::exact::EXACT_TEXT);
		let texts = [self.allocate_string("["), self.allocate_string("]"), self.allocate_string(" ")];
		let map_texts = [self.allocate_string("{"), self.allocate_string("}"), self.allocate_string(":")];
		let no_text = self.allocate_string("");
		let empty_text = self.allocate_string(EMPTY_TEXT);
		let own_index = self.next_func_idx; // list_text joins a nested list by calling itself
		let (node_ref, nullable) = (Ref(self.node_ref(false)), Ref(self.node_ref(true)));
		let node_type = self.type_manager.node_type;
		let mut locals = vec![nullable, nullable];
		locals.extend([ValType::I32; 4]);
		locals.extend([ValType::I64, nullable]);
		self.runtime_function(name, vec![nullable, node_ref], vec![node_ref], locals, |s, f| {
			let (cell, element) = (2, 3);
			let (bound, address, position, is_first, number, entry_value) = (4, 5, 6, 7, 8, 9);
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
				// list_text: a nested list as its literal, "[" + list_text(item, " ") + "]", a map in braces
				if nested {
					let [open, close, space] = texts;
					let [open_map, close_map, colon] = map_texts;
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
					// an instance `point{x:1}` (a key without operator) joins its name and fields without one (P123)
					s.emit_field(f, element, 0);
					Self::emit_list(f, &[I::I64Const(KIND_BITS), I::I64ShrU, I::I64Const(OP_INFO_MASK), I::I64And, I::I64Const(INSTANCE_OP_CODE), I::I64Eq]);
					f.instruction(&I::If(BlockType::Result(node_ref)));
					new_text(f, no_text);
					f.instruction(&I::Else);
					new_text(f, colon);
					f.instruction(&I::End);
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
			// a text slices its characters
			Self::emit_list(f, &[I::LocalGet(list), I::RefIsNull, I::I32Eqz, I::If(BlockType::Empty)]);
			for kind in [Kind::Text, Kind::Codepoint] {
				s.emit_field(f, list, 0);
				Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(kind as i64), I::I64Eq, I::If(BlockType::Empty)]);
				Self::emit_list(f, &[I::LocalGet(list), I::RefAsNonNull]);
				s.call(f, "text_chars");
				Self::emit_list(f, &[I::LocalSet(list), I::I32Const(1), I::LocalSet(is_text), I::End]);
			}
			f.instruction(&I::End);
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
			Self::emit_list(f, &[I::LocalSet(sliced), I::LocalGet(is_text), I::If(BlockType::Result(node_ref)), I::LocalGet(sliced), I::I32Const(0), I::I32Const(0)]);
			s.call(f, "new_text");
			s.call(f, "list_join");
			Self::emit_list(f, &[I::Else, I::LocalGet(sliced), I::RefAsNonNull, I::End]);
		});
	}

	/// Push the node in `local` (not null), a `key:value` entry as the one-entry map `{key:value}` it stands for, so its
	/// text keeps the braces: `{a:{b:1}}` is the entry a:(b:1) at run time and would write a:b:1
	pub(super) fn emit_entry_in_braces(&self, f: &mut Function, local: u32) {
		let node_type = self.type_manager.node_type;
		let entry_kind_mask = (OP_INFO_MASK << KIND_BITS) | KIND_MASK;
		let colon_entry_kind = (crate::operators::op_to_code(&crate::operators::Op::Colon) << KIND_BITS) | KEY_KIND;
		self.emit_field(f, local, 0);
		Self::emit_list(f, &[I::I64Const(entry_kind_mask), I::I64And, I::I64Const(colon_entry_kind), I::I64Eq, I::If(BlockType::Result(Ref(self.node_ref(false))))]);
		Self::emit_list(f, &[I::I64Const(CURLY_LIST_KIND), I::LocalGet(local), I::RefNull(HeapType::Concrete(node_type)), I::StructNew(node_type)]);
		Self::emit_list(f, &[I::Else, I::LocalGet(local), I::RefAsNonNull, I::End]);
	}

	/// field_with(object, name, value): a copy of the object with the field `name` set to `value`, added at the end when it is
	/// new (value semantics: the object itself never changes). A one-entry object `{a:1}` is the entry node itself.
	pub(super) fn emit_field_with(&mut self) {
		if !self.should_emit_function("field_with") {
			return;
		}
		let (node_ref, nullable) = (Ref(self.node_ref(false)), Ref(self.node_ref(true)));
		let node_type = self.type_manager.node_type;
		let next_index = |s: &Self| s.ctx.func_registry.import_count() + s.ctx.func_registry.code_count();
		let colon = crate::operators::op_to_code(&crate::operators::Op::Colon);

		// replace_entry(cells, name, entry): the cells with the entry of that name replaced, or `entry` added at the end of
		// the fields: meta entries `@name:value` stay behind every field, so a walk by position never meets one
		let is_meta_entry = super::equality::IS_META_ENTRY;
		let replace_entry = next_index(self);
		self.runtime_function("map_replace_entry", vec![nullable, node_ref, node_ref], vec![node_ref], vec![nullable], |s, f| {
			let (cells, name, entry, head) = (0, 1, 2, 3);
			Self::emit_list(f, &[I::LocalGet(cells), I::RefIsNull, I::If(BlockType::Empty), I::I64Const(CURLY_LIST_KIND), I::LocalGet(entry)]);
			Self::emit_list(f, &[I::RefNull(HeapType::Concrete(node_type)), I::StructNew(node_type), I::Return, I::End]);
			// a new field goes in front of the first meta entry
			s.emit_field(f, cells, 1);
			s.call(f, is_meta_entry);
			f.instruction(&I::LocalGet(entry));
			s.call(f, is_meta_entry);
			Self::emit_list(f, &[I::I32Eqz, I::I32And, I::If(BlockType::Empty)]);
			s.emit_field(f, cells, 0);
			Self::emit_list(f, &[I::LocalGet(entry), I::LocalGet(cells), I::StructNew(node_type), I::Return, I::End]);
			s.emit_field(f, cells, 1);
			Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(node_type)), I::LocalSet(head), I::LocalGet(head), I::RefAsNonNull, I::LocalGet(name)]);
			s.call(f, "map_entry_has_key");
			f.instruction(&I::If(BlockType::Result(node_ref)));
			s.emit_field(f, cells, 0);
			Self::emit_list(f, &[I::LocalGet(entry)]);
			s.emit_field(f, cells, 2);
			f.instruction(&I::StructNew(node_type));
			f.instruction(&I::Else);
			s.emit_field(f, cells, 0);
			s.emit_field(f, cells, 1);
			s.emit_field(f, cells, 2);
			Self::emit_list(f, &[I::LocalGet(name), I::LocalGet(entry), I::Call(replace_entry), I::StructNew(node_type), I::End]);
		});
		assert_eq!(self.func_index("map_replace_entry"), replace_entry, "recursive call index");

		let field_with = next_index(self);
		let has_instances = !self.ctx.type_registry.types().is_empty();
		self.runtime_function("field_with", vec![node_ref, node_ref, node_ref], vec![node_ref], vec![nullable, ValType::I64, nullable], |s, f| {
			let (entry, kind, body) = (3, 4, 5);
			// an instance `T:{fields}` keeps its type: the field is set in its field list
			if has_instances {
				f.instruction(&I::LocalGet(0));
				s.call(f, super::list_ops::STRUCT_BODY);
				Self::emit_list(f, &[I::LocalTee(body), I::LocalGet(0), I::RefEq, I::I32Eqz, I::If(BlockType::Empty)]);
				s.emit_field(f, 0, 0);
				s.emit_field(f, 0, 1);
				Self::emit_list(f, &[I::LocalGet(body), I::RefAsNonNull, I::LocalGet(1), I::LocalGet(2), I::Call(field_with), I::StructNew(node_type), I::Return, I::End]);
			}
			// the entry `name:value`, with the name as a symbol like the names of an object literal
			f.instruction(&I::LocalGet(1));
			s.call(f, super::list_ops::MAP_KEY_NAME);
			f.instruction(&I::LocalSet(1));
			s.emit_require_kind(f, 1, Kind::Symbol, "not_a_text");
			Self::emit_list(f, &[I::LocalGet(1), I::LocalGet(2), I::I64Const(colon)]);
			s.call(f, "new_key");
			f.instruction(&I::LocalSet(entry));
			s.emit_field(f, 0, 0);
			Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::LocalSet(kind)]);
			// the empty object `{}` grows its first entry
			Self::emit_list(f, &[I::LocalGet(kind), I::I64Const(Kind::Empty as i64), I::I64Eq, I::If(BlockType::Empty), I::LocalGet(entry), I::RefAsNonNull, I::Return, I::End]);
			// a single entry: replaced when it has the name, else the two entries as an object
			Self::emit_list(f, &[I::LocalGet(kind), I::I64Const(KEY_KIND), I::I64Eq, I::If(BlockType::Empty), I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "map_entry_has_key");
			Self::emit_list(f, &[I::If(BlockType::Result(node_ref)), I::LocalGet(entry), I::RefAsNonNull, I::Else]);
			// the two entries as an object, a meta entry behind the field
			f.instruction(&I::LocalGet(0));
			s.call(f, super::equality::IS_META_ENTRY);
			f.instruction(&I::LocalGet(entry));
			s.call(f, super::equality::IS_META_ENTRY);
			Self::emit_list(f, &[I::I32Eqz, I::I32And, I::If(BlockType::Result(node_ref))]);
			for (first, second) in [(entry, 0), (0, entry)] {
				Self::emit_list(f, &[I::LocalGet(first), I::LocalGet(second), I::RefNull(HeapType::Concrete(node_type)), I::I64Const(0)]);
				s.call(f, "new_list");
				f.instruction(&I::I64Const(0));
				s.call(f, "new_list");
				if first == entry {
					f.instruction(&I::Else);
				}
			}
			Self::emit_list(f, &[I::End, I::End, I::Return, I::End]);
			Self::emit_list(f, &[I::LocalGet(kind), I::I64Const(Kind::List as i64), I::I64Ne, I::If(BlockType::Empty)]);
			s.call(f, "not_an_object");
			Self::emit_list(f, &[I::End, I::LocalGet(0), I::LocalGet(1), I::LocalGet(entry), I::RefAsNonNull, I::Call(replace_entry)]);
		});
		assert_eq!(self.func_index("field_with"), field_with, "recursive call index");
	}
}
