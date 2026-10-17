//! Nodes read and built by warp code, as a parser written in warp must (card self-node-api, notes/self_hosting.md):
//! `n.key`, `n.value` and `n.op` of a key, `n.name` of a word or a key, `n.bracket` of a list (a field of that name
//! wins: `{name: 3}.name` is 3), `n.separator` of a list (card keep-list), and `symbol(text)`, `key(left, op, right)`,
//! `bracketed(items, bracket)`, `separated(items, separator)` to build them.
//! `type(n)`, `#n`, `n#i` and `for x in n` read the rest already. `n.line`, `n.column` and `n.comment` read where quoted
//! code was written (card data-meta): node_at(node, line, column, comment) gives the node as a $MetaNode.

use crate::node::Node;
use crate::operators::OP_CODES;
use crate::type_kinds::{Kind, KIND_BITS, KIND_MASK, SEPARATOR_MASK, SEPARATOR_SHIFT};
use crate::wasm_emitter::layout::BYTE;
use crate::wasm_emitter::runtime_dependencies::{any, Dependency};
use crate::wasm_emitter::text::text_builtins::{ERROR_OF, TEXT_CONCAT, TEXT_OF};
use crate::wasm_emitter::WasmGcEmitter;
use crate::words::SYMBOL_WORD;
use wasm_encoder::*;
use Instruction as I;
use ValType::Ref;

/// node_part(node, part) -> the part (NODE_PARTS' index) of a node, null when it has none
pub const NODE_PART: &str = "node_part";
/// The parts node_part reads, by the field name that reads each
pub const NODE_PARTS: [&str; 9] = ["key", "value", "op", "name", "bracket", "separator", "line", "column", "comment"];
const KEY_PART: i32 = 0;
const VALUE_PART: i32 = 1;
const OP_PART: i32 = 2;
const NAME_PART: i32 = 3;
const BRACKET_PART: i32 = 4;
const SEPARATOR_PART: i32 = 5;
/// The parts only a $MetaNode has: its line, its column and its comment (fields 3, 4 and 5)
const LINE_PART: i32 = 6;
const META_FIELDS: [(i32, u32); 3] = [(LINE_PART, 3), (LINE_PART + 1, 4), (LINE_PART + 2, 5)];
/// node_at(node, line, column, comment) -> the node with its source position and comment (or null), a $MetaNode
pub const NODE_AT: &str = "node_at";
/// operator_code(text) -> i64: the code of the operator written so (operators::OP_CODES), -1 for none
const OPERATOR_CODE: &str = "operator_code";
/// bracket_code(text) -> i64: the bracket info of the opening bracket written so, -1 for none
const BRACKET_CODE: &str = "bracket_code";
/// separator_code(text) -> i64: the code of the separator written so (wasm_emitter::SEPARATORS), -1 for none
const SEPARATOR_CODE: &str = "separator_code";
const NEW_KEY_OF: &str = "new_key_of";
const NEW_BRACKETED: &str = "new_bracketed";
const NEW_SEPARATED: &str = "new_separated";
const KEY: &str = "key";
const BRACKETED: &str = "bracketed";
const SEPARATED: &str = "separated";
/// `key_of(n)`, `value_of(n)`, …: the node part itself, which a field of that name never shadows (card
/// node-part-clash: `.key` of the code `key = 3` is 3), ø for a part the node has not
const PART_READERS: [(&str, i32); 6] = [
	("key_of", KEY_PART),
	("value_of", VALUE_PART),
	("op_of", OP_PART),
	("name_of", NAME_PART),
	("bracket_of", BRACKET_PART),
	("separator_of", SEPARATOR_PART),
];
/// `positioned(node, line, column)`: the node at that source position, as `n.line` and `n.column` read it (card
/// carries-line: a pass written in warp reports where Rust's does)
pub const POSITIONED: &str = "positioned";
/// name, number of arguments, result kind (a part read or a positioned node is a Node of any kind)
pub const NODE_BUILTINS: [(&str, usize, Kind); 11] = [
	(SYMBOL_WORD, 1, Kind::Symbol),
	(KEY, 3, Kind::Key),
	(BRACKETED, 2, Kind::List),
	(SEPARATED, 2, Kind::List),
	(PART_READERS[0].0, 1, Kind::Empty),
	(PART_READERS[1].0, 1, Kind::Empty),
	(PART_READERS[2].0, 1, Kind::Empty),
	(PART_READERS[3].0, 1, Kind::Empty),
	(PART_READERS[4].0, 1, Kind::Empty),
	(PART_READERS[5].0, 1, Kind::Empty),
	(POSITIONED, 3, Kind::Empty),
];
/// The opening brackets by their bracket info in a list's kind field (Curly 0, Square 1, Round 2, Less 3, None 5)
const BRACKETS: [(&str, i64); 5] = [("{", 0), ("[", 1), ("(", 2), ("<", 3), ("", 5)];
/// The bracket info of a round group `(…)`
const ROUND_INFO: i64 = BRACKETS[2].1;
const UNKNOWN_OPERATOR: &str = "unknown operator: ";
const UNKNOWN_BRACKET: &str = "unknown bracket: ";
const UNKNOWN_SEPARATOR: &str = "unknown separator: ";

