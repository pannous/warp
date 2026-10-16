//! Text as UTF-8 bytes: `a + b` concatenates texts and characters, `read(path)` loads a file,
//! `byte_at(text, offset)` and `byte_slice(text, start, end)` address bytes by 0-based offset, end exclusive.
//! `error message` makes an Error value of a text, the way a failed `read` reports.
//! Offsets are bytes, not characters: code that scans UTF-8 itself (a parser, a binary index) needs them.

use crate::node::{Bracket, Node, Separator};
use crate::type_kinds::Kind;
use crate::wasm_emitter::layout::BYTE;
use crate::wasm_emitter::runtime_dependencies::{all, any, Dependency};
use crate::wasm_emitter::{WasmGcEmitter, RAN_WITHOUT_ERROR};
use crate::words::{CHR_WORD, ERROR_WORD, IS_ERROR_WORD, RAISE_WORD};
use std::collections::HashSet;
use wasm_encoder::*;
use Instruction as I;
use ValType::Ref;

pub const TEXT_CONCAT: &str = "text_concat";
/// text_concat for a text held in a variable, which may be built up (`t += x`): it grows into rooms (text_rooms.rs)
pub const TEXT_APPEND: &str = "text_append";
pub const TEXT_OF: &str = "text_of";
/// What a text joined with `+` takes as its literal when made at run time: `"x " + parse("[1 2]")`
pub(in crate::wasm_emitter) const STRUCTURE_KINDS: [Kind; 2] = [Kind::Key, Kind::List];
/// text_argument(node): the text a text builtin reads (text_of), but an Error fails with its own message (card byte-slice)
const TEXT_ARGUMENT: &str = "text_argument";
/// float_text when the analyzer saw floats (makes_floats), and the run-time functions that make floats
const FLOAT_MAKERS: [&str; 4] = [
	super::super::numbers::float_text::FLOAT_TEXT,
	super::super::lists::list_ops::TEXT_AS_FLOAT,
	super::super::numbers::casts::TEXT_AS_NUMBER,
	super::super::numbers::uncertain::UNCERTAIN_NEW,
];
/// The builtins that read their text through text_argument
const TEXT_ARGUMENT_USERS: [&str; 5] = [BYTE_AT, BYTE_SLICE, TEXT_TRIM, TEXT_STARTS_WITH, TEXT_ENDS_WITH];
/// c_string(text) -> i32: a zero-terminated copy of a text known only at run time, for a C function's `char*`
pub const C_STRING: &str = "c_string";
const BYTE_AT: &str = "byte_at";
const BYTE_SLICE: &str = "byte_slice";
/// `memory_byte(address)`, `memory_set_byte(address, value)`: one byte of linear memory, what lib/memory.warp's
/// `memory.slice` and `memory.copy` are made of (samples/wasm/wasm_interop.warp)
const MEMORY_BYTE: &str = "memory_byte";
const MEMORY_SET_BYTE: &str = "memory_set_byte";
/// `trim(text)`: the text without the whitespace at either end, sharing its memory
const TRIM: &str = "trim";
const TEXT_TRIM: &str = "text_trim";
pub const TEXT_ROUGH_TRIM: &str = "text_rough_trim";
/// `float_bits(x)`: the IEEE 754 bits of a float as an Int, `bits_float(n)` the float of them (an `f64.const` to emit)
pub const FLOAT_BITS: &str = "float_bits";
pub const BITS_FLOAT: &str = "bits_float";
/// `starts_with(text, prefix)`, `ends_with(text, suffix)`: 1 or 0
const STARTS_WITH: &str = "starts_with";
const ENDS_WITH: &str = "ends_with";
const TEXT_STARTS_WITH: &str = "text_starts_with";
const TEXT_ENDS_WITH: &str = "text_ends_with";
/// text_matches_at(text, part, offset) -> i32: 1 when the bytes of part are those of text at the byte offset
const TEXT_MATCHES_AT: &str = "text_matches_at";
/// text_find(text, part) -> i64: the 1-based byte position of part in text, 0 when absent (`"b" in "abc"`, contains)
pub const TEXT_FIND: &str = "text_find";
/// The bytes trim drops: space, tab, line feed, carriage return
const WHITESPACE_BYTES: [i32; 4] = [b' ' as i32, b'\t' as i32, b'\n' as i32, b'\r' as i32];
/// The bytes below are ASCII; text_rough_trim keeps any byte of a longer UTF-8 character
const ASCII_END: i32 = 0x80;
const ASCII_ALPHANUMERIC_RANGES: [(u8, u8); 3] = [(b'0', b'9'), (b'A', b'Z'), (b'a', b'z')];
const READ: &str = "read";
const READ_TEXT: &str = "read_text";
const HOST_READ: &str = "host_read";
/// error_of(text) → Error: exported, the host builds the Error of a caught stack overflow with it (guarded_call)
pub const ERROR_OF: &str = "error_of";
pub const WARNING: &str = "warning";
const WARN_TEXT: &str = "warn_text";
pub(in crate::wasm_emitter) const HOST_WARN: &str = "host_warn";
/// text_with_char_at encodes a code point as UTF-8; it is emitted with node_with_at
const CHARACTER_ENCODER: &str = "node_with_at";
/// `text_form(x)`: the text of a value, what an interpolation hole `"\(x)"` becomes (interpolation.rs)
pub const TEXT_FORM: &str = "text_form";

/// name, number of arguments, result kind
const TEXT_BUILTINS: [(&str, usize, Kind); 23] = [
	(MEMORY_BYTE, 1, Kind::Int),
	(MEMORY_SET_BYTE, 2, Kind::Int),
	(crate::memoization::MEMO_KNOWN, 2, Kind::Int),
	(crate::memoization::MEMO_VALUE, 2, Kind::Int),
	(crate::memoization::MEMO_STORE, 3, Kind::Int),
	(READ, 1, Kind::Text),
	(BYTE_AT, 2, Kind::Int),
	(BYTE_SLICE, 3, Kind::Text),
	(ERROR_WORD, 1, Kind::Empty),
	(RAISE_WORD, 1, Kind::Text),
	(IS_ERROR_WORD, 1, Kind::Int),
	(WARNING, 1, Kind::Text),
	(TEXT_FORM, 1, Kind::Text),
	(RAN_WITHOUT_ERROR, 1, Kind::Int),
	(TRIM, 1, Kind::Text),
	(STARTS_WITH, 2, Kind::Int),
	(ENDS_WITH, 2, Kind::Int),
	(CHR_WORD, 1, Kind::Codepoint),
	(FLOAT_BITS, 1, Kind::Int),
	(BITS_FLOAT, 1, Kind::Float),
	(crate::wasm_emitter::CAUGHT_ERROR, 2, Kind::Error),
	(crate::numbers::uncertain::CERTAINLY, 1, Kind::Int),
	(crate::numbers::uncertain::POSSIBLY, 1, Kind::Int),
];

