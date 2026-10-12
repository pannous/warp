//! Nodes read and built by warp code, as a parser written in warp must (card self-node-api, notes/self_hosting.md):
//! `n.key`, `n.value` and `n.op` of a key, `n.name` of a word or a key, `n.bracket` of a list (a field of that name
//! wins: `{name: 3}.name` is 3), and `symbol(text)`, `key(left, op, right)`, `bracketed(items, bracket)` to build them.
//! `type(n)`, `#n`, `n#i` and `for x in n` read the rest already.

use crate::node::Node;
use crate::operators::OP_CODES;
use crate::type_kinds::{Kind, KIND_BITS, KIND_MASK};
use crate::wasm_emitter::layout::BYTE;
use crate::wasm_emitter::text_builtins::{TEXT_CONCAT, TEXT_OF};
use crate::wasm_emitter::WasmGcEmitter;
use std::collections::HashSet;
use wasm_encoder::*;
use Instruction as I;
use ValType::Ref;

/// node_part(node, part) -> the part (NODE_PARTS' index) of a node, null when it has none
pub const NODE_PART: &str = "node_part";
/// The parts node_part reads, by the field name that reads each
pub const NODE_PARTS: [&str; 5] = ["key", "value", "op", "name", "bracket"];
const KEY_PART: i32 = 0;
const VALUE_PART: i32 = 1;
const OP_PART: i32 = 2;
const NAME_PART: i32 = 3;
const BRACKET_PART: i32 = 4;
/// operator_code(text) -> i64: the code of the operator written so (operators::OP_CODES), -1 for none
const OPERATOR_CODE: &str = "operator_code";
/// bracket_code(text) -> i64: the bracket info of the opening bracket written so, -1 for none
const BRACKET_CODE: &str = "bracket_code";
const NEW_KEY_OF: &str = "new_key_of";
const NEW_BRACKETED: &str = "new_bracketed";
const SYMBOL: &str = "symbol";
const KEY: &str = "key";
const BRACKETED: &str = "bracketed";
/// name, number of arguments, result kind
pub const NODE_BUILTINS: [(&str, usize, Kind); 3] = [
	(SYMBOL, 1, Kind::Symbol),
	(KEY, 3, Kind::Key),
	(BRACKETED, 2, Kind::List),
];
/// The opening brackets by their bracket info in a list's kind field (Curly 0, Square 1, Round 2, Less 3, None 5)
const BRACKETS: [(&str, i64); 5] = [("{", 0), ("[", 1), ("(", 2), ("<", 3), ("", 5)];
const UNKNOWN_OPERATOR: &str = "unknown operator: ";
const UNKNOWN_BRACKET: &str = "unknown bracket: ";

/// The part a field name reads when the node has no field of that name
pub fn node_part(field: &str) -> Option<i32> {
	NODE_PARTS
		.iter()
		.position(|part| *part == field)
		.map(|part| part as i32)
}

/// Runtime functions the node words call
pub fn add_dependencies(required: &mut HashSet<&'static str>) {
	if required.contains(NODE_PART) {
		required.insert("new_text");
	}
	if [NEW_KEY_OF, NEW_BRACKETED].iter().any(|name| required.contains(name)) {
		required.extend([
			OPERATOR_CODE,
			BRACKET_CODE,
			TEXT_OF,
			TEXT_CONCAT,
			super::text_builtins::ERROR_OF,
			"new_key",
			"new_text",
		]);
	}
}

impl WasmGcEmitter {
	/// `symbol(text)`, `key(left, op, right)`, `bracketed(items, bracket)`
	pub(super) fn emit_node_builtin(&mut self, func: &mut Function, name: &str, arguments: &[Node]) {
		for argument in arguments {
			self.emit_node_instructions(func, argument);
			func.instruction(&I::RefAsNonNull);
		}
		match name {
			SYMBOL => {
				self.emit_call(func, TEXT_OF);
				let text = self.node_scratch();
				func.instruction(&I::LocalTee(text));
				self.emit_call(func, "get_text_ptr");
				func.instruction(&I::LocalGet(text));
				self.emit_call(func, "get_text_len");
				self.emit_call(func, "new_symbol");
			}
			KEY => self.emit_call(func, NEW_KEY_OF),
			_ => self.emit_call(func, NEW_BRACKETED),
		}
	}

