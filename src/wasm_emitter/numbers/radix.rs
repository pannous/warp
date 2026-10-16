//! card hex-printing (user, 2026-10-11): `255 as hex` and a byte are ints that show as 0xFF; card binary-int (user,
//! 2026-10-12): `5 as binary` (alias `bin`) shows as 0b101. They compute as plain ints; where such a value becomes a
//! Node it is an Int marked with its radix's info above the kind bits (type_kinds), which list_join writes in that
//! radix and the readers read back as the Int showing its literal
use super::super::*;
use crate::type_kinds;
use crate::wasm_emitter::functions::late_calls::LateCall;

/// A radix an Int may show in: its type word, its mark above the kind bits, its literal prefix, the bits per digit, its
/// runtime function `as_<word>(node) -> ref $Node` (an Int marked, any other node as it is) and lib/prelude.warp's
/// `<word>_text(n: int)`, the text 0xFF, which list_join calls (by its prelude name)
#[derive(PartialEq)]
pub struct Radix {
	pub word: &'static str,
	pub info: i64,
	prefix: &'static str,
	digit_bits: i64,
	pub as_function: &'static str,
	text_word: &'static str,
	pub text_function: &'static str,
}

pub const HEX: Radix = Radix {
	word: "hex",
	info: type_kinds::HEX_INFO,
	prefix: "0x",
	digit_bits: 4,
	as_function: "as_hex",
	text_word: "hex_text",
	text_function: "prelude·hex_text",
};
pub const BINARY: Radix = Radix {
	word: "binary",
	info: type_kinds::BINARY_INFO,
	prefix: "0b",
	digit_bits: 1,
	as_function: "as_binary",
	text_word: "binary_text",
	text_function: "prelude·binary_text",
};
pub static RADIXES: [Radix; 2] = [HEX, BINARY];
/// Python's `bin`: warp says binary
const BINARY_ALIAS: &str = "bin";

impl Radix {
	/// The kind field of an Int shown in this radix
	pub const fn kind(&self) -> i64 {
		(self.info << type_kinds::KIND_BITS) | Kind::Int as i64
	}

	/// The literal an Int shown in this radix reads as: 0xFF, -0x1, 0b101
	pub fn literal(&self, value: i64) -> String {
		let sign = if value < 0 { "-" } else { "" };
		let digits = match self.digit_bits {
			1 => format!("{:b}", value.unsigned_abs()),
			_ => format!("{:X}", value.unsigned_abs()),
		};
		format!("{sign}{}{digits}", self.prefix)
	}
}

/// `hex`, `binary`, `bin`: the radix the word names
pub fn named_radix(type_name: &str) -> Option<&'static Radix> {
	match type_name.to_lowercase().as_str() {
		BINARY_ALIAS => Some(&BINARY),
		name => RADIXES.iter().find(|radix| radix.word == name),
	}
}

/// `hex`, `binary`, `byte`, `u8`, `uint8`: the radix a type's values show in
pub fn shown_radix(type_name: &str) -> Option<&'static Radix> {
	named_radix(type_name).or_else(|| {
		crate::numbers::fixed_width::fixed_width(&type_name.to_lowercase())
			.is_some_and(|width| width.high == u8::MAX as i128 && width.low == 0)
			.then_some(&HEX)
	})
}

/// The prelude words that print the radixes the statements name (`as hex`, `byte b`), for modules::resolve: the printer
/// calls them, the program does not
pub(crate) fn implied_words(statements: &[Node]) -> Vec<String> {
	let mut words = Vec::new();
	for statement in statements {
		statement.visit(&mut |node| {
			if let Some(radix) = node.symbol_name().and_then(shown_radix) {
				words.push(radix.text_word.to_string());
			}
		});
	}
	words
}

/// The radix of an Int's info bits, None for a plain Int
pub fn radix_of_info(info: i64) -> Option<&'static Radix> {
	RADIXES.iter().find(|radix| radix.info == info)
}

/// The literal an Int read back shows: 0xFF for one marked hex, None for a plain one
pub fn marked_literal(info: i64, value: i64) -> Option<String> {
	radix_of_info(info).map(|radix| radix.literal(value))
}

/// `x as hex`, `x as binary` compute as an int; the Node it becomes shows in that radix (emit_node_instructions).
/// `as bin` is `as binary` with a got-it note
pub(in crate::wasm_emitter) fn computed_type(target: &Node) -> Option<Node> {
	let name = target.name();
	named_radix(&name)?;
	if name.to_lowercase() == BINARY_ALIAS {
		crate::errors::diagnostic::note_alias(BINARY_ALIAS, BINARY.word);
	}
	Some(Node::Symbol("int".into()))
}

impl WasmGcEmitter {
	/// `x as hex`, `x as byte`, `x as binary`, a variable declared or assigned so
	pub(in crate::wasm_emitter) fn shown_radix(&self, node: &Node) -> Option<&'static Radix> {
		match node.drop_meta() {
			Node::Key(_, Op::As, target) => shown_radix(&target.name()),
			Node::Symbol(name) => self
				.scope
				.lookup(name)
				.and_then(|local| local.type_node.as_ref())
				.and_then(|type_node| shown_radix(&type_node.name())),
			_ => None,
		}
	}

	/// The radixes whose prelude text word the program loaded, each with the late call list_join prints it by
	pub(in crate::wasm_emitter) fn radix_text_calls(&mut self) -> Vec<(&'static Radix, LateCall)> {
		let node_ref = Ref(self.node_ref(false));
		let loaded: Vec<&'static Radix> =
			RADIXES.iter().filter(|radix| self.ctx.user_functions.contains_key(radix.text_function)).collect();
		loaded
			.into_iter()
			.map(|radix| (radix, self.late_call(radix.text_function, vec![ValType::I64], vec![node_ref])))
			.collect()
	}

	pub(in crate::wasm_emitter) fn emit_radix_runtimes(&mut self) {
		for radix in &RADIXES {
			self.emit_radix_runtime(radix);
		}
	}

	fn emit_radix_runtime(&mut self, radix: &Radix) {
		// a declared byte asks for as_hex only once emitted (discovered_needs): its type word is gone before the analysis
		if !self.should_emit_function(radix.as_function) {
			return;
		}
		self.emit_text_heap_global();
		let (node_type, i64_box) = (self.type_manager.node_type, self.type_manager.i64_box_type);
		let node_ref = Ref(self.node_ref(false));
		self.runtime_function(radix.as_function, vec![node_ref], vec![node_ref], vec![], |_, f| {
			let field = |index: u32| I::StructGet { struct_type_index: node_type, field_index: index };
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(0), field(0), I::I64Const(type_kinds::KIND_MASK), I::I64And, I::I64Const(Kind::Int as i64),
				I::I64Eq, I::LocalGet(0), field(1), I::RefTestNonNull(HeapType::Concrete(i64_box)), I::I32And,
				I::If(BlockType::Result(node_ref)),
					I::I64Const(radix.kind()), I::LocalGet(0), field(1), I::RefNull(HeapType::Concrete(node_type)),
					I::StructNew(node_type),
				I::Else,
					I::LocalGet(0),
				I::End,
			]);
		});
	}
}
