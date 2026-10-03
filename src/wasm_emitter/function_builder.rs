//! Emitting a whole function: its type, its declaration, its body and its name in one call.

use super::WasmGcEmitter;
use wasm_encoder::*;

impl WasmGcEmitter {
	/// Declares and emits the function `name`; `body` writes the instructions, the closing `end` is added here
	pub(super) fn runtime_function(
		&mut self,
		name: &'static str,
		params: Vec<ValType>,
		results: Vec<ValType>,
		locals: Vec<ValType>,
		body: impl FnOnce(&mut Self, &mut Function),
	) -> u32 {
		let func_type = self.type_manager.types().len();
		self.type_manager.types_mut().ty().function(params, results);
		self.functions.function(func_type);
		let mut func = Function::new(locals.into_iter().map(|t| (1, t)).collect::<Vec<_>>());
		body(self, &mut func);
		func.instruction(&Instruction::End);
		self.code.function(&func);
		self.register_func(name)
	}

	/// A runtime function the host can call too
	pub(super) fn exported_function(
		&mut self,
		name: &'static str,
		params: Vec<ValType>,
		results: Vec<ValType>,
		locals: Vec<ValType>,
		body: impl FnOnce(&mut Self, &mut Function),
	) {
		let index = self.runtime_function(name, params, results, locals, body);
		self.exports.export(name, ExportKind::Func, index);
	}

	pub(super) fn emit_list(func: &mut Function, instructions: &[Instruction]) {
		for instruction in instructions {
			func.instruction(instruction);
		}
	}
}
