//! Closures at run time (lowering in src/closures.rs, notes/closures.md):
//!   (type $Closure (struct (field $entry (ref func)) (field $captured (ref null $Node))))
//!   (type $closure_entry_n (func (param $captured (ref null $Node)) (param (ref null $Node))×n (result (ref null $Node))))
//! A closure value is a $Node of Kind::Function whose data is the $Closure and whose value is the function name as a Symbol.
//! Every closure entry takes and returns Nodes (the universal signature): `closure_entry_<target>` unboxes the captured values
//! (a list) and the arguments to the parameter kinds of the target, calls it and boxes its result. `closure_call_n(f, a1…an)`
//! casts the entry to $closure_entry_n and call_refs it, unboxing the result to the kind all closures of arity n return.

use super::WasmGcEmitter;
use crate::closures::closure_call_arity;
use crate::context::UserFunctionDef;
use crate::analyzer::param_kind;
use crate::node::{Bracket, Node, Separator};
use crate::type_kinds::Kind;
use std::collections::HashMap;
use wasm_encoder::*;
use Instruction as I;
use StorageType::Val;
use ValType::Ref;

const ENTRY_PREFIX: &str = "closure_entry_";
/// Runtime errors of a closure call: the value called is no closure, or a closure of another arity
pub(super) const NOT_A_FUNCTION: &str = "not_a_function";
pub(super) const WRONG_ARGUMENT_COUNT: &str = "wrong_number_of_arguments";
const ENTRY_FIELD: u32 = 0;
const CAPTURED_FIELD: u32 = 1;
const NODE_DATA_FIELD: u32 = 1;
const NODE_VALUE_FIELD: u32 = 2;

/// The types and entry functions of the closures in a module
#[derive(Default, Clone)]
pub(super) struct ClosureTypes {
	closure: Option<u32>,
	/// Arity → $closure_entry_n
	entries: HashMap<usize, u32>,
	/// Target function → (entry function index, captured count, arity)
	entry_functions: Vec<(String, u32, usize, usize)>,
}

impl ClosureTypes {
	/// The entry functions referenced by `ref.func`: a module must declare them in an element segment
	pub(super) fn declared_functions(&self) -> Vec<u32> {
		self.entry_functions.iter().map(|(_, index, _, _)| *index).collect()
	}

	pub(super) fn entry_names(&self) -> Vec<(u32, String)> {
		self.entry_functions.iter().map(|(target, index, _, _)| (*index, format!("{ENTRY_PREFIX}{target}"))).collect()
	}
}

impl WasmGcEmitter {
	fn nullable_node(&self) -> ValType {
		Ref(self.node_ref(true))
	}

	fn closure_type(&mut self) -> u32 {
		if let Some(closure) = self.closures.closure {
			return closure;
		}
		let index = self.type_manager.types().len();
		let entry = FieldType { element_type: Val(Ref(RefType { nullable: false, heap_type: HeapType::FUNC })), mutable: false };
		let captured = FieldType { element_type: Val(self.nullable_node()), mutable: false };
		self.type_manager.types_mut().ty().struct_(vec![entry, captured]);
		self.closures.closure = Some(index);
		index
	}

	/// Do the closures of `arity` take and return Ints (closures::type_closure_calls): their entries skip the Node boxes
	fn typed_entry(&self, arity: usize) -> bool {
		self.ctx.user_functions.get(&crate::closures::closure_call_name(arity)).is_some_and(crate::closures::is_typed_closure_call)
	}

	fn entry_type(&mut self, arity: usize) -> u32 {
		if let Some(entry) = self.closures.entries.get(&arity) {
			return *entry;
		}
		let index = self.type_manager.types().len();
		let node = self.nullable_node();
		if self.typed_entry(arity) {
			let helper = &self.ctx.user_functions[&crate::closures::closure_call_name(arity)];
			let number = |kind: Kind| if kind.is_float() { ValType::F64 } else { ValType::I64 };
			let params = std::iter::once(node).chain(helper.params[1..].iter().map(|param| number(param_kind(param)))).collect::<Vec<_>>();
			let result = number(helper.return_kind);
			self.type_manager.types_mut().ty().function(params, vec![result]);
		} else {
			self.type_manager.types_mut().ty().function(vec![node; arity + 1], vec![node]);
		}
		self.closures.entries.insert(arity, index);
		index
	}