/// The code of the op a built key is given (OP_CODES' index), the highest when it is no constant
fn built_operator_code(op: &Node) -> i64 {
	let written = |text: String| OP_CODES.iter().position(|code| code.as_str() == text);
	op.text_value().and_then(written).unwrap_or(OP_CODES.len() - 1) as i64
}

/// The part a field name reads when the node has no field of that name
pub fn node_part(field: &str) -> Option<i32> {
	NODE_PARTS.iter().position(|part| *part == field).map(|part| part as i32)
}

/// A part only a key (or a word) has: its key, value, op and name, none of a list's or a map's
pub fn is_pair_part(part: i32) -> bool {
	part <= NAME_PART
}

/// Runtime functions the node words call
pub(in crate::wasm_emitter) fn dependencies() -> Vec<Dependency> {
	vec![
		any(&[NODE_PART], &["new_text", "new_empty"]),
		any(
			&[NEW_KEY_OF, NEW_BRACKETED, NEW_SEPARATED],
			&[OPERATOR_CODE, BRACKET_CODE, SEPARATOR_CODE, TEXT_OF, TEXT_CONCAT, ERROR_OF, "new_key", "new_text"],
		),
	]
}

impl WasmGcEmitter {
	/// `symbol(text)`, `key(left, op, right)`, `bracketed(items, bracket)`, `separated(items, separator)`,
	/// `positioned(node, line, column)`
	pub(in crate::wasm_emitter) fn emit_node_builtin(&mut self, func: &mut Function, name: &str, arguments: &[Node]) {
		if let (POSITIONED, [node, line, column]) = (name, arguments) {
			self.emit_node_instructions(func, node);
			func.instruction(&I::RefAsNonNull);
			self.emit_numeric_value(func, line);
			self.emit_numeric_value(func, column);
			self.emit_node_null(func);
			self.emit_call(func, NODE_AT);
			return;
		}
		for argument in arguments {
			self.emit_node_instructions(func, argument);
			func.instruction(&I::RefAsNonNull);
		}
		match name {
			SYMBOL_WORD => {
				self.emit_call(func, TEXT_OF);
				let text = self.node_scratch();
				func.instruction(&I::LocalTee(text));
				self.emit_call(func, "get_text_ptr");
				func.instruction(&I::LocalGet(text));
				self.emit_call(func, "get_text_len");
				self.emit_call(func, "new_symbol");
			}
			KEY => {
				// list_text writes the op as written: the op given, any when only the run time knows it
				let code = arguments.get(1).map_or(0, built_operator_code);
				self.discovered_needs.insert(crate::wasm_emitter::Need::KeyOperator(code));
				self.emit_call(func, NEW_KEY_OF)
			}
			SEPARATED => self.emit_call(func, NEW_SEPARATED),
			BRACKETED => self.emit_call(func, NEW_BRACKETED),
			reader => {
				let Some((_, part)) = PART_READERS.iter().find(|(word, _)| *word == reader) else {
					unreachable!("NODE_BUILTINS names {reader}");
				};
				let read = self.node_scratch();
				func.instruction(&I::I32Const(*part));
				self.emit_call(func, NODE_PART);
				#[rustfmt::skip]
				Self::emit_list(func, &[
					I::LocalTee(read), I::RefIsNull, I::If(BlockType::Result(Ref(self.node_ref(false)))),
				]);
				self.emit_call(func, "new_empty");
				Self::emit_list(func, &[I::Else, I::LocalGet(read), I::RefAsNonNull, I::End]);
			}
		}
	}

