//! The runtime witness tables of Comparable and Printable (notes/traits.md): where the static type of an instance is
//! unknown, `node_order` asks the funcref global `compare·witness` to order two of them, and list_text asks
//! `text·witness` for the text of one (card list-printable-items: `[N(1), N(2)]` prints `[n1 n2]`). Each holds a
//! dispatcher (`instance_compare`, `text·dispatch`), which matches the type name of the instance and calls that type's
//! witness `compare·T` / `text·T` (traits.rs). The dispatchers are compiled after the user functions, whose indices
//! they need, and the start function installs them in every instance; runtime functions compiled earlier reach them
//! through the globals with call_ref. `text·shown` gives main's value with every printable instance in it as the
//! symbol of its text.

use super::super::WasmGcEmitter;
use crate::traits::{witness_name, witness_type, COMPARE};
use crate::type_kinds::Kind;
use crate::words::TEXT_WORD;
use wasm_encoder::*;
use Instruction as I;
use ValType::Ref;

pub(in crate::wasm_emitter) const INSTANCE_COMPARE: &str = "instance_compare";
/// exported under witness-like names, which no function of a program takes (lib/units.warp defines a `shown`)
const INSTANCE_TEXT: &str = "text·dispatch";
const SHOWN: &str = "text·shown";
const WITNESS_SUFFIX: &str = "·witness";
/// The operations whose witnesses the runtime calls with Nodes, so they take no calling convention of their own
pub(in crate::wasm_emitter) const DISPATCHED_OPERATIONS: [&str; 2] = [COMPARE, TEXT_WORD];

#[derive(Clone, Copy)]
pub(in crate::wasm_emitter) struct WitnessTable {
	global: u32,
	function_type: u32,
	/// The dispatcher's function index, once compiled
	dispatcher: Option<u32>,
}

impl WasmGcEmitter {
	/// The declared types that define `operation` (`compare(a:T, b:T)`, `text(p:T)`), in a stable order
	fn witness_types(&self, operation: &str) -> Vec<String> {
		let mut types: Vec<String> = self
			.ctx
			.user_functions
			.keys()
			.filter_map(|name| witness_type(name, operation))
			.map(str::to_string)
			.collect();
		types.sort();
		types
	}

	/// The exported funcref global `<operation>·witness` of a dispatcher's type; `None` when no type defines the operation
	fn witness_table(&mut self, operation: &str, params: Vec<ValType>, results: Vec<ValType>) -> Option<WitnessTable> {
		if self.witness_types(operation).is_empty() {
			return None;
		}
		let function_type = self.type_manager.function_type(params, results);
		let witness_ref = RefType { nullable: true, heap_type: HeapType::Concrete(function_type) };
		let global = self.declare_global_of(Ref(witness_ref), ConstExpr::ref_null(HeapType::Concrete(function_type)));
		self.exports.export(&format!("{operation}{WITNESS_SUFFIX}"), ExportKind::Global, global);
		Some(WitnessTable { global, function_type, dispatcher: None })
	}

	/// The Comparable table, its dispatcher ordering `(Node, Node) -> i32`
	pub(in crate::wasm_emitter) fn compare_witness_table(&mut self) -> Option<WitnessTable> {
		if self.compare_witness.is_none() {
			let node_ref = Ref(self.node_ref(false));
			self.compare_witness = self.witness_table(COMPARE, vec![node_ref, node_ref], vec![ValType::I32]);
		}
		self.compare_witness
	}

	/// The Printable table, its dispatcher giving `(Node) -> Node` the text of an instance of a printable type, any
	/// other node itself
	pub(in crate::wasm_emitter) fn text_witness_table(&mut self) -> Option<WitnessTable> {
		if self.text_witness.is_none() {
			let node_ref = Ref(self.node_ref(false));
			self.text_witness = self.witness_table(TEXT_WORD, vec![node_ref], vec![node_ref]);
		}
		self.text_witness
	}

