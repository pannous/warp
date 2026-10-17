//! Constructors of the compact Node, `new_int(i64) -> ref $Node` and so on (see "Serialization" in AGENTS.md).
//! Each is emitted only when the program needs it (tree shaking) and exported for the host.

use super::WasmGcEmitter;
use crate::type_kinds::{self, any_heap_type, Kind, KIND_BITS};
use wasm_encoder::*;
use Instruction as I;
use ValType::Ref;

/// Where a constructor's kind field comes from
#[derive(PartialEq)]
enum KindField {
	Plain,
	/// The kind ORed with the i64 parameter 2 above the kind bits: a key's operator, a list's bracket
	WithInfo,
}

impl WasmGcEmitter {
	/// `name(params) -> ref $Node`: the node {kind, fields…}, where `fields` push its data and value
	fn emit_node_constructor(
		&mut self,
		name: &'static str,
		kind: Kind,
		params: Vec<ValType>,
		kind_field: KindField,
		fields: Vec<Instruction<'static>>,
	) {
		if !self.should_emit_function(name) {
			return;
		}
		let node = Ref(self.node_ref(false));
		self.exported_function(name, params, vec![node], vec![], |s, func| {
			if kind_field == KindField::WithInfo {
				Self::emit_list(func, &[I::LocalGet(2), I::I64Const(KIND_BITS), I::I64Shl]);
			}
			s.emit_kind(func, kind);
			if kind_field == KindField::WithInfo {
				func.instruction(&I::I64Or);
			}
			Self::emit_list(func, &fields);
			func.instruction(&I::StructNew(s.type_manager.node_type));
		});
	}
}

impl WasmGcEmitter {
	/// `new_bool(i64) -> ref $Node`: true for anything but 0, the Int 1/0 marked as bool (type_kinds::BOOL_KIND)
	fn emit_new_bool(&mut self) {
		if !self.should_emit_function(NEW_BOOL) {
			return;
		}
		let node = Ref(self.node_ref(false));
		let (node_type, i64_box) = (self.type_manager.node_type, self.type_manager.i64_box_type);
		self.exported_function(NEW_BOOL, vec![ValType::I64], vec![node], vec![], |_, func| {
			#[rustfmt::skip]
			Self::emit_list(func, &[
				I::I64Const(type_kinds::BOOL_KIND), I::LocalGet(0), I::I64Const(0), I::I64Ne, I::I64ExtendI32U,
				I::StructNew(i64_box), I::RefNull(HeapType::Concrete(node_type)), I::StructNew(node_type),
			]);
		});
		let node_ref = Ref(self.node_ref(false));
		self.runtime_function(AS_BOOL, vec![node_ref], vec![node_ref], vec![], |s, func| {
			let field = |index: u32| I::StructGet { struct_type_index: node_type, field_index: index };
			#[rustfmt::skip]
			Self::emit_list(func, &[
				I::LocalGet(0), field(0), I::I64Const(type_kinds::KIND_MASK), I::I64And, I::I64Const(Kind::Int as i64),
				I::I64Eq, I::LocalGet(0), field(1), I::RefTestNonNull(HeapType::Concrete(i64_box)), I::I32And,
				I::If(BlockType::Result(node_ref)),
					I::LocalGet(0), field(1), I::RefCastNonNull(HeapType::Concrete(i64_box)),
					I::StructGet { struct_type_index: i64_box, field_index: 0 },
			]);
			s.call(func, NEW_BOOL);
			Self::emit_list(func, &[I::Else, I::LocalGet(0), I::End]);
		});
	}

	/// `as_code(node) -> ref $Node`: a new top node marked as code (type_kinds::CODE_MARK), its parts shared
	fn emit_as_code(&mut self) {
		if !self.should_emit_function(crate::words::AS_CODE) {
			return;
		}
		let node_type = self.type_manager.node_type;
		let node_ref = Ref(self.node_ref(false));
		self.runtime_function(crate::words::AS_CODE, vec![node_ref], vec![node_ref], vec![], |_, func| {
			let field = |index: u32| I::StructGet { struct_type_index: node_type, field_index: index };
			#[rustfmt::skip]
			Self::emit_list(func, &[
				I::LocalGet(0), field(0), I::I64Const(type_kinds::CODE_MARK), I::I64Or,
				I::LocalGet(0), field(1), I::LocalGet(0), field(2), I::StructNew(node_type),
			]);
		});
	}
}

