//! Constructors of the compact Node, `new_int(i64) -> ref $Node` and so on (see "Serialization" in AGENTS.md).
//! Each is emitted only when the program needs it (tree shaking) and exported for the host.

use super::WasmGcEmitter;
use crate::type_kinds::{any_heap_type, Kind, KIND_BITS};
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
	emitter.emit_node_constructor("new_float", Kind::Float, vec![ValType::F64], Plain, vec![LocalGet(0), StructNew(f64_box), no_value()]);
	emitter.emit_node_constructor("new_codepoint", Kind::Codepoint, vec![ValType::I32], Plain, vec![LocalGet(0), I::RefI31, no_value()]);
	let text = || vec![LocalGet(0), LocalGet(1), StructNew(string), no_value()];
	emitter.emit_node_constructor("new_text", Kind::Text, vec![ValType::I32, ValType::I32], Plain, text());
	emitter.emit_node_constructor("new_symbol", Kind::Symbol, vec![ValType::I32, ValType::I32], Plain, text());
	emitter.emit_node_constructor("new_key", Kind::Key, vec![node, node, ValType::I64], WithInfo, vec![LocalGet(0), LocalGet(1)]);
	emitter.emit_node_constructor("new_type", Kind::TypeDef, vec![node, node], Plain, vec![LocalGet(0), LocalGet(1)]);
	emitter.emit_node_constructor("new_list", Kind::List, vec![nullable_node, nullable_node, ValType::I64], WithInfo, vec![LocalGet(0), LocalGet(1)]);
}