/// The text builtins and the node words (node_api.rs), which are called the same way
fn builtins() -> impl Iterator<Item = &'static (&'static str, usize, Kind)> {
	TEXT_BUILTINS.iter().chain(super::super::reflection::node_api::NODE_BUILTINS.iter())
}

pub fn text_builtin_kind(name: &str, arguments: usize) -> Option<Kind> {
	builtins().find(|(builtin, arity, _)| *builtin == name && *arity == arguments).map(|(_, _, kind)| *kind)
}

/// The numbers of arguments a text builtin takes, empty for any other name
pub fn text_builtin_arities(name: &str) -> Vec<usize> {
	builtins().filter(|(builtin, _, _)| *builtin == name).map(|(_, arity, _)| *arity).collect()
}

pub fn is_text_builtin(name: &str) -> bool {
	builtins().any(|(builtin, _, _)| *builtin == name)
}

/// `+` of two texts or characters is a text; anything else joins a text in its text form, as str() writes it
/// (`"F:" + 13` → `"F:13"`, as JS/Kotlin; `"oldest: " + [1 2]` → `"oldest: [1 2]"`, card print-oldest), but a function
pub fn concatenates(left: Kind, right: Kind) -> bool {
	let is_text = |kind: &Kind| matches!(kind, Kind::Text | Kind::Codepoint);
	// an Error joins as its message (card try-raise: `"caught: " + e` of `catch e`), a symbol value as its name (P230:
	// `"square is " + effects of square`)
	[left, right].iter().any(is_text) && !matches!(left, Kind::Function) && !matches!(right, Kind::Function)
}

/// Runtime functions the text builtins call, and the list, map and number functions emitted with them
pub(in crate::wasm_emitter) fn dependencies() -> Vec<Dependency> {
	use super::super::host::wasi_emitter::{PRINT_VALUE, PUT_VALUE};
	use super::super::lists::list_ops::*;
	use super::super::maps::map_backend::{NODE_MAP_FUNCTIONS, NODE_MAP_NEW};
	use super::super::numbers::uncertain::{UNCERTAIN_NEW, UNCERTAIN_TEXT};
	use super::super::numbers::{casts::TEXT_AS_NUMBER, exact::EXACT_TEXT, float_text::FLOAT_AS_EXACT};
	use super::super::reflection::equality::IS_TRUTHY;
	use super::super::reflection::library_ops::{
		CODEPOINT_OF, LIST_JOIN, LIST_SORTED_BY, LIST_TEXT, NODE_ORDER, NODE_SLICE, TEXT_QUOTED,
	};
	use super::super::time::times::{TIME_FIELD, TIME_TEXT};
	use super::super::{CAUGHT_ERROR, INT_RUNTIME, VALUES_EQUAL};
	use crate::library_words::{FIELD_WITH, INSTANCE_COPY, MAP_WORD_FUNCTIONS};
	let calls_text_of = [
		VALUES_EQUAL,
		TEXT_CONCAT,
		TEXT_APPEND,
		BYTE_AT,
		BYTE_SLICE,
		TEXT_TRIM,
		TEXT_ROUGH_TRIM,
		C_STRING,
		ERROR_OF,
		WARN_TEXT,
		"list_join",
		"text_upper",
		"text_lower",
		"text_split",
		"list_reverse",
		"text_chars",
		"list_sort",
		NODE_ORDER,
	];
	vec![
		// `catch e`: the caught error becomes an Error through error_of
		any(&[CAUGHT_ERROR], &[ERROR_OF]),
		any(&[LIST_TEXT], &["list_join"]), // emitted together, with the same helpers
		any(&[NODE_SLICE], &["text_chars", "list_join", "list_reverse"]),
		// a text searched for a text (`s.contains("b")`, `"b" in s`); the map words are emitted together
		any(&MAP_WORD_FUNCTIONS, &["map_find", TEXT_FIND, TEXT_MATCHES_AT, TEXT_OF]),
		any(&[ELEMENT_BODY], &[ELEMENT_CHILDREN]),
		any(&[ELEMENTS_FIELD], &["map_find", TAGGED_FIELD, STRUCT_BODY]),
		any(&[NODE_AT_KEY], &["node_index_at", "map_get"]),
		// instance_copy is emitted with field_with, both looking into an instance's fields; `shallow` is tested for truth
		any(&[INSTANCE_COPY], &[FIELD_WITH, IS_TRUTHY]),
		any(&[FIELD_WITH], &[STRUCT_BODY]), // an instance keeps its type
		any(&[NODE_WITH_KEY], &[NODE_WITH_NODE_AT, FIELD_WITH]),
		any(&[NODE_WITH_NODE_AT], &["node_with_at", "get_int_value"]),
		// a map variable held as a hash table names its keys by map_key_name, emitted with map_get; a map node found or
		// set by name gets an index of the same tables (map_index.rs); maps compare keys by name: map_key_name reads a
		// character key as its text
		any(&["map_get", "map_find", FIELD_WITH], &[NODE_MAP_NEW, TEXT_OF, VALUES_EQUAL]),
		any(&NODE_MAP_FUNCTIONS, &NODE_MAP_FUNCTIONS),
		any(&NODE_MAP_FUNCTIONS, &["map_get"]),
		any(&["node_list_of"], &["node_list_push"]),
		any(&["exact_euclid_div"], &[INT_RUNTIME]),
		any(&[FLOAT_AS_EXACT], &[INT_RUNTIME, "new_int"]),
		any(&[LIST_EXTEND], &["list_concat"]), // of a non-list
		// two lists added are concatenated, two texts too, a text and a number joined
		any(&[NODE_ADD], &["list_concat", TEXT_CONCAT, TEXT_OF, LIST_JOIN]),
		any(&[TEXT_AS_NUMBER], &[TEXT_AS_FLOAT, INT_RUNTIME, "exact_div", "new_text", "new_int", "new_float"]),
		any(&[TEXT_AS_FLOAT], &[TEXT_AS_INT]),
		any(&[TEXT_AS_INT], &["get_int_value"]),
		all(&[TEXT_AS_FLOAT, INT_RUNTIME], &["exact_to_f64"]),
		all(&[TEXT_AS_INT, INT_RUNTIME], &["exact_mul", "exact_add", "exact_sub"]),
		any(&["list_sort"], &[LIST_SORTED_BY]),
		any(&[LIST_SORTED_BY], &[NODE_ORDER, "text_chars", "list_join"]),
		any(&[TEXT_FIND, TEXT_STARTS_WITH, TEXT_ENDS_WITH], &[TEXT_MATCHES_AT]),
		any(&[PRINT_VALUE, PUT_VALUE], &["list_join"]),
		// the text of a list (list_text) is emitted with join, as the analyzer requires list_join for both; a nested
		// list joins as "[…]", a text quoted
		any(&["list_join"], &[TEXT_CONCAT, LIST_TEXT, TEXT_QUOTED]),
		all(&["list_join", UNCERTAIN_NEW], &[UNCERTAIN_TEXT]), // `7.0 ± 2.0`
		all(&["list_join", TIME_FIELD], &[TIME_TEXT]),         // `2026-10-10T14:43:33.682Z`
		// numbers that are no fixnum (big integers, ratios) join as their exact text, built by text_concat
		all(&["list_join", INT_RUNTIME], &[EXACT_TEXT, TEXT_CONCAT]),
		// codepoint_of reads a one-character text as its character
		any(&[CODEPOINT_OF], &["string_char_at", "text_grapheme_count"]),
		any(&TEXT_ARGUMENT_USERS, &[TEXT_ARGUMENT]),
		any(&[TEXT_ARGUMENT], &[TEXT_OF]),
		any(&calls_text_of, &[TEXT_OF]),
		// the characters are collected backwards, and list_reverse hands texts on
		any(&["text_chars"], &["list_reverse", "text_reverse"]),
		any(&["list_reverse"], &["text_reverse"]),
		any(&[TEXT_OF], &[CHARACTER_ENCODER]),
	]
}