	/// Pass 1 of compile_user_functions: a function index for the entry of every closure target
	pub(super) fn register_closure_entries(&mut self) {
		for (target, captured) in self.ctx.closure_targets.clone() {
			let Some(function) = self.ctx.user_functions.get(&target) else { continue };
			let arity = function.params.len().saturating_sub(captured);
			let entry_type = self.entry_type(arity);
			self.functions.function(entry_type);
			self.closures.entry_functions.push((target, self.next_func_idx, captured, arity));
			self.next_func_idx += 1;
		}
	}

	/// Pass 2: the entry bodies, in the order of their registration
	pub(super) fn compile_closure_entries(&mut self) {
		for (target, _, captured, arity) in self.closures.entry_functions.clone() {
			let function = self.ctx.user_functions[&target].clone();
			let typed = self.typed_entry(arity);
			let mut func = Function::new(vec![]);
			for (index, param) in function.params.iter().enumerate() {
				if index < captured {
					// the index-th value of the captured list: data of the index-th cons cell
					func.instruction(&I::LocalGet(0));
					for _ in 0..index {
						func.instruction(&I::StructGet { struct_type_index: self.type_manager.node_type, field_index: NODE_VALUE_FIELD });
					}
					func.instruction(&I::StructGet { struct_type_index: self.type_manager.node_type, field_index: NODE_DATA_FIELD });
					func.instruction(&I::RefCastNonNull(HeapType::Concrete(self.type_manager.node_type)));
				} else if typed {
					func.instruction(&I::LocalGet((1 + index - captured) as u32)); // an i64 or f64 already
					continue;
				} else {
					func.instruction(&I::LocalGet((1 + index - captured) as u32));
					func.instruction(&I::RefAsNonNull);
				}
				self.emit_node_as_kind(&mut func, param_kind(param));
			}
			self.emit_call_user_function(&mut func, &function);
			if !function.return_kind.is_ref() && !typed {
				self.emit_primitive_as_node(&mut func, function.return_kind);
			}
			func.instruction(&I::End);
			self.code.function(&func);
			debug_assert!(arity + captured == function.params.len());
		}
	}

	fn emit_call_user_function(&mut self, func: &mut Function, function: &UserFunctionDef) {
		match function.func_index {
			Some(index) => func.instruction(&I::Call(index)),
			None => func.instruction(&I::Unreachable),
		};
	}

	/// A Node on the stack in the representation `storage_type(kind)`
	fn emit_node_as_kind(&mut self, func: &mut Function, kind: Kind) {
		if kind.is_ref() {
			return;
		}
		if kind.is_float() {
			func.instruction(&I::StructGet { struct_type_index: self.type_manager.node_type, field_index: NODE_DATA_FIELD });
			func.instruction(&I::RefCastNonNull(HeapType::Concrete(self.type_manager.f64_box_type)));
			func.instruction(&I::StructGet { struct_type_index: self.type_manager.f64_box_type, field_index: 0 });
		} else if kind == Kind::Codepoint {
			self.emit_codepoint_of_node(func);
		} else {
			self.emit_call(func, "get_int_value");
		}
	}

	/// Stack [Codepoint node] → [i64 code point]
	pub(super) fn emit_codepoint_of_node(&self, func: &mut Function) {
		func.instruction(&I::StructGet { struct_type_index: self.type_manager.node_type, field_index: NODE_DATA_FIELD });
		func.instruction(&I::RefCastNonNull(HeapType::I31));
		func.instruction(&I::I31GetU);
		func.instruction(&I::I64ExtendI32U);
	}