pub const NEW_BOOL: &str = "new_bool";
/// new_type(name, body): a type declaration in code, `class P {x: int}` (Kind::TypeDef)
pub const NEW_TYPE: &str = "new_type";
/// new_text_of(text) / new_symbol_of(text): a Text or Symbol node of the $Text
pub const NEW_TEXT_OF: &str = "new_text_of";
pub const NEW_SYMBOL_OF: &str = "new_symbol_of";
/// text_from_memory(ptr, len) -> $Text: the bytes a host or a builder left in linear memory, copied
pub const TEXT_FROM_MEMORY: &str = "text_from_memory";
/// text_address($Text) -> ptr: a copy of the bytes in linear memory, for a host or C function reading by address
pub const TEXT_ADDRESS: &str = "text_address";
/// text_bytes_join($Text, $Text) -> $Text
pub const TEXT_BYTES_JOIN: &str = "text_bytes_join";
/// text_flat(node) -> $Text: the bytes of a text node, a rope flattened
pub const TEXT_FLAT: &str = "text_flat";
/// text_piece(bytes, start, length) -> $Text: a copy of a stretch of bytes
pub const TEXT_PIECE: &str = "text_piece";
/// text_bytes_equal($Text, $Text) -> i32
pub const TEXT_BYTES_EQUAL: &str = "text_bytes_equal";
/// `as_bool(node) -> ref $Node`: an Int 1/0 as the bool it stands for, any other node as it is
pub const AS_BOOL: &str = "as_bool";

/// Emit all basic Node constructors
pub fn emit_all_constructors(emitter: &mut WasmGcEmitter) {
	use Instruction::{LocalGet, RefNull, StructNew};
	use KindField::{Plain, WithInfo};
	let types = &emitter.type_manager;
	let (node_type, i64_box, f64_box) = (types.node_type, types.i64_box_type, types.f64_box_type);
	let text_ref = emitter.text_ref();
	let from_memory = I::Call(emitter.func_index(TEXT_FROM_MEMORY));
	let node = Ref(emitter.node_ref(false));
	let nullable_node = Ref(emitter.node_ref(true));
	let no_value = || RefNull(HeapType::Concrete(node_type));

	emitter.emit_node_constructor("new_empty", Kind::Empty, vec![], Plain, vec![RefNull(any_heap_type()), no_value()]);
	if emitter.int_runtime() {
		emitter.emit_new_unbounded_int();
	} else {
		emitter.emit_node_constructor(
			"new_int",
			Kind::Int,
			vec![ValType::I64],
			Plain,
			vec![LocalGet(0), StructNew(i64_box), no_value()],
		);
	}
	emitter.emit_new_bool();
	emitter.emit_as_code();
	emitter.emit_node_constructor(
		"new_float",
		Kind::Float,
		vec![ValType::F64],
		Plain,
		vec![LocalGet(0), StructNew(f64_box), no_value()],
	);
	emitter.emit_node_constructor(
		"new_codepoint",
		Kind::Codepoint,
		vec![ValType::I32],
		Plain,
		vec![LocalGet(0), I::RefI31, no_value()],
	);
	let in_memory = || vec![LocalGet(0), LocalGet(1), from_memory.clone(), no_value()];
	let int_pair = || vec![ValType::I32, ValType::I32];
	emitter.emit_node_constructor("new_text", Kind::Text, int_pair(), Plain, in_memory());
	emitter.emit_node_constructor("new_symbol", Kind::Symbol, int_pair(), Plain, in_memory());
	emitter.emit_node_constructor(NEW_TEXT_OF, Kind::Text, vec![text_ref], Plain, vec![LocalGet(0), no_value()]);
	emitter.emit_node_constructor(NEW_SYMBOL_OF, Kind::Symbol, vec![text_ref], Plain, vec![LocalGet(0), no_value()]);
	emitter.emit_node_constructor(
		"new_key",
		Kind::Key,
		vec![node, node, ValType::I64],
		WithInfo,
		vec![LocalGet(0), LocalGet(1)],
	);
	emitter.emit_node_constructor(NEW_TYPE, Kind::TypeDef, vec![node, node], Plain, vec![LocalGet(0), LocalGet(1)]);
	emitter.emit_node_constructor(
		"new_list",
		Kind::List,
		vec![nullable_node, nullable_node, ValType::I64],
		WithInfo,
		vec![LocalGet(0), LocalGet(1)],
	);
}
