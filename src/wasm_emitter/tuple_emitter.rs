//! Tuple returns as wasm multi-value (src/tuples.rs, notes/multi_value.md): a function with `return a, b` has one wasm
//! result per value; `x, y = f()` stores them straight into x and y; any other call packs them into the list `[a b]`
//! through the function's packer `f$list`, so everywhere else such a function is an ordinary List-returning one.

use super::WasmGcEmitter;
use crate::node::{Bracket, Node};
use crate::tuples::{call_parts, destructuring, returned_values};
use crate::type_kinds::Kind;
use wasm_encoder::{Function, Instruction as I, ValType};

const LIST_BRACKET_INFO: i64 = 1; // `[a b]`, as emit_list_structure writes Bracket::Square

/// How the statement's own value is read afterwards: emit_numeric_value, emit_node_instructions or emit_float_value
pub(super) type ValueReader = fn(&mut WasmGcEmitter, &mut Function, &Node);

impl WasmGcEmitter {
	pub(super) fn tuple_result_types(&self, kinds: &[Kind]) -> Vec<ValType> {
		kinds.iter().map(|kind| self.storage_type(*kind)).collect()
	}

	/// `return a, b` and `x, y = …`: emitted here, true when `node` was one of them
	pub(super) fn emit_tuple_statement(&mut self, func: &mut Function, node: &Node, read: ValueReader) -> bool {
		if let Some(values) = returned_values(node) {
			self.emit_tuple_return(func, values);
			return true;
		}
		let Some((names, values)) = destructuring(node) else { return false };
		self.emit_destructuring(func, &names, values);
		// the statement's value is the last name's, as `y = v` gives v
		read(self, func, &Node::Symbol(names.last().cloned().unwrap_or_default()));
		true
	}

	/// In a tuple function every value in its own result; elsewhere (main) the values as the list `[a b]`
	fn emit_tuple_return(&mut self, func: &mut Function, values: &[Node]) {
		let kinds = self.returned_tuple.clone();
		if kinds.is_empty() {
			self.emit_list_structure(func, values, &Bracket::Square);
		} else {
			for (value, kind) in values.iter().zip(kinds) {
				self.emit_value_of_kind(func, value, kind);
			}
		}
		func.instruction(&I::Return);
		func.instruction(&I::Unreachable); // the stack is polymorphic after it: any expected value type fits
	}

	fn emit_destructuring(&mut self, func: &mut Function, names: &[String], values: &[Node]) {
		let kinds: Vec<Kind> = match values {
			[call] => match self.tuple_call(call) {
				Some((function, kinds)) if kinds.len() != names.len() => {
					self.emit_type_error(func, format!("{function} returns {} values, not {}: {} = {}", kinds.len(), names.len(), names.join(", "), call.serialize().trim()));
					return;
				}
				Some((function, kinds)) => {
					let (_, arguments) = call_parts(call).expect("a tuple call");
					let user_fn = self.ctx.user_functions[&function].clone();
					self.emit_user_function_values(func, &user_fn, arguments);
					kinds
				}
				None => {
					let message = format!("{} = {} needs a function returning {} values (`return a, b`) or {} values", names.join(", "), call.serialize().trim(), names.len(), names.len());
					self.emit_type_error(func, message);
					return;
				}
			},
			_ => {
				// all values first, then the stores: `x, y = y, x` swaps
				let kinds: Vec<Kind> = names.iter().map(|name| self.variable_kind(name)).collect();
				for (value, kind) in values.iter().zip(&kinds) {
					self.emit_value_of_kind(func, value, *kind);
				}
				kinds
			}
		};
		for (name, kind) in names.iter().zip(kinds).rev() {
			self.emit_store_destructured(func, name, kind);
		}
	}

	/// A call of a function returning several values: its name and their kinds
	fn tuple_call(&self, call: &Node) -> Option<(String, Vec<Kind>)> {
		let (name, _) = call_parts(call)?;
		let function = self.ctx.user_functions.get(name).filter(|function| !function.tuple_kinds.is_empty())?;
		Some((name.to_string(), function.tuple_kinds.clone()))
	}

	fn variable_kind(&self, name: &str) -> Kind {
		self.scope.lookup(name).map(|local| local.kind).or_else(|| self.ctx.user_globals.get(name).map(|(_, kind)| *kind)).unwrap_or(Kind::Data)
	}

	/// The value on top of the stack, of `kind`, into the variable `name`
	fn emit_store_destructured(&mut self, func: &mut Function, name: &str, kind: Kind) {
		let variable_kind = self.variable_kind(name);
		if self.storage_type(variable_kind) != self.storage_type(kind) {
			self.emit_type_error(func, format!("{name} holds {variable_kind:?}, the destructured value is {kind:?}"));
			return;
		}
		if let Some(local) = self.scope.lookup(name) {
			func.instruction(&I::LocalSet(local.position));
		} else if let Some(&(global, _)) = self.ctx.user_globals.get(name) {
			func.instruction(&I::GlobalSet(global));
		} else {
			self.emit_undefined_variable(func, name);
		}
	}

	/// After a call of a tuple function in any other place: its values as one list
	pub(super) fn emit_pack_tuple(&mut self, func: &mut Function, function: &str) {
		func.instruction(&I::Call(self.tuple_packers[function]));
	}

	/// `f$list(a, b) -> [a b]`: registered right after `f`, compiled right after its body (same order in both passes)
	pub(super) fn register_tuple_packer(&mut self, function: &str, kinds: &[Kind]) {
		let packer_type = self.type_manager.types().len();
		let (values, node) = (self.tuple_result_types(kinds), ValType::Ref(self.node_ref(false)));
		self.type_manager.types_mut().ty().function(values, vec![node]);
		self.functions.function(packer_type);
		self.tuple_packers.insert(function.to_string(), self.next_func_idx);
		self.next_func_idx += 1;
	}

	pub(super) fn compile_tuple_packer(&mut self, kinds: &[Kind]) {
		let mut packer = Function::new(vec![]);
		for (position, kind) in kinds.iter().enumerate() {
			packer.instruction(&I::LocalGet(position as u32));
			if !kind.is_ref() {
				self.emit_primitive_as_node(&mut packer, *kind);
			}
		}
		self.emit_node_null(&mut packer);
		for _ in kinds {
			packer.instruction(&I::I64Const(LIST_BRACKET_INFO));
			self.emit_call(&mut packer, "new_list");
		}
		packer.instruction(&I::End);
		self.code.function(&packer);
	}
}
