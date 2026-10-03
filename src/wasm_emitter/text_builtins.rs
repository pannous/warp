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
const BYTE_AT: &str = "byte_at";
const BYTE_SLICE: &str = "byte_slice";
const READ: &str = "read";
const READ_TEXT: &str = "read_text";
const HOST_READ: &str = "host_read";
const ERROR: &str = "error";
/// `is_error(x)`: 1 when x is an Error value; `try X else Y` tests its result with it
const IS_ERROR: &str = "is_error";
const ERROR_OF: &str = "error_of";
const WARNING: &str = "warning";
const WARN_TEXT: &str = "warn_text";
const HOST_WARN: &str = "host_warn";
/// text_with_char_at encodes a code point as UTF-8; it is emitted with node_with_at
const CHARACTER_ENCODER: &str = "node_with_at";
/// `text_form(x)`: the text of a value, what an interpolation hole `"\(x)"` becomes (interpolation.rs)
pub const TEXT_FORM: &str = "text_form";

/// name, number of arguments, result kind
const TEXT_BUILTINS: [(&str, usize, Kind); 8] = [
	(READ, 1, Kind::Text), (BYTE_AT, 2, Kind::Int), (BYTE_SLICE, 3, Kind::Text), (ERROR, 1, Kind::Text), (IS_ERROR, 1, Kind::Int),
	(WARNING, 1, Kind::Text), (TEXT_FORM, 1, Kind::Text), (RAN_WITHOUT_ERROR, 1, Kind::Int),
];

pub fn text_builtin_kind(name: &str, arguments: usize) -> Option<Kind> {
	TEXT_BUILTINS.iter().find(|(builtin, arity, _)| *builtin == name && *arity == arguments).map(|(_, _, kind)| *kind)
}

pub fn is_text_builtin(name: &str) -> bool {
	TEXT_BUILTINS.iter().any(|(builtin, _, _)| *builtin == name)
}

/// `+` of two texts or characters is a text; a number joins a text in its text form (`"F:" + 13` → `"F:13"`, as JS/Kotlin)
pub fn concatenates(left: Kind, right: Kind) -> bool {
	let is_text = |kind: &Kind| matches!(kind, Kind::Text | Kind::Codepoint);
	[left, right].iter().any(is_text) && [left, right].iter().all(|kind| is_text(kind) || is_number(*kind))
}

fn is_number(kind: Kind) -> bool {
	matches!(kind, Kind::Int | Kind::Float)
}

/// Runtime functions the text builtins call
pub fn add_dependencies(required: &mut HashSet<&'static str>) {
	if required.contains(super::library_ops::NODE_SLICE) {
		required.extend(["text_chars", "list_join", "list_reverse"]);
	}
	if crate::library_words::MAP_WORD_FUNCTIONS.iter().any(|name| required.contains(name)) {
		required.insert("map_find");
	}
	if required.contains(super::list_ops::NODE_AT_KEY) {
		required.extend(["node_index_at", "map_get"]);
	}
	if required.contains(crate::library_words::FIELD_WITH) {
		required.insert(super::list_ops::STRUCT_BODY); // an instance keeps its type
	}
	if required.contains(super::list_ops::NODE_WITH_KEY) {
		required.extend(["node_with_at", crate::library_words::FIELD_WITH]);
	}
	// maps compare keys by name: map_key_name reads a character key as its text
	if ["map_get", "map_find", "field_with"].iter().any(|name| required.contains(name)) {
		required.extend([TEXT_OF, crate::wasm_emitter::VALUES_EQUAL]);
	}
	if required.contains(super::list_ops::TEXT_AS_INT) || required.contains(super::list_ops::TEXT_AS_FLOAT) {
		required.insert("get_int_value");
	}
	if required.contains("list_sort") {
		required.insert(super::library_ops::NODE_ORDER);
	}
	if required.contains(super::wasi_emitter::PRINT_VALUE) {
		required.insert("list_join");
	}
	if required.contains("list_join") {
		required.insert(super::float_text::FLOAT_TEXT); // a float joins as its text
	}
	// numbers that are no fixnum (big integers, ratios) join as their exact text, built by text_concat
	if required.contains("list_join") && required.contains(super::INT_RUNTIME) {
		required.extend([super::exact::EXACT_TEXT, TEXT_CONCAT]);
	}
	let calls_text_of = [crate::wasm_emitter::VALUES_EQUAL, TEXT_CONCAT, ERROR_OF, WARN_TEXT, "list_join", "text_upper", "text_lower", "text_split", "list_reverse", "text_chars", "list_sort", super::library_ops::NODE_ORDER];
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
			// a hole names a variable: a word that names nothing is loud, never its own spelling
			(TEXT_FORM, [value]) => match value.drop_meta() {
				Node::Symbol(name) if self.is_unbound(name) => self.emit_undefined_variable(func, name),
				_ => self.emit_runtime_text_cast(func, value),
			},
			(BYTE_AT | IS_ERROR | RAN_WITHOUT_ERROR, _) => {
				self.emit_integer_text_builtin(func, name, arguments);
				self.emit_call(func, "new_int");
			}
			(BYTE_SLICE, [text, start, end]) => {
				self.emit_node_instructions(func, text);
				self.emit_numeric_value(func, start);
				self.emit_numeric_value(func, end);
				self.emit_call(func, BYTE_SLICE);
			}
			_ => unreachable!("text_builtin_kind admits {name} with {} arguments", arguments.len()),
		}
	}

	/// `byte_at(text, offset)` and `is_error(x)` as raw i64
	pub(super) fn emit_integer_text_builtin(&mut self, func: &mut Function, name: &str, arguments: &[Node]) {
		match (name, arguments) {
			(BYTE_AT, [text, offset]) => {
				self.emit_node_instructions(func, text);
				self.emit_numeric_value(func, offset);
				self.emit_call(func, BYTE_AT);
			}
			(IS_ERROR, [value]) => {
				self.emit_node_instructions(func, value);
				func.instruction(&I::StructGet { struct_type_index: self.type_manager.node_type, field_index: 0 });
				Self::emit_list(func, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::Error as i64), I::I64Eq, I::I64ExtendI32U]);
			}
			(RAN_WITHOUT_ERROR, [statement]) => self.emit_ran_without_error(func, statement),
			_ => unreachable!("{name} is no integer text builtin with {} arguments", arguments.len()),
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
		if !is_number(self.get_type(operand)) {
			self.emit_node_instructions(func, operand);
			return;
		}
		let written = crate::normalize::operand_text(operand);
		crate::normalize::hint(&written, &format!("str({written})"), "the number joins the text in its text form");
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
	}

	pub(super) fn emit_text_builtins(&mut self) {
		let node_ref = Ref(self.node_ref(false));
		let (int, long) = (ValType::I32, ValType::I64);

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

		// error_of(text): an Error node carrying the text as its reason
		if self.should_emit_function(ERROR_OF) {
			self.runtime_function(ERROR_OF, vec![node_ref], vec![node_ref], vec![], |s, f| {
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
				s.emit_text_allocation(f, length, copy);
				let copy_bytes = I::MemoryCopy { src_mem: 0, dst_mem: 0 };
				f.instruction(&I::LocalGet(copy));
				s.emit_text_field(f, 0, 0);
				Self::emit_list(f, &[I::LocalGet(left_length), copy_bytes.clone(), I::LocalGet(copy), I::LocalGet(left_length), I::I32Add]);
				s.emit_text_field(f, 1, 0);
				Self::emit_list(f, &[I::LocalGet(right_length), copy_bytes, I::LocalGet(copy), I::LocalGet(length)]);
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