/// A float joins as its text, made only by a program with floats (analyzer::makes_floats): float_text when a list is
/// joined and a float may be made, none otherwise
pub(in crate::wasm_emitter) fn require_float_text(required: &mut HashSet<&'static str>) {
	use super::super::numbers::float_text::FLOAT_TEXT;
	if required.contains("list_join") && FLOAT_MAKERS.iter().any(|name| required.contains(name)) {
		required.insert(FLOAT_TEXT);
	} else {
		required.remove(FLOAT_TEXT);
	}
}

impl WasmGcEmitter {
	/// `read(path)`, `byte_at(text, offset)`, `byte_slice(text, start, end)` as a Node
	pub(in crate::wasm_emitter) fn emit_text_builtin(&mut self, func: &mut Function, name: &str, arguments: &[Node]) {
		match (name, arguments) {
			(READ, [path]) if self.ctx.func_registry.get(HOST_READ).is_some() => {
				self.emit_node_instructions(func, path);
				self.emit_call(func, READ_TEXT);
			}
			(READ, [path]) => {
				let reason = format!("read {} failed: host imports are not available", path.serialize());
				self.emit_runtime_error_value(func, &reason);
			}
			// warnings are errors: the run stops as at `raise`, also where the warning's value is dropped (card under-use)
			(WARNING | RAISE_WORD, [message])
				if name == RAISE_WORD
					|| crate::errors::diagnostic::warning_mode().fails(crate::errors::diagnostic::Level::Normal) =>
			{
				self.emit_trap_detail(func, message);
				self.emit_runtime_error(func, super::super::lists::list_ops::RETURNED_ERROR);
			}
			(WARNING, [message]) if self.ctx.func_registry.get(HOST_WARN).is_some() => {
				self.emit_node_instructions(func, message);
				self.emit_call(func, WARN_TEXT);
			}
			(WARNING, [message]) => {
				let reason = format!("warning {} not reported: host imports are not available", message.serialize());
				self.emit_runtime_error_value(func, &reason);
			}
			(ERROR_WORD, [message]) => {
				self.emit_node_instructions(func, message);
				self.emit_call(func, ERROR_OF);
			}
			(crate::wasm_emitter::CAUGHT_ERROR, [finished, value]) => self.emit_caught_error(func, finished, value),
			// a hole names a variable: a word that names nothing is loud, never its own spelling
			(TEXT_FORM, [value]) => match value.drop_meta() {
				Node::Symbol(name) if self.is_unbound(name) => self.emit_undefined_variable(func, name),
				_ => self.emit_runtime_text_cast(func, value),
			},
			(
				BYTE_AT
				| MEMORY_BYTE
				| MEMORY_SET_BYTE
				| IS_ERROR_WORD
				| RAN_WITHOUT_ERROR
				| STARTS_WITH
				| ENDS_WITH
				| FLOAT_BITS
				| crate::memoization::MEMO_KNOWN
				| crate::memoization::MEMO_VALUE
				| crate::memoization::MEMO_STORE
				| crate::numbers::uncertain::CERTAINLY
				| crate::numbers::uncertain::POSSIBLY,
				_,
			) => {
				self.emit_integer_text_builtin(func, name, arguments);
				self.emit_call(func, "new_int");
			}
			(BYTE_SLICE, [text, start, end]) => {
				self.emit_text_argument(func, text);
				self.emit_numeric_value(func, start);
				self.emit_numeric_value(func, end);
				self.emit_call(func, BYTE_SLICE);
			}
			(TRIM, [text]) => {
				self.emit_text_argument(func, text);
				self.emit_call(func, TEXT_TRIM);
			}
			(CHR_WORD, [code]) => {
				self.emit_numeric_value(func, code);
				func.instruction(&I::I32WrapI64);
				self.emit_call(func, "new_codepoint");
			}
			(BITS_FLOAT, [_]) => {
				self.emit_float_builtin(func, name, arguments);
				self.emit_call(func, "new_float");
			}
			_ if super::super::reflection::node_api::NODE_BUILTINS
				.iter()
				.any(|(builtin, arity, _)| *builtin == name && *arity == arguments.len()) =>
			{
				self.emit_node_builtin(func, name, arguments);
			}
			_ => unreachable!("text_builtin_kind admits {name} with {} arguments", arguments.len()),
		}
	}