	pub(super) fn emit_node_api(&mut self) {
		let node_ref = Ref(self.node_ref(false));
		let nullable_node_ref = Ref(self.node_ref(true));
		let node_type = self.type_manager.node_type;
		let kind_of = |node: u32| {
			[
				I::LocalGet(node),
				I::StructGet {
					struct_type_index: node_type,
					field_index: 0,
				},
			]
		};

		if self.should_emit_function(NODE_PART) {
			let (node, part, kind, code) = (0, 1, 2, 3);
			self.runtime_function(
				NODE_PART,
				vec![nullable_node_ref, ValType::I32],
				vec![nullable_node_ref],
				vec![ValType::I64, ValType::I32],
				|s, f| {
					let is_kind = |kind_local: u32, wanted: Kind| {
						[
							I::LocalGet(kind_local),
							I::I64Const(KIND_MASK),
							I::I64And,
							I::I64Const(wanted as i64),
							I::I64Eq,
						]
					};
					Self::emit_list(
						f,
						&[
							I::LocalGet(node),
							I::RefIsNull,
							I::If(BlockType::Empty),
							I::RefNull(HeapType::Concrete(node_type)),
							I::Return,
							I::End,
						],
					);
					Self::emit_list(f, &kind_of(node));
					Self::emit_list(
						f,
						&[
							I::LocalTee(kind),
							I::I64Const(KIND_BITS),
							I::I64ShrU,
							I::I32WrapI64,
							I::I32Const(0xFF),
							I::I32And,
							I::LocalSet(code),
						],
					);
					let part_is = |wanted: i32| [I::LocalGet(part), I::I32Const(wanted), I::I32Eq];
					// a key's own parts
					for (wanted, field_index) in [(KEY_PART, 1), (VALUE_PART, 2)] {
						Self::emit_list(f, &part_is(wanted));
						Self::emit_list(f, &is_kind(kind, Kind::Key));
						Self::emit_list(
							f,
							&[
								I::I32And,
								I::If(BlockType::Empty),
								I::LocalGet(node),
								I::StructGet {
									struct_type_index: node_type,
									field_index,
								},
							],
						);
						if field_index == 1 {
							f.instruction(&I::RefCastNullable(HeapType::Concrete(node_type)));
						}
						Self::emit_list(f, &[I::Return, I::End]);
					}
					let written: Vec<(i64, &str)> = OP_CODES
						.iter()
						.enumerate()
						.map(|(code, op)| (code as i64, op.as_str()))
						.collect();
					s.emit_part_text(f, &part_is(OP_PART), &is_kind(kind, Kind::Key), code, &written);
					s.emit_part_text(
						f,
						&part_is(BRACKET_PART),
						&is_kind(kind, Kind::List),
						code,
						&BRACKETS.map(|(bracket, info)| (info, bracket)),
					);
					// a word's name, or the name of a key's key (`x` of `x: 3`, of `x.y: 3` too)
					Self::emit_list(f, &part_is(NAME_PART));
					Self::emit_list(f, &[I::If(BlockType::Empty), I::Loop(BlockType::Empty)]);
					Self::emit_list(f, &is_kind(kind, Kind::Key));
					Self::emit_list(
						f,
						&[
							I::If(BlockType::Empty),
							I::LocalGet(node),
							I::StructGet {
								struct_type_index: node_type,
								field_index: 1,
							},
							I::RefCastNonNull(HeapType::Concrete(node_type)),
							I::LocalTee(node),
							I::StructGet {
								struct_type_index: node_type,
								field_index: 0,
							},
							I::LocalSet(kind),
							I::Br(1),
							I::End,
							I::End,
						],
					);
					Self::emit_list(f, &is_kind(kind, Kind::Symbol));
					Self::emit_list(f, &is_kind(kind, Kind::Text));
					Self::emit_list(f, &[I::I32Or, I::If(BlockType::Empty)]);
					s.emit_text_field(f, node, 0);
					s.emit_text_field(f, node, 1);
					s.call(f, "new_text");
					Self::emit_list(
						f,
						&[I::Return, I::End, I::End, I::RefNull(HeapType::Concrete(node_type))],
					);
				},
			);
		}

		if self.should_emit_function(NEW_KEY_OF) {
			self.emit_code_of_text(
				OPERATOR_CODE,
				&OP_CODES
					.iter()
					.enumerate()
					.map(|(code, op)| (code as i64, op.as_str()))
					.collect::<Vec<_>>(),
			);
			// new_key_of(left, op, right): the key `left op right`, an Error for a text no operator is written as
			let (left, op, right) = (0, 1, 2);
			self.runtime_function(
				NEW_KEY_OF,
				vec![node_ref, node_ref, node_ref],
				vec![node_ref],
				vec![ValType::I64],
				|s, f| {
					let code = 3;
					s.emit_code_or_error(f, op, code, OPERATOR_CODE, UNKNOWN_OPERATOR);
					Self::emit_list(f, &[I::LocalGet(left), I::LocalGet(right), I::LocalGet(code)]);
					s.call(f, "new_key");
				},
			);
		}

		if self.should_emit_function(NEW_BRACKETED) {
			self.emit_code_of_text(BRACKET_CODE, &BRACKETS.map(|(bracket, info)| (info, bracket)));
			// new_bracketed(items, bracket): the list of the same items in that bracket, an Error for an unknown bracket
			let (items, bracket) = (0, 1);
			self.runtime_function(
				NEW_BRACKETED,
				vec![node_ref, node_ref],
				vec![node_ref],
				vec![ValType::I64],
				|s, f| {
					let code = 2;
					Self::emit_list(f, &kind_of(items));
					Self::emit_list(
						f,
						&[
							I::I64Const(KIND_MASK),
							I::I64And,
							I::I64Const(Kind::List as i64),
							I::I64Ne,
						],
					);
					s.emit_fail_if(f, "not_a_list");
					s.emit_code_or_error(f, bracket, code, BRACKET_CODE, UNKNOWN_BRACKET);
					Self::emit_list(f, &kind_of(items));
					Self::emit_list(
						f,
						&[
							I::I64Const(!(KIND_MASK << KIND_BITS)),
							I::I64And,
							I::LocalGet(code),
							I::I64Const(KIND_BITS),
							I::I64Shl,
							I::I64Or,
							I::LocalGet(items),
							I::StructGet {
								struct_type_index: node_type,
								field_index: 1,
							},
							I::LocalGet(items),
							I::StructGet {
								struct_type_index: node_type,
								field_index: 2,
							},
							I::StructNew(node_type),
						],
					);
				},
			);
		}
	}

