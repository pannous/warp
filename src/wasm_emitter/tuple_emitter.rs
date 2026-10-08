//! Tuple returns as wasm multi-value (src/lowering/tuples.rs, notes/multi_value.md): a function with `return a, b` has one wasm
//! result per value; `x, y = f()` stores them straight into x and y; any other call packs them into the list `[a b]`
//! through the function's packer `f$list`, so everywhere else such a function is an ordinary List-returning one.

use super::WasmGcEmitter;
use crate::node::{Bracket, Node};
use crate::tuples::{call_parts, destructuring, returned_values, unstarred};
use crate::type_kinds::Kind;
use wasm_encoder::{Function, Instruction as I, ValType};

/// `a, b = [1, 2, 3]`: the runtime error of unpacking another number of items than names
pub(super) const WRONG_NUMBER_OF_VALUES: &str = "wrong_number_of_values";
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
		read(self, func, &Self::destructured_value(&names));
		true
	}

	/// The value of a destructuring statement is its last name's, as `y = v` gives v
	fn destructured_value(names: &[String]) -> Node {
		Node::Symbol(names.last().map(|name| unstarred(name).to_string()).unwrap_or_default())
	}

	/// The kind of a destructuring statement's value (its last name's), None for any other statement
	pub(super) fn destructured_kind(&self, node: &Node) -> Option<Kind> {
		destructuring(node).map(|(names, _)| self.get_type(&Self::destructured_value(&names)))
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
		if let ([value], true) = (values, names.iter().any(|name| unstarred(name) != name)) {
			self.emit_unpacking(func, names, value); // a tuple call packs its values into a list here
			return;
		}
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
					self.emit_unpacking(func, names, call);
					return;
				}
			},
			_ => {
				// all values first, then the stores: `x, y = y, x` swaps
				let kinds: Vec<Kind> = names.iter().map(|name| self.variable_kind(name)).collect();
				for ((value, kind), name) in values.iter().zip(&kinds).zip(names) {
					let declared = self.declared_type_of(&Node::Symbol(name.clone()));
					self.emit_declared_value(func, declared.as_ref(), value, *kind);
				}
				kinds
			}
		};
		for (name, kind) in names.iter().zip(kinds).rev() {
			self.emit_store_destructured(func, name, kind);
		}
	}

	/// `a, b = xs` (a list, a text, `(1, 2)`): the items by position into the names; another count of items is the
	/// runtime error wrong_number_of_values, as Python's "too many / not enough values to unpack".
	/// A starred name `*rest` takes the items the others leave, as a list (possibly empty).
	fn emit_unpacking(&mut self, func: &mut Function, names: &[String], value: &Node) {
		let (items, count) = (self.container_scratch(), self.scratch(0));
		let star = names.iter().position(|name| unstarred(name) != name);
		let fixed = names.len() as i64 - star.map_or(0, |_| 1);
		self.emit_node_instructions(func, value);
		func.instruction(&I::LocalSet(items));
		func.instruction(&I::LocalGet(items));
		func.instruction(&I::RefAsNonNull);
		self.emit_call(func, "node_count");
		func.instruction(&I::LocalTee(count));
		func.instruction(&I::I64Const(fixed));
		func.instruction(if star.is_some() { &I::I64LtS } else { &I::I64Ne });
		self.emit_fail_if(func, WRONG_NUMBER_OF_VALUES);
		for (position, name) in names.iter().enumerate() {
			let after_star = star.is_some_and(|star| position > star);
			let from_end = names.len() as i64 - position as i64; // 1 for the last name
			func.instruction(&I::LocalGet(items));
			if Some(position) == star {
				// node_slice(items, star, count - names after it): 0-based, end exclusive, ø = to the end
				func.instruction(&I::I64Const(position as i64));
				self.emit_call(func, "new_int");
				func.instruction(&I::LocalGet(count));
				func.instruction(&I::I64Const(from_end - 1));
				func.instruction(&I::I64Sub);
				self.emit_call(func, "new_int");
				self.emit_call(func, super::library_ops::NODE_SLICE);
				self.emit_store_destructured(func, unstarred(name), Kind::List);
				continue;
			}
			func.instruction(&I::RefAsNonNull);
			if after_star {
				func.instruction(&I::LocalGet(count));
				func.instruction(&I::I64Const(from_end - 1));
				func.instruction(&I::I64Sub);
			} else {
				func.instruction(&I::I64Const(position as i64 + 1));
			}
			self.emit_call(func, "node_index_at");
			self.emit_store_destructured(func, name, Kind::Data);
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
		if matches!(variable_kind, Kind::Int | Kind::Float) && kind.is_ref() {
			// `a = 0; a, b = [5, 6]`: a number item into a variable that holds a number
			self.emit_node_as_number(func, variable_kind);
		} else if self.storage_type(variable_kind) != self.storage_type(kind) {
			self.emit_type_error(func, format!("{name} holds {variable_kind:?}, the destructured value is {kind:?}"));
			return;
		}
		if variable_kind == Kind::Int {
			self.emit_fits_declared(func, &Node::Symbol(name.to_string())); // `a: int`: no ratio, within its width
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
		let (values, node) = (self.tuple_result_types(kinds), ValType::Ref(self.node_ref(false)));
		let packer_type = self.type_manager.function_type(values, vec![node]);
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