	/// A text argument as a Text node: a one-character text held in a variable is a character (`input = "a"`)
	fn emit_text_argument(&mut self, func: &mut Function, text: &Node) {
		self.emit_node_instructions(func, text);
		if !matches!(text.drop_meta(), Node::Text(_)) {
			self.emit_call(func, TEXT_ARGUMENT);
		}
	}

	/// `byte_at(text, offset)` and `is_error(x)` as raw i64
	/// `bits_float(n)` as raw f64
	pub(in crate::wasm_emitter) fn emit_float_builtin(&mut self, func: &mut Function, name: &str, arguments: &[Node]) {
		match (name, arguments) {
			(BITS_FLOAT, [bits]) => {
				self.emit_numeric_value(func, bits);
				self.emit_int_to_machine(func);
				func.instruction(&I::F64ReinterpretI64);
			}
			_ => unreachable!("{name} with {} arguments is no float builtin", arguments.len()),
		}
	}

	pub(in crate::wasm_emitter) fn emit_integer_text_builtin(
		&mut self,
		func: &mut Function,
		name: &str,
		arguments: &[Node],
	) {
		match (name, arguments) {
			(FLOAT_BITS, [value]) => {
				self.emit_float_value(func, value);
				func.instruction(&I::I64ReinterpretF64);
				self.emit_int_from_machine(func);
			}
			(BYTE_AT, [text, offset]) => {
				self.emit_text_argument(func, text);
				self.emit_numeric_value(func, offset);
				self.emit_call(func, BYTE_AT);
			}
			(MEMORY_BYTE, [address]) => {
				self.emit_numeric_value(func, address);
				Self::emit_list(func, &[I::I32WrapI64, I::I32Load8U(BYTE), I::I64ExtendI32U]);
			}
			// the value written, as `x = v` is v
			(MEMORY_SET_BYTE, [address, value]) => {
				let written = self.scratch(0);
				self.emit_numeric_value(func, address);
				func.instruction(&I::I32WrapI64);
				self.emit_numeric_value(func, value);
				Self::emit_list(func, &[I::LocalTee(written), I::I32WrapI64, I::I32Store8(BYTE), I::LocalGet(written)]);
			}
			(IS_ERROR_WORD, [value]) => {
				self.emit_node_instructions(func, value);
				func.instruction(&I::StructGet { struct_type_index: self.type_manager.node_type, field_index: 0 });
				Self::emit_kind_test(func, Kind::Error as i64, I::I64Eq);
				Self::emit_list(func, &[I::I64ExtendI32U]);
			}
			(RAN_WITHOUT_ERROR, [statement]) => self.emit_ran_without_error(func, statement),
			(crate::numbers::uncertain::CERTAINLY | crate::numbers::uncertain::POSSIBLY, [comparison]) => {
				self.emit_certainty(func, name, comparison)
			}
			(STARTS_WITH | ENDS_WITH, [text, part]) => {
				self.emit_text_argument(func, text);
				self.emit_text_argument(func, part);
				self.emit_call(func, if name == STARTS_WITH { TEXT_STARTS_WITH } else { TEXT_ENDS_WITH });
			}
			(
				crate::memoization::MEMO_KNOWN | crate::memoization::MEMO_VALUE | crate::memoization::MEMO_STORE,
				[id, argument, rest @ ..],
			) => {
				self.emit_memo_word(func, name, id, argument, rest.first());
			}
			_ => unreachable!("{name} is no integer text builtin with {} arguments", arguments.len()),
		}
	}

	/// memo_known(id, n) (1 when the cache of function `id` holds n), memo_value(id, n), memo_store(id, n, v) (keeps v
	/// for n when n is 0 … MEMO_SIZE-1, and is v): the caches of memoization.rs, two i64 arrays per function, made at
	/// first use
	fn emit_memo_word(&mut self, func: &mut Function, name: &str, id: &Node, argument: &Node, value: Option<&Node>) {
		use crate::memoization::{MEMO_KNOWN, MEMO_SIZE, MEMO_STORE};
		let Node::Number(crate::extensions::numbers::Number::Int(id)) = id.drop_meta() else {
			unreachable!("memoization passes a constant id")
		};
		let array = self.type_manager.int_array_type;
		let array_ref = ValType::Ref(RefType { nullable: true, heap_type: HeapType::Concrete(array) });
		let new_global =
			|emitter: &mut Self| emitter.declare_global_of(array_ref, ConstExpr::ref_null(HeapType::Concrete(array)));
		let (values, known) = match self.memo_caches.get(id) {
			Some(globals) => *globals,
			None => {
				let made = (new_global(self), new_global(self));
				self.memo_caches.insert(*id, made);
				made
			}
		};
		let (index, stored) = (self.scratch(0), self.scratch(1));
		if let Some(value) = value {
			self.emit_numeric_value(func, value);
			func.instruction(&I::LocalSet(stored));
		}
		self.emit_numeric_value(func, argument);
		func.instruction(&I::LocalSet(index));
		// the arrays exist from the first word on
		for global in [values, known] {
			#[rustfmt::skip]
			Self::emit_list(func, &[
				I::GlobalGet(global), I::RefIsNull, I::If(BlockType::Empty),
					I::I32Const(MEMO_SIZE), I::ArrayNewDefault(array), I::GlobalSet(global),
				I::End,
			]);
		}
		#[rustfmt::skip]
		let in_range = [
			I::LocalGet(index), I::I64Const(0), I::I64GeS, I::LocalGet(index), I::I64Const(MEMO_SIZE as i64), I::I64LtS,
			I::I32And,
		];
		let element = |global: u32| [I::GlobalGet(global), I::RefAsNonNull, I::LocalGet(index), I::I32WrapI64];
		if name == MEMO_STORE {
			Self::emit_list(func, &in_range);
			func.instruction(&I::If(BlockType::Empty));
			Self::emit_list(func, &element(values));
			Self::emit_list(func, &[I::LocalGet(stored), I::ArraySet(array)]);
			Self::emit_list(func, &element(known));
			Self::emit_list(func, &[I::I64Const(1), I::ArraySet(array), I::End, I::LocalGet(stored)]);
		} else if name == MEMO_KNOWN {
			Self::emit_list(func, &in_range);
			func.instruction(&I::If(BlockType::Result(ValType::I64)));
			Self::emit_list(func, &element(known));
			Self::emit_list(func, &[I::ArrayGet(array), I::Else, I::I64Const(0), I::End]);
		} else {
			Self::emit_list(func, &element(values));
			func.instruction(&I::ArrayGet(array));
		}
	}

