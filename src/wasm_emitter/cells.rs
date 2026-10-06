//! Cells: one mutable value that several functions share (lowering/nonlocal_cells.rs, `nonlocal y` changed by a nested
//! function). A cell is a $Node of Kind::Data whose data is a $node_array of one element:
//!   cell_new(v) → the cell holding v, cell_get(c) → its value, cell_set(c, v) → stores v and gives it back.
//! A signal (lowering/signal_values.rs) is a cell with a second slot, the list of its listener closures:
//!   signal_new(v) → the signal holding v and no listeners, cell_get / cell_set its value,
//!   signal_listeners(s) → the listener list, signal_listeners_set(s, list) → stores it and gives it back.
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
pub const SIGNAL_NEW: &str = "signal_new";
pub const SIGNAL_LISTENERS: &str = "signal_listeners";
pub const SIGNAL_LISTENERS_SET: &str = "signal_listeners_set";
pub const CELL_WORDS: [&str; 6] = [CELL_NEW, CELL_GET, CELL_SET, SIGNAL_NEW, SIGNAL_LISTENERS, SIGNAL_LISTENERS_SET];
/// The words that make a cell
pub const MAKING_WORDS: [&str; 2] = [CELL_NEW, SIGNAL_NEW];
const NODE_DATA_FIELD: u32 = 1;
const VALUE_SLOT: i32 = 0;
const LISTENERS_SLOT: i32 = 1;

/// `cell_get` and the other words that take a cell first
pub fn is_cell_word(word: &Node) -> bool {
	matches!(word.drop_meta(), Node::Symbol(name) if CELL_WORDS.contains(&name.as_str()))
}

impl WasmGcEmitter {
	pub(super) fn emit_cells(&mut self) {
		if !CELL_WORDS.iter().any(|word| self.should_emit_function(word)) {
			return;
		}
		let (node, array) = (self.type_manager.node_type, self.type_manager.node_array_type);
		let node_ref = Ref(self.node_ref(false));
		let the_array = [I::LocalGet(0), I::StructGet { struct_type_index: node, field_index: NODE_DATA_FIELD }, I::RefCastNonNull(HeapType::Concrete(array))];
		for (word, slots) in [(CELL_NEW, 1), (SIGNAL_NEW, 2)] {
			self.runtime_function(word, vec![node_ref; slots as usize], vec![node_ref], vec![], |s, f| {
				s.emit_kind(f, Kind::Data);
				let values = (0..slots).map(I::LocalGet);
				Self::emit_list(f, &values.chain([I::ArrayNewFixed { array_type_index: array, array_size: slots }, I::RefNull(HeapType::Concrete(node)), I::StructNew(node)]).collect::<Vec<_>>());
			});
		}
		for (get, set, slot) in [(CELL_GET, CELL_SET, VALUE_SLOT), (SIGNAL_LISTENERS, SIGNAL_LISTENERS_SET, LISTENERS_SLOT)] {
			self.runtime_function(get, vec![node_ref], vec![node_ref], vec![], |_, f| {
				Self::emit_list(f, &the_array);
				Self::emit_list(f, &[I::I32Const(slot), I::ArrayGet(array), I::RefAsNonNull]);
			});
			self.runtime_function(set, vec![node_ref, node_ref], vec![node_ref], vec![], |_, f| {
				Self::emit_list(f, &the_array);
				Self::emit_list(f, &[I::I32Const(slot), I::LocalGet(1), I::ArraySet(array), I::LocalGet(1)]);
			});
		}
	}

	/// `cell_new(v)`, `cell_get(c)`, `cell_set(c, v)`: the arguments as Nodes and the call
	pub(super) fn emit_cell_call(&mut self, func: &mut Function, items: &[Node]) -> bool {
		let Some(Node::Symbol(name)) = items.first().map(Node::drop_meta) else { return false };
		let Some(word) = CELL_WORDS.iter().copied().find(|word| word == name) else { return false };
		let no_listeners = (word == SIGNAL_NEW).then_some(Node::Empty);
		for argument in items[1..].iter().chain(no_listeners.as_ref()) {
			self.emit_node_instructions(func, argument);
			func.instruction(&I::RefAsNonNull); // ø is a node too, never null
		}
		self.emit_call(func, word);
		true
	}
}