	/// Is `name` a helper `closure_call_n` whose body the emitter writes
	pub(super) fn is_closure_call(&self, name: &str) -> bool {
		closure_call_arity(name).is_some() && self.ctx.user_functions.get(name).is_some_and(|function| function.body.is_empty())
	}

	/// `closure_call_n(f, a1…an)`: the entry of f's $Closure, cast to its arity, called with the captured values and the arguments
	pub(super) fn compile_closure_call(&mut self, name: &str) {
		let function = self.ctx.user_functions[name].clone();
		let arity = closure_call_arity(name).expect("a closure call");
		let (closure, entry) = (self.closure_type(), self.entry_type(arity));
		let closure_local = (arity + 1) as u32;
		let mut func = Function::new(vec![(1, Ref(RefType { nullable: true, heap_type: HeapType::Concrete(closure) }))]);
		func.instruction(&I::LocalGet(0));
		func.instruction(&I::StructGet { struct_type_index: self.type_manager.node_type, field_index: NODE_DATA_FIELD });
		func.instruction(&I::RefTestNonNull(HeapType::Concrete(closure)));
		self.emit_fail_unless(&mut func, NOT_A_FUNCTION);
		func.instruction(&I::LocalGet(0));
		func.instruction(&I::StructGet { struct_type_index: self.type_manager.node_type, field_index: NODE_DATA_FIELD });
		func.instruction(&I::RefCastNonNull(HeapType::Concrete(closure)));
		func.instruction(&I::LocalTee(closure_local));
		func.instruction(&I::StructGet { struct_type_index: closure, field_index: ENTRY_FIELD });
		func.instruction(&I::RefTestNonNull(HeapType::Concrete(entry)));
		self.emit_fail_unless(&mut func, WRONG_ARGUMENT_COUNT);
		func.instruction(&I::LocalGet(closure_local));
		func.instruction(&I::StructGet { struct_type_index: closure, field_index: CAPTURED_FIELD });
		for argument in 1..=arity {
			func.instruction(&I::LocalGet(argument as u32));
		}
		func.instruction(&I::LocalGet(closure_local));
		func.instruction(&I::StructGet { struct_type_index: closure, field_index: ENTRY_FIELD });
		func.instruction(&I::RefCastNonNull(HeapType::Concrete(entry)));
		func.instruction(&I::CallRef(entry));
		if !self.typed_entry(arity) {
			func.instruction(&I::RefAsNonNull);
			self.emit_node_as_kind(&mut func, function.return_kind);
		}
		func.instruction(&I::End);
		self.code.function(&func);
		let index = function.func_index.expect("registered in pass 1");
		self.exports.export(name, ExportKind::Func, index);
	}

	/// The runtime error `error` unless the i32 on the stack is true
	fn emit_fail_unless(&mut self, func: &mut Function, error: &'static str) {
		func.instruction(&I::I32Eqz);
		func.instruction(&I::If(BlockType::Empty));
		self.emit_runtime_error(func, error);
		func.instruction(&I::End);
	}

	/// `closure_new(target, captured…)`: the Node holding the $Closure of the target's entry and the captured values
	pub(super) fn emit_closure_new(&mut self, func: &mut Function, target: &str, captured: &[Node]) {
		let Some(entry_function) = self.closures.entry_functions.iter().find(|(name, ..)| name == target).map(|(_, index, ..)| *index) else {
			self.emit_type_error(func, format!("{target} is not a function value"));
			return;
		};
		let closure = self.closure_type();
		self.emit_kind(func, Kind::Function);
		func.instruction(&I::RefFunc(entry_function));
		if captured.is_empty() {
			self.emit_node_null(func);
		} else {
			self.emit_node_instructions(func, &Node::List(captured.to_vec(), Bracket::Square, Separator::Space));
		}
		func.instruction(&I::StructNew(closure));
		self.emit_string_call(func, target, "new_symbol");
		func.instruction(&I::StructNew(self.type_manager.node_type));
	}
}
