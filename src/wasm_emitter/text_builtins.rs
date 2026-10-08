//! Text as UTF-8 bytes: `a + b` concatenates texts and characters, `read(path)` loads a file,
//! `byte_at(text, offset)` and `byte_slice(text, start, end)` address bytes by 0-based offset, end exclusive.
//! `error(message)` makes an Error value of a text, the way a failed `read` reports.
//! Offsets are bytes, not characters: code that scans UTF-8 itself (a parser, a binary index) needs them.

use crate::node::Node;
use crate::type_kinds::{Kind, KIND_MASK};
use crate::wasm_emitter::{WasmGcEmitter, RAN_WITHOUT_ERROR};
use std::collections::HashSet;
use wasm_encoder::*;
use Instruction as I;
use ValType::Ref;
use crate::wasm_emitter::layout::BYTE;

pub const TEXT_CONCAT: &str = "text_concat";
pub const TEXT_OF: &str = "text_of";
/// text_argument(node): the text a text builtin reads (text_of), but an Error fails with its own message (card byte-slice)
const TEXT_ARGUMENT: &str = "text_argument";
/// The builtins that read their text through text_argument
const TEXT_ARGUMENT_USERS: [&str; 5] = [BYTE_AT, BYTE_SLICE, TEXT_TRIM, TEXT_STARTS_WITH, TEXT_ENDS_WITH];
/// c_string(text) -> i32: a zero-terminated copy of a text known only at run time, for a C function's `char*`
pub const C_STRING: &str = "c_string";
const BYTE_AT: &str = "byte_at";
const BYTE_SLICE: &str = "byte_slice";
/// `memory_byte(address)`, `memory_set_byte(address, value)`: one byte of linear memory, what lib/memory.wasp's
/// `memory.slice` and `memory.copy` are made of (samples/wasm_interop.wasp)
const MEMORY_BYTE: &str = "memory_byte";
const MEMORY_SET_BYTE: &str = "memory_set_byte";
/// `trim(text)`: the text without the whitespace at either end, sharing its memory
const TRIM: &str = "trim";
const TEXT_TRIM: &str = "text_trim";
/// `chr(n)`: the character of a code point, the inverse of `ord(c)`
const CHR: &str = "chr";
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
const READ: &str = "read";
const READ_TEXT: &str = "read_text";
const HOST_READ: &str = "host_read";
const ERROR: &str = "error";
/// `raise X` (`throw X`): an exception, the runtime error returned_error with X as its detail, which `try` catches
pub const RAISE: &str = "raise";
/// `is_error(x)`: 1 when x is an Error value; `try X else Y` tests its result with it
pub const IS_ERROR: &str = "is_error";
/// error_of(text) → Error: exported, the host builds the Error of a caught stack overflow with it (guarded_call)
pub const ERROR_OF: &str = "error_of";
const WARNING: &str = "warning";
const WARN_TEXT: &str = "warn_text";
const HOST_WARN: &str = "host_warn";
/// text_with_char_at encodes a code point as UTF-8; it is emitted with node_with_at
const CHARACTER_ENCODER: &str = "node_with_at";
/// `text_form(x)`: the text of a value, what an interpolation hole `"\(x)"` becomes (interpolation.rs)
pub const TEXT_FORM: &str = "text_form";

/// name, number of arguments, result kind
const TEXT_BUILTINS: [(&str, usize, Kind); 19] = [
	(MEMORY_BYTE, 1, Kind::Int), (MEMORY_SET_BYTE, 2, Kind::Int),
	(crate::memoization::MEMO_KNOWN, 2, Kind::Int), (crate::memoization::MEMO_VALUE, 2, Kind::Int), (crate::memoization::MEMO_STORE, 3, Kind::Int),
	(READ, 1, Kind::Text), (BYTE_AT, 2, Kind::Int), (BYTE_SLICE, 3, Kind::Text), (ERROR, 1, Kind::Text), (RAISE, 1, Kind::Text), (IS_ERROR, 1, Kind::Int),
	(WARNING, 1, Kind::Text), (TEXT_FORM, 1, Kind::Text), (RAN_WITHOUT_ERROR, 1, Kind::Int), (TRIM, 1, Kind::Text),
	(STARTS_WITH, 2, Kind::Int), (ENDS_WITH, 2, Kind::Int), (CHR, 1, Kind::Codepoint), (crate::wasm_emitter::CAUGHT_ERROR, 2, Kind::Error),
];

