//! Cells: one mutable value that several functions share (lowering/nonlocal_cells.rs, `nonlocal y` changed by a nested
//! function). A cell is a $Node of Kind::Data whose data is a $node_array of one element:
//!   cell_new(v) → the cell holding v, cell_get(c) → its value, cell_set(c, v) → stores v and gives it back.
//! Everything is in the module itself, so the browser build runs it as it is.

use super::WasmGcEmitter;
use crate::node::Node;
use crate::type_kinds::Kind;
use wasm_encoder::*;
use Instruction as I;
use ValType::Ref;

pub const CELL_NEW: &str = "cell_new";
pub const CELL_GET: &str = "cell_get";
pub const CELL_SET: &str = "cell_set";
pub const CELL_WORDS: [&str; 3] = [CELL_NEW, CELL_GET, CELL_SET];
const NODE_DATA_FIELD: u32 = 1;

impl WasmGcEmitter {
	pub(super) fn emit_cells(&mut self) {
		if !CELL_WORDS.iter().any(|word| self.should_emit_function(word)) {
			return;
		}
		let (node, array) = (self.type_manager.node_type, self.type_manager.node_array_type);
		let node_ref = Ref(self.node_ref(false));
		let the_array = [I::LocalGet(0), I::StructGet { struct_type_index: node, field_index: NODE_DATA_FIELD }, I::RefCastNonNull(HeapType::Concrete(array))];
		self.runtime_function(CELL_NEW, vec![node_ref], vec![node_ref], vec![], |s, f| {
			s.emit_kind(f, Kind::Data);
			Self::emit_list(f, &[I::LocalGet(0), I::ArrayNewFixed { array_type_index: array, array_size: 1 }, I::RefNull(HeapType::Concrete(node)), I::StructNew(node)]);
		});
		self.runtime_function(CELL_GET, vec![node_ref], vec![node_ref], vec![], |_, f| {
			Self::emit_list(f, &the_array);
			Self::emit_list(f, &[I::I32Const(0), I::ArrayGet(array), I::RefAsNonNull]);
		});
		self.runtime_function(CELL_SET, vec![node_ref, node_ref], vec![node_ref], vec![], |_, f| {
			Self::emit_list(f, &the_array);
			Self::emit_list(f, &[I::I32Const(0), I::LocalGet(1), I::ArraySet(array), I::LocalGet(1)]);
		});
	}

	/// `cell_new(v)`, `cell_get(c)`, `cell_set(c, v)`: the arguments as Nodes and the call
	pub(super) fn emit_cell_call(&mut self, func: &mut Function, items: &[Node]) -> bool {
		let Some(Node::Symbol(name)) = items.first().map(Node::drop_meta) else { return false };
		let Some(word) = CELL_WORDS.iter().copied().find(|word| word == name) else { return false };
		for argument in &items[1..] {
			self.emit_node_instructions(func, argument);
			func.instruction(&I::RefAsNonNull); // ø is a node too, never null
		}
		self.emit_call(func, word);
		true
	}
}
