//! Emitting a whole function: its type, its declaration, its body and its name in one call.

use super::WasmGcEmitter;
use wasm_encoder::*;
use Instruction as I;

/// The runtime functions a page's scripts call (web/playground: host.js buildValue, guarded_call, reader.js readNode)
const PAGE_HOST_CALLS: [&str; 12] = ["new_empty", "new_int", "new_float", "new_codepoint", "new_text", "new_symbol", "new_key", "new_list", "get_kind", "error_of", super::int_lists::INTS_TO_LIST, super::int_lists::LIST_TO_INTS];

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
		let func_type = self.type_manager.function_type(params, results);
		self.functions.function(func_type);
		self.function_body(locals, body);
		self.register_func(name)
	}

	/// Pass 1 of a function whose body comes later: its type declared, its index taken
	pub(super) fn reserve_function(&mut self, func_type: u32) -> u32 {
		self.functions.function(func_type);
		self.next_func_idx += 1;
		self.next_func_idx - 1
	}

	/// The code of the next declared function: `body` writes the instructions, the closing `end` is added here
	pub(super) fn function_body(&mut self, locals: Vec<ValType>, body: impl FnOnce(&mut Self, &mut Function)) {
		let mut func = Function::new(locals.into_iter().map(|local| (1, local)).collect::<Vec<_>>());
		body(self, &mut func);
		func.instruction(&I::End);
		self.code.function(&func);
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
		self.runtime_function(name, params, results, locals, body);
		self.export_runtime_function(name);
	}

	/// A function only the host calls, exported as `export` without a name in the registry (a component adapter, a
	/// type's constructor, the compare dispatcher): its index, taken after `body` ran
	pub(super) fn host_only_function(&mut self, export: &str, params: Vec<ValType>, results: Vec<ValType>, locals: Vec<ValType>, body: impl FnOnce(&mut Self, &mut Function)) -> u32 {
		let func_type = self.type_manager.function_type(params, results);
		self.function_body(locals, body);
		let index = self.reserve_function(func_type);
		self.exports.export(export, ExportKind::Func, index);
		index
	}

	/// Exports the emitted runtime function `name` for the host; a page's host (web/playground) calls only some, and
	/// the others are left to tree shaking (web::test_bundle_budget)
	pub(super) fn export_runtime_function(&mut self, name: &'static str) {
		if !crate::pipeline::is_for_a_page() || PAGE_HOST_CALLS.contains(&name) {
			let index = self.func_index(name);
			self.exports.export(name, ExportKind::Func, index);
		}
	}

	pub(super) fn emit_list(func: &mut Function, instructions: &[Instruction]) {
		for instruction in instructions {
			func.instruction(instruction);
		}
	}

	/// `while condition { body }`: `condition` leaves an i32, `body` leaves nothing
	pub(super) fn emit_while(func: &mut Function, condition: &[Instruction], body: &[Instruction]) {
		Self::emit_list(func, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
		Self::emit_list(func, condition);
		Self::emit_list(func, &[I::I32Eqz, I::BrIf(1)]);
		Self::emit_list(func, body);
		Self::emit_list(func, &[I::Br(0), I::End, I::End]);
	}
}