	/// `left + right` of texts or characters: a fresh text holding both
	pub(in crate::wasm_emitter) fn emit_text_concat(&mut self, func: &mut Function, left: &Node, right: &Node) {
		self.emit_concatenated(func, left);
		self.emit_concatenated(func, right);
		let builds_a_variable = matches!(left.drop_meta(), Node::Symbol(_));
		self.emit_call(func, if builds_a_variable { TEXT_APPEND } else { TEXT_CONCAT });
	}

	/// A value whose kind is known only at run time (a map value, arithmetic of an any value, a stored error), not ø itself
	pub(crate) fn holds_run_time_kind(&self, operand: &Node) -> bool {
		matches!(self.get_type(operand), Kind::Empty | Kind::Data) && !operand.is_empty_node()
	}

	/// A text operand as is, anything else in its text form; the implicit conversion is hinted
	fn emit_concatenated(&mut self, func: &mut Function, operand: &Node) {
		if self.holds_run_time_kind(operand) {
			self.emit_run_time_text_form(func, operand);
			return;
		}
		if self.get_type(operand) == Kind::Error {
			self.emit_runtime_text_cast(func, operand);
			return;
		}
		if matches!(self.get_type(operand), Kind::Text | Kind::Codepoint | Kind::Symbol) {
			self.emit_node_instructions(func, operand);
			return;
		}
		self.emit_cast(func, operand, &Node::Symbol("str".to_string()));
	}

