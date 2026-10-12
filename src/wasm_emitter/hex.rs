//! card hex-printing (user, 2026-10-11): `255 as hex` and a byte are ints that show as 0xFF. They compute as plain
//! ints; where such a value becomes a Node it is an Int marked HEX_INFO above the kind bits (type_kinds), which
//! list_join writes in hex and the readers read back as the Int showing its hex literal
use super::*;
use crate::type_kinds;
use crate::wasm_emitter::layout::BYTE;

pub const HEX_WORD: &str = "hex";
/// `as_hex(node) -> ref $Node`: an Int marked to show in hex, any other node (a big int) as it is
pub const AS_HEX: &str = "as_hex";
/// `hex_text(i64) -> ref $Node`: the text 0xFF
pub const HEX_TEXT: &str = "hex_text";
const HEX_PREFIX: &str = "0x";
const NIBBLE_BITS: i64 = 4;
const NIBBLE_MASK: i64 = 0xF;
/// From the digit 9 to the letter A in ASCII
const LETTER_DIGIT_GAP: i32 = (b'A' - b'9' - 1) as i32;

/// `hex`, `byte`, `u8`, `uint8`: a type whose values show in hex
pub fn shows_in_hex(type_name: &str) -> bool {
	let name = type_name.to_lowercase();
	name == HEX_WORD
		|| crate::fixed_width::fixed_width(&name).is_some_and(|width| width.high == u8::MAX as i128 && width.low == 0)
}

/// `x as hex` computes as an int; the Node it becomes shows in hex (emit_node_instructions)
pub(super) fn computed_type(target: &Node) -> Option<Node> {
	(target.name().to_lowercase() == HEX_WORD).then(|| Node::Symbol("int".into()))
}

/// The literal an Int shown in hex reads as: 0xFF, -0x1
pub fn hex_literal(value: i64) -> String {
	let sign = if value < 0 { "-" } else { "" };
	format!("{sign}{HEX_PREFIX}{:X}", value.unsigned_abs())
}

impl WasmGcEmitter {
	/// `x as hex`, `x as byte`, a variable declared or assigned so
	pub(super) fn shows_hex(&self, node: &Node) -> bool {
		match node.drop_meta() {
			Node::Key(_, Op::As, target) => shows_in_hex(&target.name()),
			Node::Symbol(name) => self
				.scope
				.lookup(name)
				.and_then(|local| local.type_node.as_ref())
				.is_some_and(|type_node| shows_in_hex(&type_node.name())),
			_ => false,
		}
	}

