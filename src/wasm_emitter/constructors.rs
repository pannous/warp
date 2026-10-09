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
	fn emit_node_constructor(&mut self, name: &'static str, kind: Kind, params: Vec<ValType>, kind_field: KindField, fields: Vec<Instruction<'static>>) {
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
			Self::emit_list(func, &[
				I::I64Const(type_kinds::BOOL_KIND),
				I::LocalGet(0), I::I64Const(0), I::I64Ne, I::I64ExtendI32U, I::StructNew(i64_box),
				I::RefNull(HeapType::Concrete(node_type)), I::StructNew(node_type),
			]);
		});
		let node_ref = Ref(self.node_ref(false));
		self.runtime_function(AS_BOOL, vec![node_ref], vec![node_ref], vec![], |s, func| {
			let field = |index: u32| I::StructGet { struct_type_index: node_type, field_index: index };
			Self::emit_list(func, &[I::LocalGet(0), field(0), I::I64Const(type_kinds::KIND_MASK), I::I64And, I::I64Const(Kind::Int as i64), I::I64Eq]);
			Self::emit_list(func, &[I::LocalGet(0), field(1), I::RefTestNonNull(HeapType::Concrete(i64_box)), I::I32And, I::If(BlockType::Result(node_ref))]);
			Self::emit_list(func, &[I::LocalGet(0), field(1), I::RefCastNonNull(HeapType::Concrete(i64_box)), I::StructGet { struct_type_index: i64_box, field_index: 0 }]);
			s.call(func, NEW_BOOL);
			Self::emit_list(func, &[I::Else, I::LocalGet(0), I::End]);
		});
	}
}

pub const NEW_BOOL: &str = "new_bool";
/// `as_bool(node) -> ref $Node`: an Int 1/0 as the bool it stands for, any other node as it is
pub const AS_BOOL: &str = "as_bool";

/// Emit all basic Node constructors
pub fn emit_all_constructors(emitter: &mut WasmGcEmitter) {
	use Instruction::{LocalGet, RefNull, StructNew};
	use KindField::{Plain, WithInfo};
	let types = &emitter.type_manager;
	let (node_type, i64_box, f64_box, string) = (types.node_type, types.i64_box_type, types.f64_box_type, types.string_type);
	let node = Ref(emitter.node_ref(false));
	let nullable_node = Ref(emitter.node_ref(true));
	let no_value = || RefNull(HeapType::Concrete(node_type));

	emitter.emit_node_constructor("new_empty", Kind::Empty, vec![], Plain, vec![RefNull(any_heap_type()), no_value()]);
	if emitter.int_runtime() {
		emitter.emit_new_unbounded_int();
	} else {
		emitter.emit_node_constructor("new_int", Kind::Int, vec![ValType::I64], Plain, vec![LocalGet(0), StructNew(i64_box), no_value()]);
	}
	emitter.emit_new_bool();
	emitter.emit_node_constructor("new_float", Kind::Float, vec![ValType::F64], Plain, vec![LocalGet(0), StructNew(f64_box), no_value()]);
	emitter.emit_node_constructor("new_codepoint", Kind::Codepoint, vec![ValType::I32], Plain, vec![LocalGet(0), I::RefI31, no_value()]);
	let text = || vec![LocalGet(0), LocalGet(1), StructNew(string), no_value()];
	emitter.emit_node_constructor("new_text", Kind::Text, vec![ValType::I32, ValType::I32], Plain, text());
	emitter.emit_node_constructor("new_symbol", Kind::Symbol, vec![ValType::I32, ValType::I32], Plain, text());
	emitter.emit_node_constructor("new_key", Kind::Key, vec![node, node, ValType::I64], WithInfo, vec![LocalGet(0), LocalGet(1)]);
	emitter.emit_node_constructor("new_type", Kind::TypeDef, vec![node, node], Plain, vec![LocalGet(0), LocalGet(1)]);
	emitter.emit_node_constructor("new_list", Kind::List, vec![nullable_node, nullable_node, ValType::I64], WithInfo, vec![LocalGet(0), LocalGet(1)]);
}