pub fn text_builtin_kind(name: &str, arguments: usize) -> Option<Kind> {
	TEXT_BUILTINS.iter().find(|(builtin, arity, _)| *builtin == name && *arity == arguments).map(|(_, _, kind)| *kind)
}

/// The numbers of arguments a text builtin takes, empty for any other name
pub fn text_builtin_arities(name: &str) -> Vec<usize> {
	TEXT_BUILTINS.iter().filter(|(builtin, _, _)| *builtin == name).map(|(_, arity, _)| *arity).collect()
}

pub fn is_text_builtin(name: &str) -> bool {
	TEXT_BUILTINS.iter().any(|(builtin, _, _)| *builtin == name)
}

/// `+` of two texts or characters is a text; a number joins a text in its text form (`"F:" + 13` → `"F:13"`, as JS/Kotlin)
pub fn concatenates(left: Kind, right: Kind) -> bool {
	let is_text = |kind: &Kind| matches!(kind, Kind::Text | Kind::Codepoint);
	// an Error joins as its message (card try-raise: `"caught: " + e` of `catch e`)
	[left, right].iter().any(is_text) && [left, right].iter().all(|kind| is_text(kind) || is_number(*kind) || *kind == Kind::Error)
}

fn is_number(kind: Kind) -> bool {
	matches!(kind, Kind::Int | Kind::Float)
}