	/// A value held as a Node (a map value, arithmetic of an any value) may be a number at run time: joined, a number
	/// takes its text form; a stored error stays itself, for text_concat to give it on, never joined as its message
	/// (`r = error "b"; "a" + r`, card error-value-kind)
	fn emit_run_time_text_form(&mut self, func: &mut Function, operand: &Node) {
		let (held, node_ref) = (self.node_scratch(), Ref(self.node_ref(false)));
		let node_type = self.type_manager.node_type;
		let (pointer, length) = self.allocate_string("");
		self.emit_node_instructions(func, operand);
		func.instruction(&I::LocalSet(held));
		self.emit_structure_as_literal(func, held, &STRUCTURE_KINDS, (pointer, length));
		self.emit_field(func, held, 0);
		Self::emit_kind_test(func, Kind::Error as i64, I::I64Eq);
		Self::emit_list(func, &[I::If(BlockType::Result(node_ref))]);
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::LocalGet(held), I::RefAsNonNull,
			I::Else,
				I::I64Const(crate::type_kinds::SQUARE_LIST_KIND), I::LocalGet(held),
		]);
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::RefNull(HeapType::Concrete(node_type)), I::StructNew(node_type), I::I32Const(pointer as i32),
			I::I32Const(length as i32),
		]);
		self.emit_call(func, "new_text");
		self.emit_call(func, super::super::reflection::library_ops::LIST_JOIN);
		func.instruction(&I::End);
	}

	/// A value of one of `kinds` made at run time (parsed code) in Node local `local` becomes its literal text, to join
	/// a text: `"x " + parse(t)` (card print-parse); only in a program that parses code. `empty`: the allocated empty text, the separator of that one-item list
	pub(in crate::wasm_emitter) fn emit_structure_as_literal(
		&self,
		func: &mut Function,
		local: u32,
		kinds: &[Kind],
		empty: (u32, u32),
	) {
		if !self.parses_code {
			return;
		}
		let node_type = self.type_manager.node_type;
		let (pointer, length) = empty;
		for (index, kind) in kinds.iter().enumerate() {
			self.emit_field(func, local, 0);
			Self::emit_kind_test(func, *kind as i64, I::I64Eq);
			if index > 0 {
				func.instruction(&I::I32Or);
			}
		}
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::If(BlockType::Empty),
				I::I64Const(crate::type_kinds::SQUARE_LIST_KIND), I::LocalGet(local),
				I::RefNull(HeapType::Concrete(node_type)), I::StructNew(node_type), I::I32Const(pointer as i32),
				I::I32Const(length as i32),
		]);
		self.call(func, "new_text");
		self.call(func, super::super::reflection::library_ops::LIST_TEXT);
		Self::emit_list(func, &[I::LocalSet(local), I::End]);
	}

	/// An Error node carrying `reason`, the way a failed fetch reports
	fn emit_runtime_error_value(&mut self, func: &mut Function, reason: &str) {
		let (ptr, len) = self.allocate_string(reason);
		Self::emit_list(func, &[I::I32Const(ptr as i32), I::I32Const(-(len as i32))]);
		let (length, pointer) = (self.scratch(0), self.scratch(1));
		Self::emit_list(func, &[I::I64ExtendI32S, I::LocalSet(length), I::I64ExtendI32U, I::LocalSet(pointer)]);
		self.emit_host_text_result(func, length, pointer);
	}

	/// i64 locals `length` and `pointer` from a host call → Node{kind: length < 0 ? Error : Text, $String(pointer, |length|)}
	pub(in crate::wasm_emitter) fn emit_host_text_result(&self, func: &mut Function, length: u32, pointer: u32) {
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::I64Const(Kind::Error as i64), I::I64Const(Kind::Text as i64), I::LocalGet(length), I::I64Const(0),
			I::I64LtS, I::Select, I::LocalGet(pointer), I::I32WrapI64, I::I64Const(0), I::LocalGet(length), I::I64Sub,
			I::LocalGet(length), I::LocalGet(length), I::I64Const(0), I::I64LtS, I::Select, I::I32WrapI64,
			I::StructNew(self.type_manager.string_type), I::RefNull(HeapType::Concrete(self.type_manager.node_type)),
			I::StructNew(self.type_manager.node_type),
		]);
	}

	/// text_of(node): a character as a text of its UTF-8 bytes, anything else unchanged; emitted before values_equal, which calls it
	pub(in crate::wasm_emitter) fn emit_text_of(&mut self) {
		if !self.should_emit_function(TEXT_OF) {
			return;
		}
		let node_ref = Ref(self.node_ref(false));
		let (placeholder, _) = self.allocate_string(" ");
		self.runtime_function(TEXT_OF, vec![node_ref], vec![node_ref], vec![], |s, f| {
			s.emit_field(f, 0, 0);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::I64Const(Kind::Codepoint as i64), I::I64Eq, I::If(BlockType::Result(node_ref)),
					I::I32Const(placeholder as i32), I::I32Const(1),
			]);
			s.call(f, "new_text");
			f.instruction(&I::I64Const(1));
			s.emit_field(f, 0, 1);
			Self::emit_list(f, &[I::RefCastNonNull(HeapType::I31), I::I31GetU, I::I64ExtendI32U]);
			s.call(f, "text_with_char_at");
			Self::emit_list(f, &[I::Else, I::LocalGet(0), I::End]);
		});
		if self.should_emit_function(TEXT_ARGUMENT) {
			self.runtime_function(TEXT_ARGUMENT, vec![node_ref], vec![node_ref], vec![], |s, f| {
				s.emit_fail_if_error(f, 0);
				f.instruction(&I::LocalGet(0));
				s.call(f, TEXT_OF);
			});
		}
	}

	pub(in crate::wasm_emitter) fn emit_text_builtins(&mut self) {
		self.emit_text_bytes_runtime();
		self.emit_text_search_runtime();
		self.emit_text_values_runtime();
	}

	/// byte_at and byte_slice: a text read by its bytes
	fn emit_text_bytes_runtime(&mut self) {
		let node_ref = Ref(self.node_ref(false));
		let long = ValType::I64;

		// byte_at(text, offset): the byte at the 0-based offset, 0…255
		if self.should_emit_function(BYTE_AT) {
			self.runtime_function(BYTE_AT, vec![node_ref, long], vec![long], vec![], |s, f| {
				f.instruction(&I::LocalGet(1));
				s.emit_text_field(f, 0, 1);
				Self::emit_list(f, &[I::I64ExtendI32U, I::I64GeU]);
				s.emit_fail_if(f, "index_out_of_range");
				s.emit_text_field(f, 0, 0);
				Self::emit_list(f, &[I::LocalGet(1), I::I32WrapI64, I::I32Add, I::I32Load8U(BYTE), I::I64ExtendI32U]);
			});
		}

		// byte_slice(text, start, end): the bytes start…end-1, sharing the memory of text
		if self.should_emit_function(BYTE_SLICE) {
			self.runtime_function(BYTE_SLICE, vec![node_ref, long, long], vec![node_ref], vec![], |s, f| {
				Self::emit_list(f, &[I::LocalGet(1), I::LocalGet(2), I::I64GtU, I::LocalGet(2)]);
				s.emit_text_field(f, 0, 1);
				Self::emit_list(f, &[I::I64ExtendI32U, I::I64GtU, I::I32Or]);
				s.emit_fail_if(f, "index_out_of_range");
				s.emit_text_field(f, 0, 0);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(1), I::I32WrapI64, I::I32Add, I::LocalGet(2), I::LocalGet(1), I::I64Sub, I::I32WrapI64,
				]);
				s.call(f, "new_text");
			});
		}
	}

	/// A text searched or trimmed: text_matches_at, text_find, starts_with, ends_with, text_trim
	fn emit_text_search_runtime(&mut self) {
		let node_ref = Ref(self.node_ref(false));
		let long = ValType::I64;

		// text_matches_at(text, part, offset): part's bytes at the offset of text, within its bounds
		if self.should_emit_function(TEXT_MATCHES_AT) {
			self.runtime_function(
				TEXT_MATCHES_AT,
				vec![node_ref, node_ref, ValType::I32],
				vec![ValType::I32],
				vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
				|s, f| {
					let (offset, text_address, part_address, part_length, index) = (2, 3, 4, 5, 6);
					s.emit_text_field(f, 0, 0);
					f.instruction(&I::LocalSet(text_address));
					s.emit_text_field(f, 1, 0);
					f.instruction(&I::LocalSet(part_address));
					s.emit_text_field(f, 1, 1);
					f.instruction(&I::LocalSet(part_length));
					// out of bounds: no match
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(offset), I::I32Const(0), I::I32LtS, I::LocalGet(offset), I::LocalGet(part_length),
						I::I32Add,
					]);
					s.emit_text_field(f, 0, 1);
					Self::emit_list(
						f,
						&[I::I32GtU, I::I32Or, I::If(BlockType::Empty), I::I32Const(0), I::Return, I::End],
					);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::Block(BlockType::Empty),
							I::Loop(BlockType::Empty),
								I::LocalGet(index), I::LocalGet(part_length), I::I32GeU, I::BrIf(1),
								I::LocalGet(text_address), I::LocalGet(offset), I::I32Add, I::LocalGet(index),
								I::I32Add, I::I32Load8U(BYTE), I::LocalGet(part_address), I::LocalGet(index), I::I32Add,
								I::I32Load8U(BYTE), I::I32Ne, I::If(BlockType::Empty),
									I::I32Const(0), I::Return,
								I::End,
								I::LocalGet(index), I::I32Const(1), I::I32Add, I::LocalSet(index),
								I::Br(0),
						I::End, I::End,
						I::I32Const(1),
					]);
				},
			);
		}
		// text_find(text, part): the first offset where part matches, 1-based; 0 when it never does
		if self.should_emit_function(TEXT_FIND) {
			self.runtime_function(TEXT_FIND, vec![node_ref, node_ref], vec![long], vec![ValType::I32], |s, f| {
				let offset = 2;
				Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(offset)]);
				s.emit_text_field(f, 0, 1);
				Self::emit_list(f, &[I::I32GtU, I::BrIf(1), I::LocalGet(0), I::LocalGet(1), I::LocalGet(offset)]);
				s.call(f, TEXT_MATCHES_AT);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::If(BlockType::Empty),
						I::LocalGet(offset), I::I32Const(1), I::I32Add, I::I64ExtendI32U, I::Return,
					I::End,
					I::LocalGet(offset), I::I32Const(1), I::I32Add, I::LocalSet(offset),
					I::Br(0),
					I::End,
					I::End,
					I::I64Const(0),
				]);
			});
		}
		// text_starts_with(text, part) / text_ends_with(text, part): 1 when part matches at the start / at the end
		for (name, at_end) in [(TEXT_STARTS_WITH, false), (TEXT_ENDS_WITH, true)] {
			if self.should_emit_function(name) {
				self.runtime_function(name, vec![node_ref, node_ref], vec![long], vec![], |s, f| {
					Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(1)]);
					if at_end {
						s.emit_text_field(f, 0, 1);
						s.emit_text_field(f, 1, 1);
						f.instruction(&I::I32Sub);
					} else {
						f.instruction(&I::I32Const(0));
					}
					s.call(f, TEXT_MATCHES_AT);
					f.instruction(&I::I64ExtendI32U);
				});
			}
		}

		// text_trim(text): the bytes between the whitespace at either end, sharing the memory of text
		if self.should_emit_function(TEXT_TRIM) {
			self.emit_trim(TEXT_TRIM, |f, byte| {
				for (index, whitespace) in WHITESPACE_BYTES.iter().enumerate() {
					Self::emit_list(f, &[I::LocalGet(byte), I::I32Const(*whitespace), I::I32Eq]);
					if index > 0 {
						f.instruction(&I::I32Or);
					}
				}
			});
		}
		// text_rough_trim(text): without the whitespace, control characters and ASCII punctuation at either end (`a ~ b`, P211)
		if self.should_emit_function(TEXT_ROUGH_TRIM) {
			self.emit_trim(TEXT_ROUGH_TRIM, |f, byte| {
				Self::emit_list(f, &[I::LocalGet(byte), I::I32Const(ASCII_END), I::I32LtU]);
				for (first, last) in ASCII_ALPHANUMERIC_RANGES {
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(byte), I::I32Const(first as i32), I::I32Sub, I::I32Const((last - first) as i32),
						I::I32LeU, I::I32Eqz, I::I32And,
					]);
				}
			});
		}
	}

	/// name(text): the bytes between the ones `is_trimmed` holds for (given the local of a byte) at either end, sharing
	/// the memory of text
	fn emit_trim(&mut self, name: &'static str, is_trimmed: impl Fn(&mut Function, u32)) {
		let node_ref = Ref(self.node_ref(false));
		self.runtime_function(name, vec![node_ref], vec![node_ref], vec![ValType::I32; 4], |s, f| {
			let (address, start, end, byte) = (1, 2, 3, 4);
			s.emit_text_field(f, 0, 0);
			f.instruction(&I::LocalSet(address));
			s.emit_text_field(f, 0, 1);
			f.instruction(&I::LocalSet(end));
			let load = |f: &mut Function, offset: &[I<'static>]| {
				f.instruction(&I::LocalGet(address));
				Self::emit_list(f, offset);
				Self::emit_list(f, &[I::I32Add, I::I32Load8U(BYTE), I::LocalSet(byte)]);
			};
			// leading: start moves right while it is before end and at a trimmed byte
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(start), I::LocalGet(end), I::I32GeU, I::BrIf(1),
			]);
			load(f, &[I::LocalGet(start)]);
			is_trimmed(f, byte);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::I32Eqz, I::BrIf(1),
				I::LocalGet(start), I::I32Const(1), I::I32Add, I::LocalSet(start),
				I::Br(0),
				I::End,
				I::End,
			]);
			// trailing: end moves left while the byte before it is trimmed
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(end), I::LocalGet(start), I::I32LeU, I::BrIf(1),
			]);
			load(f, &[I::LocalGet(end), I::I32Const(1), I::I32Sub]);
			is_trimmed(f, byte);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::I32Eqz, I::BrIf(1),
				I::LocalGet(end), I::I32Const(1), I::I32Sub, I::LocalSet(end),
				I::Br(0),
				I::End,
				I::End,
			]);
			Self::emit_list(
				f,
				&[I::LocalGet(address), I::LocalGet(start), I::I32Add, I::LocalGet(end), I::LocalGet(start), I::I32Sub],
			);
			s.call(f, "new_text");
		});
	}

	/// c_string, error_of, warn_text, text_concat and read_text
	fn emit_text_values_runtime(&mut self) {
		let node_ref = Ref(self.node_ref(false));
		let (int, long) = (ValType::I32, ValType::I64);

		// c_string(text): the address of a zero-terminated copy of the text (or character) for a C function's char*
		if self.should_emit_function(C_STRING) {
			self.emit_text_heap_global();
			self.runtime_function(C_STRING, vec![node_ref], vec![int], vec![int, int], |s, f| {
				let (length, address) = (1, 2);
				f.instruction(&I::LocalGet(0));
				s.call(f, TEXT_OF);
				f.instruction(&I::LocalSet(0));
				s.emit_text_field(f, 0, 1);
				Self::emit_list(f, &[I::I32Const(1), I::I32Add, I::LocalSet(length)]);
				s.emit_text_allocation(f, length, address);
				f.instruction(&I::LocalGet(address));
				s.emit_text_field(f, 0, 0);
				s.emit_text_field(f, 0, 1);
				f.instruction(&I::MemoryCopy { src_mem: 0, dst_mem: 0 });
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(address), I::LocalGet(length), I::I32Add, I::I32Const(1), I::I32Sub, I::I32Const(0),
					I::I32Store8(BYTE),
				]);
				f.instruction(&I::LocalGet(address));
			});
		}

		// error_of(text): an Error node carrying the text as its reason
		if self.should_emit_function(ERROR_OF) {
			self.exported_function(ERROR_OF, vec![node_ref], vec![node_ref], vec![], |s, f| {
				Self::emit_list(f, &[I::LocalGet(0)]);
				s.call(f, TEXT_OF);
				Self::emit_list(f, &[I::LocalSet(0), I::I64Const(Kind::Error as i64)]);
				s.emit_field(f, 0, 1);
				Self::emit_list(
					f,
					&[I::RefNull(HeapType::Concrete(s.type_manager.node_type)), I::StructNew(s.type_manager.node_type)],
				);
			});
		}

		// warn_text(message): reports the message through the host, yields ""
		if self.should_emit_function(WARN_TEXT) {
			if let Some(host_warn) = self.ctx.func_registry.get(HOST_WARN).map(|f| f.call_index as u32) {
				self.runtime_function(WARN_TEXT, vec![node_ref], vec![node_ref], vec![], |s, f| {
					f.instruction(&I::LocalGet(0));
					s.call(f, TEXT_OF);
					f.instruction(&I::LocalSet(0));
					s.emit_text_field(f, 0, 0);
					s.emit_text_field(f, 0, 1);
					Self::emit_list(f, &[I::Call(host_warn), I::I32Const(0), I::I32Const(0)]);
					s.call(f, "new_text");
				});
			}
		}

		self.emit_text_joiner(TEXT_CONCAT, false);
		self.emit_text_joiner(TEXT_APPEND, true);

		// read_text(path): the file's bytes as text, or an Error node with the reason
		if self.should_emit_function(READ_TEXT) {
			if let Some(host_read) = self.ctx.func_registry.get(HOST_READ).map(|f| f.call_index as u32) {
				self.emit_text_heap_global();
				let (length, pointer) = (1, 2);
				self.runtime_function(READ_TEXT, vec![node_ref], vec![node_ref], vec![long; 2], |s, f| {
					s.emit_text_field(f, 0, 0);
					s.emit_text_field(f, 0, 1);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::Call(host_read), I::I64ExtendI32S, I::LocalSet(length),
						I::I64ExtendI32U, I::LocalSet(pointer),
					]);
					s.emit_host_text_result(f, length, pointer);
				});
			}
		}
	}

	/// text_concat(left, right): a fresh text, left's bytes then right's; an Error operand is the result, errors
	/// propagate. text_append, called for a variable's text, also grows texts into rooms (text_rooms.rs)
	fn emit_text_joiner(&mut self, name: &'static str, grows_in_rooms: bool) {
		if !self.should_emit_function(name) {
			return;
		}
		let node_ref = Ref(self.node_ref(false));
		let int = ValType::I32;
		self.emit_text_heap_global();
		let (left_length, right_length, length, copy, room) = (2, 3, 4, 5, 6);
		self.runtime_function(name, vec![node_ref, node_ref], vec![node_ref], vec![int; 5], |s, f| {
			for side in 0..2 {
				s.emit_field(f, side, 0);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::I64Const(Kind::Error as i64), I::I64Eq, I::If(BlockType::Empty),
						I::LocalGet(side), I::Return,
					I::End,
				]);
			}
			for side in 0..2 {
				f.instruction(&I::LocalGet(side));
				s.call(f, TEXT_OF);
				f.instruction(&I::LocalSet(side));
				// a type value joins as it prints, `class int` (decision type-print)
				s.emit_is_type_value(f, side);
				f.instruction(&I::If(BlockType::Empty));
				s.emit_type_value_text(f, side);
				Self::emit_list(f, &[I::LocalSet(side), I::End]);
			}
			s.emit_text_field(f, 0, 1);
			f.instruction(&I::LocalTee(left_length));
			s.emit_text_field(f, 1, 1);
			Self::emit_list(f, &[I::LocalTee(right_length), I::I32Add, I::LocalSet(length)]);
			let copy_bytes = I::MemoryCopy { src_mem: 0, dst_mem: 0 };
			let heap = s.text_heap_global.expect("emit_text_heap_global before text_concat");
			// a left text ending where the heap starts (`t += "x"` in a loop) grows in place: its own bytes stay as they
			// are, right's follow them, so building a text is linear, not quadratic. Right made just now (a character's
			// bytes) already follows left: nothing to copy
			s.emit_text_field(f, 0, 0);
			Self::emit_list(f, &[I::LocalTee(copy), I::LocalGet(left_length), I::I32Add]);
			s.emit_text_field(f, 1, 0);
			Self::emit_list(f, &[I::I32Eq]);
			s.emit_text_field(f, 1, 0);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(right_length), I::I32Add, I::GlobalGet(heap), I::I32Eq, I::I32And, I::LocalGet(left_length),
				I::I32Const(0), I::I32Ne, I::I32And,
			]);
			Self::emit_list(f, &[I::If(BlockType::Empty), I::Else]);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(copy), I::LocalGet(left_length), I::I32Add, I::GlobalGet(heap), I::I32Eq,
				I::LocalGet(left_length), I::I32Const(0), I::I32Ne, I::I32And,
			]);
			f.instruction(&I::If(BlockType::Empty));
			s.emit_memory_room(f, heap, right_length);
			f.instruction(&I::GlobalGet(heap));
			s.emit_text_field(f, 1, 0);
			Self::emit_list(
				f,
				&[
					I::LocalGet(right_length),
					copy_bytes.clone(),
					I::GlobalGet(heap),
					I::LocalGet(right_length),
					I::I32Add,
					I::GlobalSet(heap),
				],
			);
			// a left text ending where its room is used up grows there (text_rooms.rs); else both are copied
			Self::emit_list(f, &[I::Else, I::Block(BlockType::Empty)]);
			if grows_in_rooms {
				s.emit_grow_in_room(f, copy, left_length, right_length, room, 0);
			}
			s.emit_text_allocation(f, length, copy);
			f.instruction(&I::LocalGet(copy));
			s.emit_text_field(f, 0, 0);
			Self::emit_list(
				f,
				&[I::LocalGet(left_length), copy_bytes.clone(), I::LocalGet(copy), I::LocalGet(left_length), I::I32Add],
			);
			s.emit_text_field(f, 1, 0);
			Self::emit_list(f, &[I::LocalGet(right_length), copy_bytes]);
			if grows_in_rooms {
				s.emit_room_behind(f, copy, length, room);
			}
			Self::emit_list(f, &[I::End, I::End, I::End, I::LocalGet(copy), I::LocalGet(length)]);
			s.call(f, "new_text");
		});
	}
}

/// The call `join(list, separator)`: the items' text forms with the separator between them
pub(crate) fn join_call(list: Node, separator: &str) -> Node {
	Node::List(
		vec![Node::Symbol("join".to_string()), list, Node::Text(separator.to_string())],
		Bracket::Round,
		Separator::None,
	)
}

pub(crate) fn joined_text(items: &[Node], separator: &str) -> Node {
	join_call(Node::List(items.to_vec(), Bracket::Square, Separator::Space), separator)
}