	/// `if condition and kind: return new_text(written)` for the entry whose code is in local `code`
	fn emit_part_text(
		&mut self,
		func: &mut Function,
		condition: &[Instruction],
		kind: &[Instruction],
		code: u32,
		written: &[(i64, &str)],
	) {
		Self::emit_list(func, condition);
		Self::emit_list(func, kind);
		Self::emit_list(func, &[I::I32And, I::If(BlockType::Empty)]);
		for (entry, text) in written {
			let (pointer, length) = self.allocate_string(text);
			Self::emit_list(
				func,
				&[
					I::LocalGet(code),
					I::I32Const(*entry as i32),
					I::I32Eq,
					I::If(BlockType::Empty),
					I::I32Const(pointer as i32),
					I::I32Const(length as i32),
				],
			);
			self.call(func, "new_text");
			Self::emit_list(func, &[I::Return, I::End]);
		}
		func.instruction(&I::End);
	}

	/// `name(text) -> i64`: the code of the entry written as the text, -1 for none; byte by byte against each
	fn emit_code_of_text(&mut self, name: &'static str, written: &[(i64, &str)]) {
		let node_ref = Ref(self.node_ref(false));
		let written: Vec<(i64, String)> = written.iter().map(|(code, text)| (*code, text.to_string())).collect();
		self.runtime_function(
			name,
			vec![node_ref],
			vec![ValType::I64],
			vec![ValType::I32, ValType::I32],
			|s, f| {
				let (pointer, length) = (1, 2);
				s.emit_text_field(f, 0, 0);
				f.instruction(&I::LocalSet(pointer));
				s.emit_text_field(f, 0, 1);
				f.instruction(&I::LocalSet(length));
				for (code, text) in &written {
					Self::emit_list(
						f,
						&[
							I::LocalGet(length),
							I::I32Const(text.len() as i32),
							I::I32Eq,
							I::If(BlockType::Empty),
						],
					);
					f.instruction(&I::I32Const(1));
					for (offset, byte) in text.bytes().enumerate() {
						Self::emit_list(
							f,
							&[
								I::LocalGet(pointer),
								I::I32Load8U(MemArg {
									offset: offset as u64,
									..BYTE
								}),
								I::I32Const(byte as i32),
								I::I32Eq,
								I::I32And,
							],
						);
					}
					Self::emit_list(
						f,
						&[I::If(BlockType::Empty), I::I64Const(*code), I::Return, I::End, I::End],
					);
				}
				f.instruction(&I::I64Const(-1));
			},
		);
	}

	/// The code of the text in Node local `text` by `coder`, in i64 local `code`; for none, return the Error `prefix` + the text
	fn emit_code_or_error(&mut self, func: &mut Function, text: u32, code: u32, coder: &'static str, prefix: &str) {
		func.instruction(&I::LocalGet(text));
		self.call(func, TEXT_OF);
		func.instruction(&I::LocalTee(text));
		self.call(func, coder);
		Self::emit_list(
			func,
			&[I::LocalTee(code), I::I64Const(-1), I::I64Eq, I::If(BlockType::Empty)],
		);
		let (pointer, length) = self.allocate_string(prefix);
		Self::emit_list(func, &[I::I32Const(pointer as i32), I::I32Const(length as i32)]);
		self.call(func, "new_text");
		func.instruction(&I::LocalGet(text));
		self.call(func, TEXT_CONCAT);
		self.call(func, super::text_builtins::ERROR_OF);
		Self::emit_list(func, &[I::Return, I::End]);
	}
}