/// Runtime functions the text builtins call
pub fn add_dependencies(required: &mut HashSet<&'static str>) {
	// `catch e`: the caught error becomes an Error through error_of
	if required.contains(crate::wasm_emitter::CAUGHT_ERROR) {
		required.insert(ERROR_OF);
	}
	if required.contains(super::library_ops::LIST_TEXT) {
		required.insert("list_join"); // emitted together, with the same helpers
	}
	if required.contains(super::library_ops::NODE_SLICE) {
		required.extend(["text_chars", "list_join", "list_reverse"]);
	}
	if crate::library_words::MAP_WORD_FUNCTIONS.iter().any(|name| required.contains(name)) {
		required.insert("map_find");
	}
	if required.contains(super::list_ops::ELEMENT_BODY) {
		required.insert(super::list_ops::ELEMENT_CHILDREN);
	}
	if required.contains(super::list_ops::NODE_AT_KEY) {
		required.extend(["node_index_at", "map_get"]);
	}
	// instance_copy is emitted with field_with, both looking into an instance's fields
	if required.contains(crate::library_words::INSTANCE_COPY) {
		required.insert(crate::library_words::FIELD_WITH);
	}
	if required.contains(crate::library_words::FIELD_WITH) {
		required.insert(super::list_ops::STRUCT_BODY); // an instance keeps its type
	}
	if required.contains(super::list_ops::NODE_WITH_KEY) {
		required.extend([super::list_ops::NODE_WITH_NODE_AT, crate::library_words::FIELD_WITH]);
	}
	if required.contains(super::list_ops::NODE_WITH_NODE_AT) {
		required.extend(["node_with_at", "get_int_value"]);
	}
	// a map variable held as a hash table names its keys by map_key_name, emitted with map_get
	if super::map_backend::NODE_MAP_FUNCTIONS.iter().any(|name| required.contains(name)) {
		required.extend(super::map_backend::NODE_MAP_FUNCTIONS);
		required.insert("map_get");
	}
	// maps compare keys by name: map_key_name reads a character key as its text
	if ["map_get", "map_find", "field_with"].iter().any(|name| required.contains(name)) {
		required.extend([TEXT_OF, crate::wasm_emitter::VALUES_EQUAL]);
	}
	if required.contains("node_list_of") {
		required.insert("node_list_push");
	}
	if required.contains("exact_euclid_div") {
		required.insert(super::INT_RUNTIME);
	}
	if required.contains(super::list_ops::NODE_ADD) {
		required.extend(["list_concat", TEXT_CONCAT, TEXT_OF, super::library_ops::LIST_JOIN]); // two lists added are concatenated, two texts too, a text and a number joined
	}
	if super::list_ops::NODE_ARITHMETIC.iter().any(|(name, _, _)| required.contains(name)) {
		required.extend([super::list_ops::TEXT_AS_FLOAT, super::INT_RUNTIME, "exact_add", "exact_sub", "exact_mul", "exact_div", "new_float"]);
	}
	if required.contains(super::list_ops::TEXT_AS_INT) || required.contains(super::list_ops::TEXT_AS_FLOAT) {
		required.insert("get_int_value");
	}
	if required.contains(super::list_ops::TEXT_AS_FLOAT) {
		required.insert(super::list_ops::TEXT_AS_INT);
		if required.contains(super::INT_RUNTIME) {
			required.insert("exact_to_f64");
		}
	}
	if required.contains(super::list_ops::TEXT_AS_INT) && required.contains(super::INT_RUNTIME) {
		required.extend(["exact_mul", "exact_add", "exact_sub"]);
	}
	if required.contains("list_sort") {
		required.extend([super::library_ops::NODE_ORDER, "text_chars", "list_join"]);
	}
	if [TEXT_FIND, TEXT_STARTS_WITH, TEXT_ENDS_WITH].iter().any(|name| required.contains(name)) {
		required.insert(TEXT_MATCHES_AT);
	}
	// a text searched for a text (`s.contains("b")`, `"b" in s`); the map words are emitted together
	if crate::library_words::MAP_WORD_FUNCTIONS.iter().any(|name| required.contains(name)) {
		required.extend([TEXT_FIND, TEXT_MATCHES_AT, TEXT_OF]);
	}
	if required.contains(super::wasi_emitter::PRINT_VALUE) || required.contains(super::wasi_emitter::PUT_VALUE) {
		required.insert("list_join");
	}
	// the text of a list (list_text) is emitted with join, as the analyzer requires list_join for both
	if required.contains("list_join") {
		required.extend([super::float_text::FLOAT_TEXT, TEXT_CONCAT, super::library_ops::LIST_TEXT, super::library_ops::TEXT_QUOTED]); // a float joins as its text, a nested list as "[…]", a text quoted
	}
	// numbers that are no fixnum (big integers, ratios) join as their exact text, built by text_concat
	if required.contains("list_join") && required.contains(super::INT_RUNTIME) {
		required.extend([super::exact::EXACT_TEXT, TEXT_CONCAT]);
	}
	// codepoint_of reads a one-character text as its character
	if required.contains(super::library_ops::CODEPOINT_OF) {
		required.extend(["string_char_at", "text_grapheme_count"]);
	}
	if TEXT_ARGUMENT_USERS.iter().any(|name| required.contains(name)) {
		required.insert(TEXT_ARGUMENT);
	}
	if required.contains(TEXT_ARGUMENT) {
		required.insert(TEXT_OF);
	}
	let calls_text_of = [crate::wasm_emitter::VALUES_EQUAL, TEXT_CONCAT, BYTE_AT, BYTE_SLICE, TEXT_TRIM, C_STRING, ERROR_OF, WARN_TEXT, "list_join", "text_upper", "text_lower", "text_split", "list_reverse", "text_chars", "list_sort", super::library_ops::NODE_ORDER];
	if calls_text_of.iter().any(|name| required.contains(name)) {
		required.insert(TEXT_OF);
	}
	if required.contains("text_chars") {
		required.extend(["list_reverse", "text_reverse"]); // the characters are collected backwards, and list_reverse hands texts on
	}
	if required.contains("list_reverse") {
		required.insert("text_reverse");
	}
	if required.contains(TEXT_OF) {
		required.insert(CHARACTER_ENCODER);
	}
}