	/// In a runtime function: the node in `local` replaced by its text when it is a printable instance
	pub(in crate::wasm_emitter) fn emit_instance_as_text(&self, func: &mut Function, table: WitnessTable, local: u32) {
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::LocalGet(local), I::RefAsNonNull, I::GlobalGet(table.global), I::RefAsNonNull,
			I::CallRef(table.function_type), I::LocalSet(local),
		]);
	}

	/// An i32: is the instance (a Key node) in `local` of the type `type_name`; its type name is the data of the Key
	fn emit_is_instance_of(&mut self, func: &mut Function, local: u32, type_name: &str) {
		self.emit_field(func, local, 1);
		func.instruction(&I::RefCastNonNull(HeapType::Concrete(self.type_manager.node_type)));
		let (pointer, length) = self.memory_string(type_name);
		Self::emit_list(func, &[I::I32Const(pointer as i32), I::I32Const(length as i32)]);
		self.emit_call(func, "new_symbol");
		self.emit_call(func, super::super::VALUES_EQUAL);
	}

	/// In node_order: two instances (Key nodes) in locals `first` and `second` return the witness's order
	pub(in crate::wasm_emitter) fn emit_instance_order(
		&self,
		func: &mut Function,
		table: WitnessTable,
		kinds: [u32; 2],
		first: u32,
		second: u32,
	) {
		for kind in kinds {
			Self::emit_list(func, &[I::LocalGet(kind), I::I64Const(Kind::Key as i64), I::I64Eq]);
		}
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::I32And, I::If(BlockType::Empty),
				I::LocalGet(first), I::LocalGet(second), I::GlobalGet(table.global), I::RefAsNonNull,
				I::CallRef(table.function_type), I::Return,
			I::End,
		]);
	}

	/// instance_compare(a, b): the order of two instances by the witness of a's type; a type without one is not comparable
	pub(in crate::wasm_emitter) fn emit_compare_dispatcher(&mut self) {
		let Some(table) = self.compare_witness else { return };
		let node_ref = Ref(self.node_ref(false));
		let index = self.host_only_function(
			INSTANCE_COMPARE,
			vec![node_ref; 2],
			vec![ValType::I32],
			vec![ValType::I64, ValType::F64],
			|s, func| {
				let (first, second, order, float_order) = (0, 1, 2, 3);
				for type_name in s.witness_types(COMPARE) {
					let witness = s.ctx.user_functions[&witness_name(COMPARE, &type_name)].clone();
					let Some(index) = witness.func_index else { continue };
					s.emit_is_instance_of(func, first, &type_name);
					Self::emit_list(
						func,
						&[I::If(BlockType::Empty), I::LocalGet(first), I::LocalGet(second), I::Call(index)],
					);
					if witness.return_kind.is_float() {
						let zero = I::F64Const(Ieee64::new(0f64.to_bits()));
						Self::emit_list(
							func,
							&[
								I::LocalTee(float_order),
								zero.clone(),
								I::F64Gt,
								I::LocalGet(float_order),
								zero,
								I::F64Lt,
								I::I32Sub,
								I::Return,
								I::End,
							],
						);
						continue;
					}
					if witness.return_kind.is_ref() {
						s.emit_call(func, "get_int_value");
					}
					// the sign of the i64: (order > 0) - (order < 0)
					#[rustfmt::skip]
					Self::emit_list(func, &[
						I::LocalTee(order), I::I64Const(0), I::I64GtS, I::LocalGet(order), I::I64Const(0), I::I64LtS,
						I::I32Sub, I::Return,
						I::End,
					]);
				}
				s.emit_call(func, "not_comparable");
				func.instruction(&I::Unreachable);
			},
		);
		self.compare_witness = Some(WitnessTable { dispatcher: Some(index), ..table });
	}

	/// text·dispatch(node): the text `text·T` gives an instance of a printable type T, any other node itself
	pub(in crate::wasm_emitter) fn emit_text_dispatcher(&mut self) {
		let Some(table) = self.text_witness else { return };
		let node_ref = Ref(self.node_ref(false));
		let index = self.host_only_function(INSTANCE_TEXT, vec![node_ref], vec![node_ref], vec![], |s, func| {
			let node = 0;
			s.emit_field(func, node, 0);
			Self::emit_kind_test(func, Kind::Key as i64, I::I64Ne);
			Self::emit_list(func, &[I::If(BlockType::Empty), I::LocalGet(node), I::Return, I::End]);
			for type_name in s.witness_types(TEXT_WORD) {
				let witness = s.ctx.user_functions[&witness_name(TEXT_WORD, &type_name)].clone();
				let (Some(index), true) = (witness.func_index, witness.return_kind.is_ref()) else { continue };
				s.emit_is_instance_of(func, node, &type_name);
				Self::emit_list(func, &[I::If(BlockType::Empty), I::LocalGet(node), I::Call(index), I::Return, I::End]);
			}
			func.instruction(&I::LocalGet(node));
		});
		self.text_witness = Some(WitnessTable { dispatcher: Some(index), ..table });
		self.emit_shown(index);
	}

	/// shown(node): the node with each printable instance in it, at any depth of lists and entries, the symbol of its
	/// text, as main gives it to be shown (`[N(1), N(2)]` as `[n1 n2]`)
	fn emit_shown(&mut self, instance_text: u32) {
		let (node_type, nullable) = (self.type_manager.node_type, Ref(self.node_ref(true)));
		let own_index = self.next_func_idx;
		self.shown = Some(own_index);
		let locals = vec![nullable, ValType::I64, Ref(RefType::ANYREF)];
		self.host_only_function(SHOWN, vec![nullable], vec![nullable], locals, |_, func| {
			let (node, text, kind, data) = (0, 1, 2, 3);
			let node_heap = HeapType::Concrete(node_type);
			let field = |field_index: u32| I::StructGet { struct_type_index: node_type, field_index };
			let kind_is = |func: &mut Function, shape: Kind| {
				func.instruction(&I::LocalGet(kind));
				Self::emit_kind_test(func, shape as i64, I::I64Eq);
			};
			#[rustfmt::skip]
			Self::emit_list(func, &[
				I::LocalGet(node), I::RefIsNull, I::If(BlockType::Empty), I::RefNull(node_heap), I::Return, I::End,
				I::LocalGet(node), field(0), I::LocalSet(kind),
				// a printable instance: the symbol of its text
				I::LocalGet(node), I::RefAsNonNull, I::Call(instance_text), I::LocalTee(text), I::LocalGet(node), I::RefEq,
				I::I32Eqz, I::If(BlockType::Empty),
					I::I64Const(Kind::Symbol as i64), I::LocalGet(text), field(1), I::RefNull(node_heap),
					I::StructNew(node_type), I::Return,
				I::End,
			]);
			// a list cell (its first item and the rest) or an entry (its key and value): rebuilt of its shown parts
			kind_is(func, Kind::List);
			kind_is(func, Kind::Block);
			func.instruction(&I::I32Or);
			kind_is(func, Kind::Key);
			// the data field holds a node only in some lists (others hold arrays): any other payload is kept
			#[rustfmt::skip]
			Self::emit_list(func, &[
				I::I32Or, I::If(BlockType::Empty),
					I::LocalGet(kind),
					I::LocalGet(node), field(1), I::LocalTee(data), I::RefTestNullable(node_heap),
					I::If(BlockType::Result(Ref(RefType::ANYREF))),
						I::LocalGet(data), I::RefCastNullable(node_heap), I::Call(own_index),
					I::Else, I::LocalGet(data), I::End,
					I::LocalGet(node), field(2), I::Call(own_index),
					I::StructNew(node_type), I::Return,
				I::End,
				I::LocalGet(node),
			]);
		});
	}

	/// At the end of main: its value as shown
	pub(in crate::wasm_emitter) fn emit_shown_value(&self, func: &mut Function) {
		if let Some(shown) = self.shown {
			Self::emit_list(func, &[I::Call(shown), I::RefAsNonNull]);
		}
	}

	/// In the start function: fill the witness tables
	pub(in crate::wasm_emitter) fn emit_witness_installation(&self, func: &mut Function) {
		for table in [self.compare_witness, self.text_witness].into_iter().flatten() {
			if let Some(dispatcher) = table.dispatcher {
				Self::emit_list(func, &[I::RefFunc(dispatcher), I::GlobalSet(table.global)]);
			}
		}
	}

	/// `ref.func` needs its functions declared in an element segment
	pub(in crate::wasm_emitter) fn witness_dispatchers(&self) -> Vec<u32> {
		[self.compare_witness, self.text_witness].into_iter().flatten().filter_map(|table| table.dispatcher).collect()
	}
}