	pub(in crate::wasm_emitter) fn emit_node_api(&mut self) {
		let node_ref = Ref(self.node_ref(false));
		let nullable_node_ref = Ref(self.node_ref(true));
		let node_type = self.type_manager.node_type;
		let kind_of = |node: u32| [I::LocalGet(node), I::StructGet { struct_type_index: node_type, field_index: 0 }];

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
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(node), I::RefIsNull, I::If(BlockType::Empty),
							I::RefNull(HeapType::Concrete(node_type)), I::Return,
						I::End,
					]);
					s.emit_meta_part(f, node, part);
					// `(a * 2)` in quoted code: a group of one item has its item's key, value, op and name (card data-op)
					Self::emit_list(
						f,
						&[I::Loop(BlockType::Empty), I::LocalGet(part), I::I32Const(BRACKET_PART), I::I32LtS],
					);
					Self::emit_list(f, &kind_of(node));
					f.instruction(&I::LocalSet(kind));
					Self::emit_list(f, &is_kind(kind, Kind::List));
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::I32And, I::LocalGet(kind), I::I64Const(KIND_BITS), I::I64ShrU, I::I64Const(0xFF), I::I64And,
						I::I64Const(ROUND_INFO), I::I64Eq, I::I32And, I::LocalGet(node),
						I::StructGet { struct_type_index: node_type, field_index: 2 }, I::RefIsNull, I::I32And,
						I::If(BlockType::Empty),
							I::LocalGet(node), I::StructGet { struct_type_index: node_type, field_index: 1 },
							I::RefCastNullable(HeapType::Concrete(node_type)), I::LocalSet(node),
							I::Br(1),
						I::End,
						I::End,
					]);
					Self::emit_list(f, &kind_of(node));
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalTee(kind), I::I64Const(KIND_BITS), I::I64ShrU, I::I32WrapI64, I::I32Const(0xFF),
						I::I32And, I::LocalSet(code),
					]);
					let part_is = |wanted: i32| [I::LocalGet(part), I::I32Const(wanted), I::I32Eq];
					// a key's own parts
					for (wanted, field_index) in [(KEY_PART, 1), (VALUE_PART, 2)] {
						Self::emit_list(f, &part_is(wanted));
						Self::emit_list(f, &is_kind(kind, Kind::Key));
						#[rustfmt::skip]
						Self::emit_list(f, &[
							I::I32And, I::If(BlockType::Empty),
								I::LocalGet(node), I::StructGet { struct_type_index: node_type, field_index },
						]);
						if field_index == 1 {
							f.instruction(&I::RefCastNullable(HeapType::Concrete(node_type)));
						}
						Self::emit_list(f, &[I::Return, I::End]);
					}
					let written: Vec<(i64, &str)> =
						OP_CODES.iter().enumerate().map(|(code, op)| (code as i64, op.as_str())).collect();
					s.emit_part_text(f, &part_is(OP_PART), &is_kind(kind, Kind::Key), code, &written);
					s.emit_part_text(
						f,
						&part_is(BRACKET_PART),
						&is_kind(kind, Kind::List),
						code,
						&BRACKETS.map(|(bracket, info)| (info, bracket)),
					);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(kind), I::I64Const(SEPARATOR_MASK), I::I64And, I::I64Const(SEPARATOR_SHIFT),
						I::I64ShrU, I::I32WrapI64, I::LocalSet(code),
					]);
					let separators = super::super::separator_texts();
					let separators: Vec<(i64, &str)> =
						separators.iter().map(|(code, text)| (*code, text.as_str())).collect();
					s.emit_part_text(f, &part_is(SEPARATOR_PART), &is_kind(kind, Kind::List), code, &separators);
					// a node that is no group has no bracket and no separator: ø, so a walk tests groups without `try`
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(part), I::I32Const(BRACKET_PART), I::I32Eq, I::LocalGet(part),
						I::I32Const(SEPARATOR_PART), I::I32Eq, I::I32Or, I::If(BlockType::Empty),
					]);
					s.call(f, "new_empty");
					Self::emit_list(f, &[I::Return, I::End]);
					// a word's name, or the name of a key's key (`x` of `x: 3`, of `x.y: 3` too)
					Self::emit_list(f, &part_is(NAME_PART));
					Self::emit_list(f, &[I::If(BlockType::Empty), I::Loop(BlockType::Empty)]);
					Self::emit_list(f, &is_kind(kind, Kind::Key));
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::If(BlockType::Empty),
							I::LocalGet(node), I::StructGet { struct_type_index: node_type, field_index: 1 },
							I::RefCastNonNull(HeapType::Concrete(node_type)), I::LocalTee(node),
							I::StructGet { struct_type_index: node_type, field_index: 0 }, I::LocalSet(kind),
							I::Br(1),
						I::End,
						I::End,
					]);
					Self::emit_list(f, &is_kind(kind, Kind::Symbol));
					Self::emit_list(f, &is_kind(kind, Kind::Text));
					Self::emit_list(f, &[I::I32Or, I::If(BlockType::Empty)]);
					s.emit_text_field(f, node, 0);
					s.emit_text_field(f, node, 1);
					s.call(f, "new_text");
					Self::emit_list(f, &[I::Return, I::End, I::End, I::RefNull(HeapType::Concrete(node_type))]);
				},
			);
		}

		// the type is there exactly when node_at is (emit_gc_types)
		if let (true, Some(meta_node_type)) = (self.should_emit_function(NODE_AT), self.type_manager.meta_node_type) {
			let field = |field_index| I::StructGet { struct_type_index: node_type, field_index };
			self.runtime_function(
				NODE_AT,
				vec![node_ref, ValType::I64, ValType::I64, nullable_node_ref],
				vec![node_ref],
				vec![],
				|_, f| {
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(0), field(0), I::LocalGet(0), field(1), I::LocalGet(0), field(2), I::LocalGet(1),
						I::LocalGet(2), I::LocalGet(3), I::StructNew(meta_node_type),
					]);
				},
			);
			// the host builds a run-time parse's positions with it (tasks.rs TaskValue::Positioned)
			self.export_runtime_function(NODE_AT);
		}

		if self.should_emit_function(NEW_KEY_OF) {
			self.emit_code_of_text(
				OPERATOR_CODE,
				&OP_CODES.iter().enumerate().map(|(code, op)| (code as i64, op.as_str())).collect::<Vec<_>>(),
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

		let brackets = BRACKETS.map(|(bracket, info)| (info, bracket.to_string()));
		self.emit_relisted(NEW_BRACKETED, BRACKET_CODE, &brackets, KIND_BITS, KIND_MASK << KIND_BITS, UNKNOWN_BRACKET);
		let separators = super::super::separator_texts();
		self.emit_relisted(
			NEW_SEPARATED,
			SEPARATOR_CODE,
			&separators,
			SEPARATOR_SHIFT,
			SEPARATOR_MASK,
			UNKNOWN_SEPARATOR,
		);
	}

	/// In node_part: a $MetaNode's line or column (an Int) or comment (a Text or null), null from any other node
	fn emit_meta_part(&mut self, f: &mut Function, node: u32, part: u32) {
		Self::emit_list(f, &[I::LocalGet(part), I::I32Const(LINE_PART), I::I32GeS, I::If(BlockType::Empty)]);
		if let Some(meta_node_type) = self.type_manager.meta_node_type {
			self.emit_meta_fields(f, node, part, meta_node_type);
		}
		let node_type = self.type_manager.node_type;
		Self::emit_list(f, &[I::RefNull(HeapType::Concrete(node_type)), I::Return, I::End]);
	}

	fn emit_meta_fields(&mut self, f: &mut Function, node: u32, part: u32, meta_node_type: u32) {
		let meta_field = |field_index| {
			[
				I::LocalGet(node),
				I::RefCastNonNull(HeapType::Concrete(meta_node_type)),
				I::StructGet { struct_type_index: meta_node_type, field_index },
			]
		};
		let is_meta = [I::LocalGet(node), I::RefTestNullable(HeapType::Concrete(meta_node_type))];
		Self::emit_list(f, &is_meta);
		f.instruction(&I::If(BlockType::Empty));
		for (wanted, field_index) in META_FIELDS {
			Self::emit_list(f, &[I::LocalGet(part), I::I32Const(wanted), I::I32Eq, I::If(BlockType::Empty)]);
			Self::emit_list(f, &meta_field(field_index));
			if field_index != META_FIELDS[2].1 {
				self.call(f, "new_int");
			}
			Self::emit_list(f, &[I::Return, I::End]);
		}
		f.instruction(&I::End);
	}

	/// `name(items, text)`: the list of the same items with the head cell's kind bits under `mask` set to the code
	/// (shifted by `shift`) of the entry written as the text, an Error `prefix` + the text for none
	fn emit_relisted(
		&mut self,
		name: &'static str,
		coder: &'static str,
		written: &[(i64, String)],
		shift: i64,
		mask: i64,
		prefix: &'static str,
	) {
		if !self.should_emit_function(name) {
			return;
		}
		let written: Vec<(i64, &str)> = written.iter().map(|(code, text)| (*code, text.as_str())).collect();
		self.emit_code_of_text(coder, &written);
		let node_ref = Ref(self.node_ref(false));
		let node_type = self.type_manager.node_type;
		let kind_of = |node: u32| [I::LocalGet(node), I::StructGet { struct_type_index: node_type, field_index: 0 }];
		let nullable_node_ref = Ref(self.node_ref(true));
		let (items, text) = (0, 1);
		let relisted = self.next_func_idx;
		self.runtime_function(
			name,
			vec![node_ref, node_ref],
			vec![node_ref],
			vec![ValType::I64, nullable_node_ref],
			|s, f| {
				let (code, rest) = (2, 3);
				s.emit_code_or_error(f, text, code, coder, prefix);
				// no items is the empty list ø, whatever its brackets, as `()` parses
				Self::emit_list(f, &kind_of(items));
				Self::emit_kind_test(f, Kind::Empty as i64, I::I64Eq);
				Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(items), I::Return, I::End]);
				Self::emit_list(f, &kind_of(items));
				Self::emit_kind_test(f, Kind::List as i64, I::I64Ne);
				s.emit_fail_if(f, "not_a_list");
				Self::emit_list(f, &kind_of(items));
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::I64Const(!mask), I::I64And, I::LocalGet(code), I::I64Const(shift), I::I64Shl, I::I64Or,
					I::LocalGet(items), I::StructGet { struct_type_index: node_type, field_index: 1 },
				]);
				// the rest of the list is relisted too: a written list carries its info in every cell
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(items), I::StructGet { struct_type_index: node_type, field_index: 2 },
					I::LocalTee(rest), I::RefIsNull, I::If(BlockType::Result(nullable_node_ref)),
						I::LocalGet(rest),
					I::Else,
						I::LocalGet(rest), I::RefAsNonNull, I::LocalGet(text), I::Call(relisted),
					I::End,
					I::StructNew(node_type),
				]);
			},
		);
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
			#[rustfmt::skip]
			Self::emit_list(func, &[
				I::LocalGet(code), I::I32Const(*entry as i32), I::I32Eq, I::If(BlockType::Empty),
					I::I32Const(pointer as i32), I::I32Const(length as i32),
			]);
			self.call(func, "new_text");
			Self::emit_list(func, &[I::Return, I::End]);
		}
		func.instruction(&I::End);
	}

	/// `name(text) -> i64`: the code of the entry written as the text, -1 for none; byte by byte against each
	fn emit_code_of_text(&mut self, name: &'static str, written: &[(i64, &str)]) {
		let node_ref = Ref(self.node_ref(false));
		let written: Vec<(i64, String)> = written.iter().map(|(code, text)| (*code, text.to_string())).collect();
		self.runtime_function(name, vec![node_ref], vec![ValType::I64], vec![ValType::I32, ValType::I32], |s, f| {
			let (pointer, length) = (1, 2);
			s.emit_text_field(f, 0, 0);
			f.instruction(&I::LocalSet(pointer));
			s.emit_text_field(f, 0, 1);
			f.instruction(&I::LocalSet(length));
			for (code, text) in &written {
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(length), I::I32Const(text.len() as i32), I::I32Eq, I::If(BlockType::Empty),
						I::I32Const(1),
				]);
				for (offset, byte) in text.bytes().enumerate() {
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(pointer), I::I32Load8U(MemArg { offset: offset as u64, ..BYTE }),
						I::I32Const(byte as i32), I::I32Eq, I::I32And,
					]);
				}
				Self::emit_list(f, &[I::If(BlockType::Empty), I::I64Const(*code), I::Return, I::End, I::End]);
			}
			f.instruction(&I::I64Const(-1));
		});
	}

	/// The code of the text in Node local `text` by `coder`, in i64 local `code`; for none, return the Error `prefix` + the text
	fn emit_code_or_error(&mut self, func: &mut Function, text: u32, code: u32, coder: &'static str, prefix: &str) {
		func.instruction(&I::LocalGet(text));
		self.call(func, TEXT_OF);
		func.instruction(&I::LocalTee(text));
		self.call(func, coder);
		Self::emit_list(func, &[I::LocalTee(code), I::I64Const(-1), I::I64Eq, I::If(BlockType::Empty)]);
		let (pointer, length) = self.allocate_string(prefix);
		Self::emit_list(func, &[I::I32Const(pointer as i32), I::I32Const(length as i32)]);
		self.call(func, "new_text");
		func.instruction(&I::LocalGet(text));
		self.call(func, TEXT_CONCAT);
		self.call(func, super::super::text::text_builtins::ERROR_OF);
		Self::emit_list(func, &[I::Return, I::End]);
	}
}
