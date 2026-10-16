//! Calls of runtime functions into warp-written ones (lib/prelude.warp `hex_text` from list_join): the runtime functions
//! are compiled before the user functions, whose indices they cannot know yet, so each such callee gets a funcref global
//! named `late·<callee>` that main fills before anything runs, and the runtime calls it with call_ref (as witness.rs
//! does for Comparable)

use super::super::WasmGcEmitter;
use wasm_encoder::*;
use Instruction as I;

const LATE_PREFIX: &str = "late·";

#[derive(Clone)]
pub(in crate::wasm_emitter) struct LateCall {
	callee: &'static str,
	global: u32,
	function_type: u32,
}

impl WasmGcEmitter {
	/// The funcref global through which runtime functions call the user function `callee` of that signature, compiled
	/// later; declare it before the runtime function that calls it
	pub(in crate::wasm_emitter) fn late_call(
		&mut self,
		callee: &'static str,
		params: Vec<ValType>,
		results: Vec<ValType>,
	) -> LateCall {
		if let Some(late) = self.late_calls.iter().find(|late| late.callee == callee) {
			return late.clone();
		}
		let function_type = self.type_manager.function_type(params, results);
		let heap_type = HeapType::Concrete(function_type);
		let global = self.declare_named_global(
			ValType::Ref(RefType { nullable: true, heap_type }),
			ConstExpr::ref_null(heap_type),
			format!("{LATE_PREFIX}{callee}"),
		);
		let late = LateCall { callee, global, function_type };
		self.late_calls.push(late.clone());
		late
	}

	pub(in crate::wasm_emitter) fn emit_late_call(func: &mut Function, late: &LateCall) {
		Self::emit_list(func, &[I::GlobalGet(late.global), I::RefAsNonNull, I::CallRef(late.function_type)]);
	}

	/// At the start of main: every late-called function into its global
	pub(in crate::wasm_emitter) fn emit_late_call_installation(&mut self, func: &mut Function) {
		for late in self.late_calls.clone() {
			match self.late_index(&late) {
				Some(index) => Self::emit_list(func, &[I::RefFunc(index), I::GlobalSet(late.global)]),
				None => self.type_errors.push(format!("{} is called by the runtime but was not compiled", late.callee)),
			}
		}
	}

	/// The late-called functions' indices, which ref.func needs declared in an element segment
	pub(in crate::wasm_emitter) fn late_called_functions(&self) -> Vec<u32> {
		self.late_calls.iter().filter_map(|late| self.late_index(late)).collect()
	}

	fn late_index(&self, late: &LateCall) -> Option<u32> {
		self.ctx.user_functions.get(late.callee).and_then(|function| function.func_index)
	}
}
