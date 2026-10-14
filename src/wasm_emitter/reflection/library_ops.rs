//! Runtime functions of the library words (`reverse`, `sort`, `upper`, `lower`, `split`, `join`), see library_words.rs

use crate::node::{Bracket, Node, Separator};
use crate::operators::{op_to_code, Op};
use crate::type_kinds::{Kind, KIND_MASK, SEPARATOR_MASK, SEPARATOR_SHIFT};
use crate::type_kinds::{CURLY_LIST_KIND, KIND_BITS, SQUARE_LIST_KIND};
use crate::wasm_emitter::WasmGcEmitter;
use crate::words::ORD_WORD;
use wasm_encoder::*;
use Instruction as I;
use Instruction::I32Const;
use ValType::Ref;
mod object_copies;
mod ordering;
mod text_words;

/// The bracket info in a list's kind, above its KIND_BITS
const BRACKET_INFO_MASK: i64 = 0xff;
/// The operator code in a key's kind, above its KIND_BITS; a type instance `point{x:1}` has none
const OP_INFO_MASK: i64 = 0xff;
/// An operator's text in list_text's table: its pointer and length, two i32 words
const OPERATOR_ENTRY_BYTES: i32 = 2;
use crate::wasm_emitter::layout::BYTE;
use crate::wasm_emitter::text::text_unicode::CaseMapping;

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
pub const LIBRARY_FUNCTIONS: [(&str, &str); 20] = [
	(super::super::host::wasi_emitter::WASI_ENVIRONMENT, super::super::host::wasi_emitter::WASI_ENVIRONMENT),
	(crate::words::MAP_KEYS, crate::words::MAP_KEYS),
	(crate::library_words::MAP_VALUES, crate::library_words::MAP_VALUES),
	(crate::library_words::MAP_ENTRIES, crate::library_words::MAP_ENTRIES),
	(crate::library_words::COLLECTION_CONTAINS, crate::library_words::COLLECTION_CONTAINS),
	(crate::library_words::COLLECTION_POSITION, crate::library_words::COLLECTION_POSITION),
	(crate::library_words::MAP_GET_OR, crate::library_words::MAP_GET_OR),
	(crate::library_words::MAP_WITHOUT, crate::library_words::MAP_WITHOUT),
	(crate::one_entry_maps::MAP_OF_ENTRY, crate::one_entry_maps::MAP_OF_ENTRY),
	(ORD_WORD, CODEPOINT_OF),
	("chars", "text_chars"),
	("field_with", "field_with"),
	(crate::library_words::INSTANCE_COPY, crate::library_words::INSTANCE_COPY),
	("reverse", "list_reverse"),
	("sort", "list_sort"),
	("upper", "text_upper"),
	("lower", "text_lower"),
	("split", "text_split"),
	("join", "list_join"),
	(crate::words::SLICE, NODE_SLICE),
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
/// list_sorted_by(list, keys): the list ordered by the keys, a list of the same length (sort_by of a key function)
pub const LIST_SORTED_BY: &str = "list_sorted_by";

impl WasmGcEmitter {
	pub(crate) fn emit_library_ops(&mut self) {
		self.emit_text_reverse(); // list_reverse hands a text to it
		self.emit_list_reverse();
		self.emit_int_lists();
		self.emit_text_chars(); // and it ends with list_reverse
		if self.should_emit_function(NODE_ORDER) {
			self.emit_node_order();
		}
		self.emit_uncertain_order(); // after node_order, which it calls
		self.emit_text_case("text_upper", CaseMapping::Upper);
		self.emit_text_case("text_lower", CaseMapping::Lower);
		self.emit_text_split();
		self.emit_radix_runtimes(); // list_join calls hex_text, binary_text
		self.emit_list_join();
		self.emit_list_sort(); // after text_chars and list_join: a text sorts its characters
		self.emit_print_value(); // after list_join, which gives the text
		self.emit_wasi_environment();
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
			Self::emit_kind_test(f, Kind::Text as i64, I::I64Eq);
			Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(0)]);
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
			Self::emit_list(
				func,
				&[
					I::RefCastNonNull(HeapType::Concrete(self.type_manager.i64_box_type)),
					I::StructGet { struct_type_index: self.type_manager.i64_box_type, field_index: 0 },
				],
			);
		}
	}

	/// Trap unless the node in local `local` has kind `expected`
	pub(in crate::wasm_emitter) fn emit_require_kind(
		&self,
		func: &mut Function,
		local: u32,
		expected: Kind,
		error: &'static str,
	) {
		self.emit_field(func, local, 0);
		Self::emit_list(func, &[I::I64Const(expected as i64), I::I64Ne]);
		self.emit_fail_if(func, error);
	}

	/// A one-character string is a Codepoint node: local `text` becomes the equivalent Text node of its UTF-8 bytes
	pub(in crate::wasm_emitter) fn emit_codepoint_as_text(&self, func: &mut Function, text: u32) {
		Self::emit_list(func, &[I::LocalGet(text), I::RefAsNonNull]);
		self.call(func, crate::wasm_emitter::text::text_builtins::TEXT_OF);
		func.instruction(&I::LocalSet(text));
	}

	/// Trap unless local 0 is a list (or ø, which `emit_empty_as_null` turned into null)
	fn emit_require_list(&self, func: &mut Function) {
		Self::emit_list(func, &[I::LocalGet(0), I::RefIsNull, I::If(BlockType::Empty), I::Else]);
		self.emit_field(func, 0, 0);
		Self::emit_kind_test(func, Kind::List as i64, I::I64Ne);
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
				Self::emit_kind_test(f, kind as i64, I::I64Eq);
				Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(0), I::RefAsNonNull]);
				s.call(f, "text_reverse");
				Self::emit_list(f, &[I::Return, I::End]);
			}
			f.instruction(&I::End);
			s.emit_empty_as_null(f, 0);
			s.emit_require_list(f);
			s.emit_cell_walk(f, 0, |f| {
				s.emit_field(f, 0, 0);
				s.emit_field(f, 0, 1);
				Self::emit_list(f, &[I::LocalGet(reversed), I::StructNew(node_type), I::LocalSet(reversed)]);
			});
			s.emit_list_result(f, reversed);
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
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(list), I::LocalSet(cell),
					I::I64Const(0), I::LocalSet(index),
					I::Block(BlockType::Empty),
						I::Loop(BlockType::Empty),
							I::LocalGet(cell), I::RefIsNull, I::BrIf(1),
				]);
			};
			let next_cell = |f: &mut Function| {
				s.emit_field(f, cell, 2);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalSet(cell),
					I::LocalGet(index), I::I64Const(1), I::I64Add, I::LocalSet(index),
					I::Br(0),
					I::End,
					I::End,
				]);
			};
			// bound = the int in `bound`, or `default` for ø; never negative (no wrap around, Footguns.md); at most length
			let clamped_bound = |f: &mut Function, bound: u32, default: u32, target: u32| {
				s.emit_empty_as_null(f, bound);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(bound), I::RefIsNull, I::If(BlockType::Result(ValType::I64)),
						I::LocalGet(default),
					I::Else,
				]);
				s.emit_int_value_of(f, bound);
				Self::emit_list(f, &[I::End, I::LocalSet(target)]);
				s.emit_require_integral(f, target);
				Self::emit_list(f, &[I::LocalGet(target), I::I64Const(0), I::I64LtS]);
				s.emit_fail_if(f, "index_out_of_range");
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(target), I::LocalGet(length), I::LocalGet(target), I::LocalGet(length), I::I64LtS,
					I::Select, I::LocalSet(target),
				]);
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
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(index), I::LocalGet(to), I::I64GeS, I::BrIf(1),
				I::LocalGet(index), I::LocalGet(from), I::I64GeS, I::If(BlockType::Empty),
			]);
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
			Self::emit_kind_test(f, kind as i64, I::I64Eq);
			Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(local), I::RefAsNonNull]);
			self.call(f, "text_chars");
			Self::emit_list(f, &[I::LocalSet(local), I::I32Const(1), I::LocalSet(is_text), I::End]);
		}
		f.instruction(&I::End);
	}

	/// The list in `local` (not null), joined back into a text when `is_text` (emit_text_as_characters)
	fn emit_characters_as_text(&self, f: &mut Function, local: u32, is_text: u32) {
		let node_ref = Ref(self.node_ref(false));
		#[rustfmt::skip]
		Self::emit_list(f, &[
			I::LocalGet(is_text), I::If(BlockType::Result(node_ref)),
				I::LocalGet(local), I::I32Const(0), I::I32Const(0),
		]);
		self.call(f, "new_text");
		self.call(f, "list_join");
		Self::emit_list(f, &[I::Else, I::LocalGet(local), I::RefAsNonNull, I::End]);
	}

	/// Push the node in `local` (not null), a `key:value` entry as the one-entry map `{key:value}` it stands for, so its
	/// text keeps the braces: `{a:{b:1}}` is the entry a:(b:1) at run time and would write a:b:1. A tag, an entry whose
	/// value is a map (`data point{x:1}`), writes as an instance does, its name before its braces (card data-tag)
	pub(in crate::wasm_emitter) fn emit_entry_in_braces(&self, f: &mut Function, local: u32) {
		let node_type = self.type_manager.node_type;
		let entry_kind_mask = (OP_INFO_MASK << KIND_BITS) | KIND_MASK;
		let colon_entry_kind = (crate::operators::op_to_code(&crate::operators::Op::Colon) << KIND_BITS) | KEY_KIND;
		self.emit_field(f, local, 0);
		#[rustfmt::skip]
		Self::emit_list(f, &[
			I::I64Const(entry_kind_mask), I::I64And, I::I64Const(colon_entry_kind), I::I64Eq,
			I::If(BlockType::Result(Ref(self.node_ref(false)))),
		]);
		if !self.writes_tags {
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::I64Const(CURLY_LIST_KIND), I::LocalGet(local), I::RefNull(HeapType::Concrete(node_type)),
				I::StructNew(node_type),
			]);
			return Self::emit_list(f, &[I::Else, I::LocalGet(local), I::RefAsNonNull, I::End]);
		}
		self.emit_field(f, local, 2);
		Self::emit_list(f, &[I::RefIsNull, I::If(BlockType::Result(ValType::I64)), I::I64Const(0), I::Else]);
		self.emit_field(f, local, 2);
		Self::emit_list(f, &[I::StructGet { struct_type_index: node_type, field_index: 0 }, I::End]);
		let list_kind_mask = (BRACKET_INFO_MASK << KIND_BITS) | KIND_MASK;
		#[rustfmt::skip]
		Self::emit_list(f, &[
			I::I64Const(list_kind_mask), I::I64And, I::I64Const(CURLY_LIST_KIND), I::I64Eq,
			I::If(BlockType::Result(Ref(self.node_ref(false)))),
				I::I64Const(KEY_KIND),
		]);
		self.emit_field(f, local, 1);
		self.emit_field(f, local, 2);
		#[rustfmt::skip]
		Self::emit_list(f, &[
			I::StructNew(node_type),
			I::Else,
				I::I64Const(CURLY_LIST_KIND), I::LocalGet(local), I::RefNull(HeapType::Concrete(node_type)),
				I::StructNew(node_type),
			I::End,
			I::Else,
				I::LocalGet(local), I::RefAsNonNull,
			I::End,
		]);
	}
}

/// A tag quoted as data anywhere in the program (`p = data point{x:1}`): an entry whose value is a map. Markup
/// (`p{ "hello" }`) has the same shape but is no data, so a page's list_text needs no tag check
pub(in crate::wasm_emitter) fn mentions_quoted_tag(node: &Node) -> bool {
	match node.drop_meta() {
		Node::List(items, _, _) if crate::lowering::run_time_blocks::is_data(node) => {
			items.iter().skip(1).any(mentions_tag)
		}
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