impl WasmGcEmitter {
	/// `read(path)`, `byte_at(text, offset)`, `byte_slice(text, start, end)` as a Node
	pub(super) fn emit_text_builtin(&mut self, func: &mut Function, name: &str, arguments: &[Node]) {
		match (name, arguments) {
			(READ, [path]) if self.ctx.func_registry.get(HOST_READ).is_some() => {
				self.emit_node_instructions(func, path);
				self.emit_call(func, READ_TEXT);
			}
			(READ, [path]) => {
				let reason = format!("read {} failed: host imports are not available", path.serialize());
				self.emit_runtime_error_value(func, &reason);
			}
			// warnings are errors: the message is an Error value, which text concatenation propagates
			(WARNING, [message]) if crate::diagnostic::warning_mode() == crate::diagnostic::WarningMode::Error => {
				self.emit_node_instructions(func, message);
				self.emit_call(func, ERROR_OF);
			}
			(WARNING, [message]) if self.ctx.func_registry.get(HOST_WARN).is_some() => {
				self.emit_node_instructions(func, message);
				self.emit_call(func, WARN_TEXT);
			}
			(WARNING, [message]) => {
				let reason = format!("warning {} not reported: host imports are not available", message.serialize());
				self.emit_runtime_error_value(func, &reason);
			}
			(ERROR, [message]) => {
				self.emit_node_instructions(func, message);
				self.emit_call(func, ERROR_OF);
			}
			(crate::wasm_emitter::CAUGHT_ERROR, [finished, value]) => self.emit_caught_error(func, finished, value),
			(RAISE, [message]) => {
				self.emit_trap_detail(func, message);
				self.emit_runtime_error(func, super::list_ops::RETURNED_ERROR);
			}
			// a hole names a variable: a word that names nothing is loud, never its own spelling
			(TEXT_FORM, [value]) => match value.drop_meta() {
				Node::Symbol(name) if self.is_unbound(name) => self.emit_undefined_variable(func, name),
				_ => self.emit_runtime_text_cast(func, value),
			},
			(BYTE_AT | MEMORY_BYTE | MEMORY_SET_BYTE | IS_ERROR | RAN_WITHOUT_ERROR | STARTS_WITH | ENDS_WITH | crate::memoization::MEMO_KNOWN | crate::memoization::MEMO_VALUE | crate::memoization::MEMO_STORE, _) => {
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
			(CHR, [code]) => {
				self.emit_numeric_value(func, code);
				func.instruction(&I::I32WrapI64);
				self.emit_call(func, "new_codepoint");
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
	pub(super) fn emit_integer_text_builtin(&mut self, func: &mut Function, name: &str, arguments: &[Node]) {
		match (name, arguments) {
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
			(IS_ERROR, [value]) => {
				self.emit_node_instructions(func, value);
				func.instruction(&I::StructGet { struct_type_index: self.type_manager.node_type, field_index: 0 });
				Self::emit_list(func, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::Error as i64), I::I64Eq, I::I64ExtendI32U]);
			}
			(RAN_WITHOUT_ERROR, [statement]) => self.emit_ran_without_error(func, statement),
			(STARTS_WITH | ENDS_WITH, [text, part]) => {
				self.emit_text_argument(func, text);
				self.emit_text_argument(func, part);
				self.emit_call(func, if name == STARTS_WITH { TEXT_STARTS_WITH } else { TEXT_ENDS_WITH });
			}
			(crate::memoization::MEMO_KNOWN | crate::memoization::MEMO_VALUE | crate::memoization::MEMO_STORE, [id, argument, rest @ ..]) => {
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
		let Node::Number(crate::extensions::numbers::Number::Int(id)) = id.drop_meta() else { unreachable!("memoization passes a constant id") };
		let array = self.type_manager.int_array_type;
		let array_ref = ValType::Ref(RefType { nullable: true, heap_type: HeapType::Concrete(array) });
		let new_global = |emitter: &mut Self| {
			emitter.globals.global(GlobalType { val_type: array_ref, mutable: true, shared: false }, &ConstExpr::ref_null(HeapType::Concrete(array)));
			emitter.next_global_idx += 1;
			emitter.next_global_idx - 1
		};
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
			Self::emit_list(func, &[I::GlobalGet(global), I::RefIsNull, I::If(BlockType::Empty), I::I32Const(MEMO_SIZE), I::ArrayNewDefault(array), I::GlobalSet(global), I::End]);
		}
		let in_range = [I::LocalGet(index), I::I64Const(0), I::I64GeS, I::LocalGet(index), I::I64Const(MEMO_SIZE as i64), I::I64LtS, I::I32And];
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
	pub(super) fn emit_text_concat(&mut self, func: &mut Function, left: &Node, right: &Node) {
		self.emit_concatenated(func, left);
		self.emit_concatenated(func, right);
		self.emit_call(func, TEXT_CONCAT);
	}

	/// A text operand as is, a number in its text form; the implicit conversion is hinted
	fn emit_concatenated(&mut self, func: &mut Function, operand: &Node) {
		if self.get_type(operand) == Kind::Empty {
			// a value held as a Node (a map value) may be a number at runtime: joined, a number takes its text form
			self.emit_node_instructions(func, &super::joined_text(std::slice::from_ref(operand), ""));
			return;
		}
		if self.get_type(operand) == Kind::Error {
			self.emit_runtime_text_cast(func, operand);
			return;
		}
		if !is_number(self.get_type(operand)) {
			self.emit_node_instructions(func, operand);
			return;
		}
		self.emit_cast(func, operand, &Node::Symbol("str".to_string()));
	}

	/// An Error node carrying `reason`, the way a failed fetch reports
	fn emit_runtime_error_value(&mut self, func: &mut Function, reason: &str) {
		let (ptr, len) = self.allocate_string(reason);
		func.instruction(&I::I32Const(ptr as i32));
		func.instruction(&I::I32Const(-(len as i32)));
		let (length, pointer) = (self.scratch(0), self.scratch(1));
		func.instruction(&I::I64ExtendI32S);
		func.instruction(&I::LocalSet(length));
		func.instruction(&I::I64ExtendI32U);
		func.instruction(&I::LocalSet(pointer));
		self.emit_host_text_result(func, length, pointer);
	}

	/// i64 locals `length` and `pointer` from a host call → Node{kind: length < 0 ? Error : Text, $String(pointer, |length|)}
	pub(super) fn emit_host_text_result(&self, func: &mut Function, length: u32, pointer: u32) {
		Self::emit_list(func, &[
			I::I64Const(Kind::Error as i64), I::I64Const(Kind::Text as i64),
			I::LocalGet(length), I::I64Const(0), I::I64LtS, I::Select,
			I::LocalGet(pointer), I::I32WrapI64,
			I::I64Const(0), I::LocalGet(length), I::I64Sub, I::LocalGet(length),
			I::LocalGet(length), I::I64Const(0), I::I64LtS, I::Select, I::I32WrapI64,
			I::StructNew(self.type_manager.string_type),
			I::RefNull(HeapType::Concrete(self.type_manager.node_type)),
			I::StructNew(self.type_manager.node_type),
		]);
	}

	/// text_of(node): a character as a text of its UTF-8 bytes, anything else unchanged; emitted before values_equal, which calls it
	pub(super) fn emit_text_of(&mut self) {
		if !self.should_emit_function(TEXT_OF) {
			return;
		}
		let node_ref = Ref(self.node_ref(false));
		let (placeholder, _) = self.allocate_string(" ");
		self.runtime_function(TEXT_OF, vec![node_ref], vec![node_ref], vec![], |s, f| {
			s.emit_field(f, 0, 0);
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

	pub(super) fn emit_text_builtins(&mut self) {
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
				Self::emit_list(f, &[
					I::LocalGet(1), I::I32WrapI64, I::I32Add,
					I::LocalGet(2), I::LocalGet(1), I::I64Sub, I::I32WrapI64,
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
			self.runtime_function(TEXT_MATCHES_AT, vec![node_ref, node_ref, ValType::I32], vec![ValType::I32], vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32], |s, f| {
				let (offset, text_address, part_address, part_length, index) = (2, 3, 4, 5, 6);
				s.emit_text_field(f, 0, 0);
				f.instruction(&I::LocalSet(text_address));
				s.emit_text_field(f, 1, 0);
				f.instruction(&I::LocalSet(part_address));
				s.emit_text_field(f, 1, 1);
				f.instruction(&I::LocalSet(part_length));
				// out of bounds: no match
				Self::emit_list(f, &[I::LocalGet(offset), I::I32Const(0), I::I32LtS, I::LocalGet(offset), I::LocalGet(part_length), I::I32Add]);
				s.emit_text_field(f, 0, 1);
				Self::emit_list(f, &[I::I32GtU, I::I32Or, I::If(BlockType::Empty), I::I32Const(0), I::Return, I::End]);
				Self::emit_list(f, &[
					I::Block(BlockType::Empty), I::Loop(BlockType::Empty),
					I::LocalGet(index), I::LocalGet(part_length), I::I32GeU, I::BrIf(1),
					I::LocalGet(text_address), I::LocalGet(offset), I::I32Add, I::LocalGet(index), I::I32Add, I::I32Load8U(BYTE),
					I::LocalGet(part_address), I::LocalGet(index), I::I32Add, I::I32Load8U(BYTE),
					I::I32Ne, I::If(BlockType::Empty), I::I32Const(0), I::Return, I::End,
					I::LocalGet(index), I::I32Const(1), I::I32Add, I::LocalSet(index), I::Br(0), I::End, I::End,
					I::I32Const(1),
				]);
			});
		}
		// text_find(text, part): the first offset where part matches, 1-based; 0 when it never does
		if self.should_emit_function(TEXT_FIND) {
			self.runtime_function(TEXT_FIND, vec![node_ref, node_ref], vec![long], vec![ValType::I32], |s, f| {
				let offset = 2;
				Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(offset)]);
				s.emit_text_field(f, 0, 1);
				Self::emit_list(f, &[I::I32GtU, I::BrIf(1), I::LocalGet(0), I::LocalGet(1), I::LocalGet(offset)]);
				s.call(f, TEXT_MATCHES_AT);
				Self::emit_list(f, &[
					I::If(BlockType::Empty), I::LocalGet(offset), I::I32Const(1), I::I32Add, I::I64ExtendI32U, I::Return, I::End,
					I::LocalGet(offset), I::I32Const(1), I::I32Add, I::LocalSet(offset), I::Br(0), I::End, I::End,
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
			self.runtime_function(TEXT_TRIM, vec![node_ref], vec![node_ref], vec![ValType::I32, ValType::I32, ValType::I32], |s, f| {
				let (address, start, end) = (1, 2, 3);
				s.emit_text_field(f, 0, 0);
				f.instruction(&I::LocalSet(address));
				s.emit_text_field(f, 0, 1);
				f.instruction(&I::LocalSet(end));
				let is_whitespace = |f: &mut Function, offset: Vec<I<'static>>| {
					for (index, byte) in WHITESPACE_BYTES.iter().enumerate() {
						Self::emit_list(f, &[I::LocalGet(address)]);
						Self::emit_list(f, &offset);
						Self::emit_list(f, &[I::I32Add, I::I32Load8U(BYTE), I::I32Const(*byte), I::I32Eq]);
						if index > 0 {
							f.instruction(&I::I32Or);
						}
					}
				};
				// leading: start moves right while it is before end and at whitespace
				Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(start), I::LocalGet(end), I::I32GeU, I::BrIf(1)]);
				is_whitespace(f, vec![I::LocalGet(start)]);
				Self::emit_list(f, &[I::I32Eqz, I::BrIf(1), I::LocalGet(start), I::I32Const(1), I::I32Add, I::LocalSet(start), I::Br(0), I::End, I::End]);
				// trailing: end moves left while the byte before it is whitespace
				Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(end), I::LocalGet(start), I::I32LeU, I::BrIf(1)]);
				is_whitespace(f, vec![I::LocalGet(end), I::I32Const(1), I::I32Sub]);
				Self::emit_list(f, &[I::I32Eqz, I::BrIf(1), I::LocalGet(end), I::I32Const(1), I::I32Sub, I::LocalSet(end), I::Br(0), I::End, I::End]);
				Self::emit_list(f, &[I::LocalGet(address), I::LocalGet(start), I::I32Add, I::LocalGet(end), I::LocalGet(start), I::I32Sub]);
				s.call(f, "new_text");
			});
		}
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
				Self::emit_list(f, &[I::LocalGet(address), I::LocalGet(length), I::I32Add, I::I32Const(1), I::I32Sub, I::I32Const(0), I::I32Store8(BYTE)]);
				f.instruction(&I::LocalGet(address));
			});
		}

		// error_of(text): an Error node carrying the text as its reason
		if self.should_emit_function(ERROR_OF) {
			self.exported_function(ERROR_OF, vec![node_ref], vec![node_ref], vec![], |s, f| {
				Self::emit_list(f, &[I::LocalGet(0)]);
				s.call(f, TEXT_OF);
				f.instruction(&I::LocalSet(0));
				f.instruction(&I::I64Const(Kind::Error as i64));
				s.emit_field(f, 0, 1);
				f.instruction(&I::RefNull(HeapType::Concrete(s.type_manager.node_type)));
				f.instruction(&I::StructNew(s.type_manager.node_type));
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

		// text_concat(left, right): a fresh text, left's bytes then right's; an Error operand is the result, errors propagate
		if self.should_emit_function(TEXT_CONCAT) {
			self.emit_text_heap_global();
			let (left_length, right_length, length, copy) = (2, 3, 4, 5);
			self.runtime_function(TEXT_CONCAT, vec![node_ref, node_ref], vec![node_ref], vec![int; 4], |s, f| {
				for side in 0..2 {
					s.emit_field(f, side, 0);
					Self::emit_list(f, &[I::I64Const(Kind::Error as i64), I::I64Eq, I::If(BlockType::Empty), I::LocalGet(side), I::Return, I::End]);
				}
				for side in 0..2 {
					f.instruction(&I::LocalGet(side));
					s.call(f, TEXT_OF);
					f.instruction(&I::LocalSet(side));
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
				Self::emit_list(f, &[I::LocalGet(right_length), I::I32Add, I::GlobalGet(heap), I::I32Eq, I::I32And, I::LocalGet(left_length), I::I32Const(0), I::I32Ne, I::I32And]);
				f.instruction(&I::If(BlockType::Empty));
				f.instruction(&I::Else);
				Self::emit_list(f, &[I::LocalGet(copy), I::LocalGet(left_length), I::I32Add, I::GlobalGet(heap), I::I32Eq, I::LocalGet(left_length), I::I32Const(0), I::I32Ne, I::I32And]);
				f.instruction(&I::If(BlockType::Empty));
				s.emit_memory_room(f, heap, right_length);
				f.instruction(&I::GlobalGet(heap));
				s.emit_text_field(f, 1, 0);
				Self::emit_list(f, &[I::LocalGet(right_length), copy_bytes.clone(), I::GlobalGet(heap), I::LocalGet(right_length), I::I32Add, I::GlobalSet(heap)]);
				f.instruction(&I::Else);
				s.emit_text_allocation(f, length, copy);
				f.instruction(&I::LocalGet(copy));
				s.emit_text_field(f, 0, 0);
				Self::emit_list(f, &[I::LocalGet(left_length), copy_bytes.clone(), I::LocalGet(copy), I::LocalGet(left_length), I::I32Add]);
				s.emit_text_field(f, 1, 0);
				Self::emit_list(f, &[I::LocalGet(right_length), copy_bytes, I::End, I::End, I::LocalGet(copy), I::LocalGet(length)]);
				s.call(f, "new_text");
			});
		}

		// read_text(path): the file's bytes as text, or an Error node with the reason
		if self.should_emit_function(READ_TEXT) {
			if let Some(host_read) = self.ctx.func_registry.get(HOST_READ).map(|f| f.call_index as u32) {
				self.emit_text_heap_global();
				let (length, pointer) = (1, 2);
				self.runtime_function(READ_TEXT, vec![node_ref], vec![node_ref], vec![long; 2], |s, f| {
					s.emit_text_field(f, 0, 0);
					s.emit_text_field(f, 0, 1);
					Self::emit_list(f, &[
						I::Call(host_read),
						I::I64ExtendI32S, I::LocalSet(length), I::I64ExtendI32U, I::LocalSet(pointer),
					]);
					s.emit_host_text_result(f, length, pointer);
				});
			}
		}
	}
}