	pub(super) fn emit_hex_runtime(&mut self) {
		// a declared byte asks for as_hex only once emitted (discovered_needs): its type word is gone before the analysis
		if !self.should_emit_function(HEX_TEXT) && !self.should_emit_function(AS_HEX) {
			return;
		}
		self.emit_text_heap_global();
		let (node_type, i64_box) = (self.type_manager.node_type, self.type_manager.i64_box_type);
		let node_ref = Ref(self.node_ref(false));
		self.runtime_function(AS_HEX, vec![node_ref], vec![node_ref], vec![], |_, f| {
			let field = |index: u32| I::StructGet {
				struct_type_index: node_type,
				field_index: index,
			};
			Self::emit_list(
				f,
				&[
					I::LocalGet(0),
					field(0),
					I::I64Const(type_kinds::KIND_MASK),
					I::I64And,
					I::I64Const(Kind::Int as i64),
					I::I64Eq,
				],
			);
			Self::emit_list(
				f,
				&[
					I::LocalGet(0),
					field(1),
					I::RefTestNonNull(HeapType::Concrete(i64_box)),
					I::I32And,
					I::If(BlockType::Result(node_ref)),
				],
			);
			Self::emit_list(
				f,
				&[
					I::I64Const(type_kinds::HEX_KIND),
					I::LocalGet(0),
					field(1),
					I::RefNull(HeapType::Concrete(node_type)),
					I::StructNew(node_type),
				],
			);
			Self::emit_list(f, &[I::Else, I::LocalGet(0), I::End]);
		});
		let (i32t, i64t) = (ValType::I32, ValType::I64);
		self.runtime_function(
			HEX_TEXT,
			vec![i64t],
			vec![node_ref],
			vec![i32t, i64t, i64t, i32t, i32t, i32t, i32t],
			|s, f| {
				let (value, negative, magnitude, rest, address, length, position, nibble) = (0, 1, 2, 3, 4, 5, 6, 7);
				Self::emit_list(
					f,
					&[I::LocalGet(value), I::I64Const(0), I::I64LtS, I::LocalSet(negative)],
				);
				Self::emit_list(
					f,
					&[
						I::I64Const(0),
						I::LocalGet(value),
						I::I64Sub,
						I::LocalGet(value),
						I::LocalGet(negative),
						I::Select,
						I::LocalSet(magnitude),
					],
				);
				// length: sign, prefix and one digit, one more for each nibble above the last
				Self::emit_list(
					f,
					&[
						I::LocalGet(negative),
						I::I32Const(HEX_PREFIX.len() as i32 + 1),
						I::I32Add,
						I::LocalSet(length),
						I::LocalGet(magnitude),
						I::LocalSet(rest),
					],
				);
				Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
				Self::emit_list(
					f,
					&[
						I::LocalGet(rest),
						I::I64Const(NIBBLE_BITS),
						I::I64ShrU,
						I::LocalTee(rest),
						I::I64Eqz,
						I::BrIf(1),
					],
				);
				Self::emit_list(
					f,
					&[
						I::LocalGet(length),
						I::I32Const(1),
						I::I32Add,
						I::LocalSet(length),
						I::Br(0),
						I::End,
						I::End,
					],
				);
				s.emit_text_allocation(f, length, address);
				Self::emit_list(
					f,
					&[
						I::LocalGet(negative),
						I::If(BlockType::Empty),
						I::LocalGet(address),
						I::I32Const(b'-' as i32),
						I::I32Store8(BYTE),
						I::End,
					],
				);
				for (offset, byte) in HEX_PREFIX.bytes().enumerate() {
					Self::emit_list(
						f,
						&[
							I::LocalGet(address),
							I::LocalGet(negative),
							I::I32Add,
							I::I32Const(offset as i32),
							I::I32Add,
							I::I32Const(byte as i32),
							I::I32Store8(BYTE),
						],
					);
				}
				// the digits from the last one back to the prefix: the nibble's digit, a letter past 9
				Self::emit_list(
					f,
					&[
						I::LocalGet(length),
						I::LocalSet(position),
						I::Block(BlockType::Empty),
						I::Loop(BlockType::Empty),
					],
				);
				Self::emit_list(
					f,
					&[
						I::LocalGet(position),
						I::LocalGet(negative),
						I::I32Const(HEX_PREFIX.len() as i32),
						I::I32Add,
						I::I32LeU,
						I::BrIf(1),
					],
				);
				Self::emit_list(
					f,
					&[I::LocalGet(position), I::I32Const(1), I::I32Sub, I::LocalSet(position)],
				);
				Self::emit_list(
					f,
					&[
						I::LocalGet(magnitude),
						I::I64Const(NIBBLE_MASK),
						I::I64And,
						I::I32WrapI64,
						I::LocalSet(nibble),
					],
				);
				Self::emit_list(f, &[I::LocalGet(address), I::LocalGet(position), I::I32Add]);
				Self::emit_list(
					f,
					&[
						I::LocalGet(nibble),
						I::I32Const(b'0' as i32),
						I::I32Add,
						I::LocalGet(nibble),
						I::I32Const(9),
						I::I32GtU,
						I::I32Const(LETTER_DIGIT_GAP),
						I::I32Mul,
						I::I32Add,
					],
				);
				Self::emit_list(
					f,
					&[
						I::I32Store8(BYTE),
						I::LocalGet(magnitude),
						I::I64Const(NIBBLE_BITS),
						I::I64ShrU,
						I::LocalSet(magnitude),
						I::Br(0),
						I::End,
						I::End,
					],
				);
				Self::emit_list(f, &[I::LocalGet(address), I::LocalGet(length)]);
				s.call(f, "new_text");
			},
		);
	}
}
