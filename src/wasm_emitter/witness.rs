//! The runtime witness table of Comparable (notes/traits.md): where the static type of two instances is unknown,
//! `node_order` asks the funcref global `compare·witness` to order them. It holds the dispatcher `instance_compare`,
//! which matches the type name of the first instance and calls that type's witness `compare·T` (traits.rs).
//! The dispatcher is compiled after the user functions, whose indices it needs, and main installs it before anything
//! runs; runtime functions compiled earlier reach it through the global with call_ref.

use super::WasmGcEmitter;
use crate::traits::{witness_name, witness_type, COMPARE};
use crate::type_kinds::Kind;
use wasm_encoder::*;
use Instruction as I;
use ValType::Ref;

pub(super) const INSTANCE_COMPARE: &str = "instance_compare";
const WITNESS_GLOBAL: &str = "compare·witness";

#[derive(Clone, Copy)]
pub(super) struct WitnessTable {
	global: u32,
	order_type: u32,
	/// The dispatcher's function index, once compiled
	dispatcher: Option<u32>,
}

impl WasmGcEmitter {
	/// The declared types that define `compare(a:T, b:T)`, in a stable order
	fn comparable_types(&self) -> Vec<String> {
		let mut types: Vec<String> = self.ctx.user_functions.keys().filter_map(|name| witness_type(name, COMPARE)).map(str::to_string).collect();
		types.sort();
		types
	}

	/// The witness global and the type of an ordering function `(Node, Node) -> i32`; `None` when no type is Comparable
	pub(super) fn compare_witness_table(&mut self) -> Option<WitnessTable> {
		if let Some(table) = self.compare_witness {
			return Some(table);
		}
		if self.comparable_types().is_empty() {
			return None;
		}
		let node_ref = Ref(self.node_ref(false));
		let order_type = self.type_manager.function_type(vec![node_ref, node_ref], vec![ValType::I32]);
		let witness_ref = RefType { nullable: true, heap_type: HeapType::Concrete(order_type) };
		self.globals.global(GlobalType { val_type: Ref(witness_ref), mutable: true, shared: false }, &ConstExpr::ref_null(HeapType::Concrete(order_type)));
		self.exports.export(WITNESS_GLOBAL, ExportKind::Global, self.next_global_idx);
		let table = WitnessTable { global: self.next_global_idx, order_type, dispatcher: None };
		self.next_global_idx += 1;
		self.compare_witness = Some(table);
		Some(table)
	}

	/// In node_order: two instances (Key nodes) in locals `first` and `second` return the witness's order
	pub(super) fn emit_instance_order(&self, func: &mut Function, table: WitnessTable, kinds: [u32; 2], first: u32, second: u32) {
		for kind in kinds {
			Self::emit_list(func, &[I::LocalGet(kind), I::I64Const(Kind::Key as i64), I::I64Eq]);
		}
		Self::emit_list(func, &[I::I32And, I::If(BlockType::Empty), I::LocalGet(first), I::LocalGet(second), I::GlobalGet(table.global)]);
		Self::emit_list(func, &[I::RefAsNonNull, I::CallRef(table.order_type), I::Return, I::End]);
	}

	/// instance_compare(a, b): the order of two instances by the witness of a's type; a type without one is not comparable
	pub(super) fn emit_compare_dispatcher(&mut self) {
		let Some(table) = self.compare_witness else { return };
		let node_type = self.type_manager.node_type;
		let mut func = Function::new(vec![(1, ValType::I64), (1, ValType::F64)]);
		let (first, second, order, float_order) = (0, 1, 2, 3);
		for type_name in self.comparable_types() {
			let witness = self.ctx.user_functions[&witness_name(COMPARE, &type_name)].clone();
			let Some(index) = witness.func_index else { continue };
			// the type name of an instance is the data of its Key node
			self.emit_field(&mut func, first, 1);
			func.instruction(&I::RefCastNonNull(HeapType::Concrete(node_type)));
			let (pointer, length) = self.allocate_string(&type_name);
			Self::emit_list(&mut func, &[I::I32Const(pointer as i32), I::I32Const(length as i32)]);
			self.emit_call(&mut func, "new_symbol");
			self.emit_call(&mut func, super::VALUES_EQUAL);
			Self::emit_list(&mut func, &[I::If(BlockType::Empty), I::LocalGet(first), I::LocalGet(second), I::Call(index)]);
			if witness.return_kind.is_float() {
				let zero = I::F64Const(Ieee64::new(0f64.to_bits()));
				Self::emit_list(&mut func, &[I::LocalTee(float_order), zero.clone(), I::F64Gt, I::LocalGet(float_order), zero, I::F64Lt, I::I32Sub, I::Return, I::End]);
				continue;
			}
			if witness.return_kind.is_ref() {
				self.emit_call(&mut func, "get_int_value");
			}
			// the sign of the i64: (order > 0) - (order < 0)
			Self::emit_list(&mut func, &[I::LocalTee(order), I::I64Const(0), I::I64GtS, I::LocalGet(order), I::I64Const(0), I::I64LtS, I::I32Sub, I::Return, I::End]);
		}
		self.emit_call(&mut func, "not_comparable");
		func.instruction(&I::Unreachable);
		func.instruction(&I::End);
		self.functions.function(table.order_type);
		self.code.function(&func);
		let index = self.next_func_idx;
		self.next_func_idx += 1;
		self.exports.export(INSTANCE_COMPARE, ExportKind::Func, index);
		self.compare_witness = Some(WitnessTable { dispatcher: Some(index), ..table });
	}

	/// At the start of main: fill the witness table
	pub(super) fn emit_witness_installation(&self, func: &mut Function) {
		let Some(WitnessTable { global, dispatcher: Some(dispatcher), .. }) = self.compare_witness else { return };
		Self::emit_list(func, &[I::RefFunc(dispatcher), I::GlobalSet(global)]);
	}

	/// `ref.func` needs its functions declared in an element segment
	pub(super) fn witness_dispatcher(&self) -> Option<u32> {
		self.compare_witness?.dispatcher
	}
}
