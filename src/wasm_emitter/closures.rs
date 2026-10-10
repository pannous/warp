//! Closures at run time (lowering in src/lowering/closures.rs, notes/closures.md):
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
/// closure_rebuild(name, captured) and closure_captured(f): what a host needs to carry a closure to another instance of
/// the module (a task, src/tasks.rs): its target's name and its captured values; the entry is found again by name
pub const CLOSURE_REBUILD: &str = "closure_rebuild";
pub const CLOSURE_CAPTURED: &str = "closure_captured";
/// closure_apply(f, arguments): f called with the items of the list `arguments`, as many as f takes (missing ones ø,
/// extra ones dropped, as JavaScript calls a function): how a foreign runtime calls back a warp function it was given
pub const CLOSURE_APPLY: &str = "closure_apply";
/// Runtime errors of a closure call: the value called is no closure, or a closure of another arity
pub(super) const NOT_A_FUNCTION: &str = "not_a_function";
pub(super) const WRONG_ARGUMENT_COUNT: &str = "wrong_number_of_arguments";
const ENTRY_FIELD: u32 = 0;
/// The name a capture global is read by while a nested function's closure is made (nested_captures_now)
const CAPTURE_VALUE_PREFIX: &str = "·capture·";
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
	/// closure_rebuild and closure_captured, for a program that starts tasks
	carriers: Option<(u32, u32)>,
	/// closure_apply, for a program that calls foreign code
	apply: Option<u32>,
}

impl ClosureTypes {
	/// The entry functions referenced by `ref.func`: a module must declare them in an element segment
	pub(super) fn declared_functions(&self) -> Vec<u32> {
		self.entry_functions.iter().map(|(_, index, _, _)| *index).collect()
	}

	pub(super) fn entry_names(&self) -> Vec<(u32, String)> {
		let carriers = self.carriers.into_iter().flat_map(|(rebuild, captured)| [(rebuild, CLOSURE_REBUILD.to_string()), (captured, CLOSURE_CAPTURED.to_string())]);
		let apply = self.apply.map(|apply| (apply, CLOSURE_APPLY.to_string()));
		self.entry_functions.iter().map(|(target, index, _, _)| (*index, format!("{ENTRY_PREFIX}{target}"))).chain(carriers).chain(apply).collect()
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
		let node = self.nullable_node();
		let index = if self.typed_entry(arity) {
			let helper = &self.ctx.user_functions[&crate::closures::closure_call_name(arity)];
			let number = |kind: Kind| if kind.is_float() { ValType::F64 } else { ValType::I64 };
			let params = std::iter::once(node).chain(helper.params[1..].iter().map(|param| number(param_kind(param)))).collect::<Vec<_>>();
			let result = number(helper.return_kind);
			self.type_manager.function_type(params, vec![result])
		} else {
			self.type_manager.function_type(vec![node; arity + 1], vec![node])
		};
		self.closures.entries.insert(arity, index);
		index
	}

	/// Pass 1 of compile_user_functions: a function index for the entry of every closure target
	pub(super) fn register_closure_entries(&mut self) {
		for (target, captured) in self.ctx.closure_targets.clone() {
			let Some(function) = self.ctx.user_functions.get(&target) else { continue };
			let arity = function.params.len().saturating_sub(captured);
			let entry_type = self.entry_type(arity);
			let entry = self.reserve_function(entry_type);
			self.closures.entry_functions.push((target, entry, captured, arity));
		}
		// a program that starts tasks may pass closures to them: the carriers, exported
		let starts_tasks = self.ctx.ffi_imports.contains_key(crate::host::TASK_SPAWN_VALUES);
		if starts_tasks && !self.closures.entry_functions.is_empty() {
			let node = self.nullable_node();
			let rebuild_type = self.type_manager.function_type(vec![node, node], vec![node]);
			let captured_type = self.type_manager.function_type(vec![node], vec![node]);
			let (rebuild, captured) = (self.reserve_function(rebuild_type), self.reserve_function(captured_type));
			self.exports.export(CLOSURE_REBUILD, ExportKind::Func, rebuild);
			self.exports.export(CLOSURE_CAPTURED, ExportKind::Func, captured);
			self.closures.carriers = Some((rebuild, captured));
		}
		// a program that calls foreign code may hand it closures to call back
		let calls_foreign = self.ctx.ffi_imports.contains_key(crate::host::FOREIGN_CALL);
		if calls_foreign && !self.closures.entry_functions.is_empty() {
			let node = self.nullable_node();
			let apply_type = self.type_manager.function_type(vec![node, node], vec![node]);
			let apply = self.reserve_function(apply_type);
			self.exports.export(CLOSURE_APPLY, ExportKind::Func, apply);
			self.closures.apply = Some(apply);
		}
	}

	/// closure_apply(f, arguments): for each arity a closure of the module has, a test of f's entry against that arity's
	/// type; the matching one is called with that many items of the list
	fn compile_closure_apply(&mut self) {
		if self.closures.apply.is_none() {
			return;
		}
		let (node_type, closure) = (self.type_manager.node_type, self.closure_type());
		let (arguments, closure_local, item) = (1, 2, 3);
		let nullable_node = self.nullable_node();
		self.function_body(vec![Ref(RefType { nullable: true, heap_type: HeapType::Concrete(closure) }), nullable_node], |s, func| {
			func.instruction(&I::LocalGet(0));
			func.instruction(&I::RefAsNonNull);
			func.instruction(&I::StructGet { struct_type_index: node_type, field_index: NODE_DATA_FIELD });
			func.instruction(&I::RefTestNonNull(HeapType::Concrete(closure)));
			s.emit_fail_unless(func, NOT_A_FUNCTION);
			func.instruction(&I::LocalGet(0));
			func.instruction(&I::StructGet { struct_type_index: node_type, field_index: NODE_DATA_FIELD });
			func.instruction(&I::RefCastNonNull(HeapType::Concrete(closure)));
			func.instruction(&I::LocalSet(closure_local));
			let mut arities = s.closures.entries.keys().copied().collect::<Vec<_>>();
			arities.sort();
			for arity in arities {
				let entry = s.entry_type(arity);
				let typed = s.typed_entry(arity);
				let helper = s.ctx.user_functions.get(&crate::closures::closure_call_name(arity)).cloned();
				func.instruction(&I::LocalGet(closure_local));
				func.instruction(&I::StructGet { struct_type_index: closure, field_index: ENTRY_FIELD });
				func.instruction(&I::RefTestNonNull(HeapType::Concrete(entry)));
				func.instruction(&I::If(BlockType::Empty));
				func.instruction(&I::LocalGet(closure_local));
				func.instruction(&I::StructGet { struct_type_index: closure, field_index: CAPTURED_FIELD });
				for index in 0..arity {
					s.emit_next_argument(func, arguments, item);
					if let Some(helper) = helper.as_ref().filter(|_| typed) {
						func.instruction(&I::RefAsNonNull);
						s.emit_node_as_kind(func, param_kind(&helper.params[1 + index]));
					}
				}
				func.instruction(&I::LocalGet(closure_local));
				func.instruction(&I::StructGet { struct_type_index: closure, field_index: ENTRY_FIELD });
				func.instruction(&I::RefCastNonNull(HeapType::Concrete(entry)));
				func.instruction(&I::CallRef(entry));
				if let Some(helper) = helper.filter(|_| typed) {
					s.emit_primitive_as_node(func, helper.return_kind);
				}
				func.instruction(&I::Return);
				func.instruction(&I::End);
			}
			s.emit_runtime_error(func, WRONG_ARGUMENT_COUNT);
		});
	}

	/// The first item of the list in local `arguments` (ø when it has none), the list advanced to its rest
	fn emit_next_argument(&mut self, func: &mut Function, arguments: u32, item: u32) {
		let node_type = self.type_manager.node_type;
		let nullable_node = BlockType::Result(self.nullable_node());
		let field_or_null = |field_index| {
			[I::LocalGet(arguments), I::RefIsNull, I::If(nullable_node), I::RefNull(HeapType::Concrete(node_type)), I::Else,
				I::LocalGet(arguments), I::StructGet { struct_type_index: node_type, field_index }, I::RefCastNullable(HeapType::Concrete(node_type)), I::End]
		};
		Self::emit_list(func, &field_or_null(NODE_DATA_FIELD));
		Self::emit_list(func, &[I::LocalTee(item), I::RefIsNull, I::If(nullable_node)]);
		self.emit_call(func, "new_empty");
		Self::emit_list(func, &[I::Else, I::LocalGet(item), I::End]);
		Self::emit_list(func, &field_or_null(NODE_VALUE_FIELD));
		func.instruction(&I::LocalSet(arguments));
	}

	/// closure_rebuild(name, captured): the closure of the target of that name with those captured values, ø for none;
	/// closure_captured(f): the captured values of a closure (its entry stays in the instance)
	fn compile_closure_carriers(&mut self) {
		if self.closures.carriers.is_none() {
			return;
		}
		let closure = self.closure_type();
		let node_type = self.type_manager.node_type;
		self.function_body(vec![], |s, rebuild| {
			for (target, entry, _, _) in s.closures.entry_functions.clone() {
				rebuild.instruction(&I::LocalGet(0));
				s.emit_string_call(rebuild, &target, "new_symbol");
				s.emit_call(rebuild, super::equality::VALUES_EQUAL);
				rebuild.instruction(&I::If(BlockType::Empty));
				s.emit_kind(rebuild, Kind::Function);
				Self::emit_list(rebuild, &[I::RefFunc(entry), I::LocalGet(1), I::StructNew(closure), I::LocalGet(0), I::StructNew(node_type), I::Return, I::End]);
			}
			s.emit_call(rebuild, "new_empty");
		});
		self.function_body(vec![], |_, captured| Self::emit_list(captured, &[
			I::LocalGet(0), I::RefAsNonNull, I::StructGet { struct_type_index: node_type, field_index: NODE_DATA_FIELD },
			I::RefCastNonNull(HeapType::Concrete(closure)), I::StructGet { struct_type_index: closure, field_index: CAPTURED_FIELD },
		]));
	}

	/// Pass 2: the entry bodies, in the order of their registration
	pub(super) fn compile_closure_entries(&mut self) {
		for (target, _, captured, arity) in self.closures.entry_functions.clone() {
			let function = self.ctx.user_functions[&target].clone();
			let typed = self.typed_entry(arity);
			self.function_body(vec![], |s, func| {
				s.emit_nested_captures_restored(func, &target);
				for (index, param) in function.params.iter().enumerate() {
					if index < captured {
						func.instruction(&I::LocalGet(0));
						s.emit_nth_cell_data(func, index);
					} else if typed {
						func.instruction(&I::LocalGet((1 + index - captured) as u32)); // an i64 or f64 already
						continue;
					} else {
						func.instruction(&I::LocalGet((1 + index - captured) as u32));
						func.instruction(&I::RefAsNonNull);
					}
					s.emit_node_as_kind(func, param_kind(param));
				}
				s.emit_call_user_function(func, &function);
				if !function.return_kind.is_ref() && !typed {
					s.emit_primitive_as_node(func, function.return_kind);
				}
			});
			debug_assert!(arity + captured == function.params.len());
		}
		self.compile_closure_carriers();
		self.compile_closure_apply();
	}

	/// A nested function `outer·inner` as a value carries the values it captured where the value is made (each
	/// `make()` its own `k`): its capture globals, refreshed, as names the closure's captured list reads
	fn nested_captures_now(&mut self, func: &mut Function, target: &str) -> Vec<Node> {
		if !self.ctx.enclosing_functions.contains_key(target) {
			return vec![];
		}
		self.refresh_enclosing_captures(func);
		let captures = self.ctx.captures.get(target).cloned().unwrap_or_default();
		captures.into_iter().map(|(_, (global, kind))| {
			let name = format!("{CAPTURE_VALUE_PREFIX}{global}");
			self.ctx.user_globals.insert(name.clone(), (global, kind));
			Node::Symbol(name)
		}).collect()
	}

	/// The cons list on the stack → its `index`-th (0-based) item: the data of that cell
	fn emit_nth_cell_data(&self, func: &mut Function, index: usize) {
		let node_type = self.type_manager.node_type;
		for _ in 0..index {
			func.instruction(&I::StructGet { struct_type_index: node_type, field_index: NODE_VALUE_FIELD });
		}
		func.instruction(&I::StructGet { struct_type_index: node_type, field_index: NODE_DATA_FIELD });
		func.instruction(&I::RefCastNonNull(HeapType::Concrete(node_type)));
	}

	/// The entry of a nested function's closure: its capture globals set from the closure's captured values first
	fn emit_nested_captures_restored(&mut self, func: &mut Function, target: &str) {
		if !self.ctx.enclosing_functions.contains_key(target) {
			return;
		}
		let captures = self.ctx.captures.get(target).cloned().unwrap_or_default();
		for (index, (_, (global, kind))) in captures.into_iter().enumerate() {
			func.instruction(&I::LocalGet(0));
			self.emit_nth_cell_data(func, index);
			self.emit_node_as_kind(func, kind);
			func.instruction(&I::GlobalSet(global));
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
		self.function_body(vec![Ref(RefType { nullable: true, heap_type: HeapType::Concrete(closure) })], |s, func| {
			func.instruction(&I::LocalGet(0));
			func.instruction(&I::StructGet { struct_type_index: s.type_manager.node_type, field_index: NODE_DATA_FIELD });
			func.instruction(&I::RefTestNonNull(HeapType::Concrete(closure)));
			s.emit_fail_unless(func, NOT_A_FUNCTION);
			func.instruction(&I::LocalGet(0));
			func.instruction(&I::StructGet { struct_type_index: s.type_manager.node_type, field_index: NODE_DATA_FIELD });
			func.instruction(&I::RefCastNonNull(HeapType::Concrete(closure)));
			func.instruction(&I::LocalTee(closure_local));
			func.instruction(&I::StructGet { struct_type_index: closure, field_index: ENTRY_FIELD });
			func.instruction(&I::RefTestNonNull(HeapType::Concrete(entry)));
			s.emit_fail_unless(func, WRONG_ARGUMENT_COUNT);
			func.instruction(&I::LocalGet(closure_local));
			func.instruction(&I::StructGet { struct_type_index: closure, field_index: CAPTURED_FIELD });
			for argument in 1..=arity {
				func.instruction(&I::LocalGet(argument as u32));
			}
			func.instruction(&I::LocalGet(closure_local));
			func.instruction(&I::StructGet { struct_type_index: closure, field_index: ENTRY_FIELD });
			func.instruction(&I::RefCastNonNull(HeapType::Concrete(entry)));
			func.instruction(&I::CallRef(entry));
			if !s.typed_entry(arity) {
				func.instruction(&I::RefAsNonNull);
				s.emit_node_as_kind(func, function.return_kind);
			}
		});
		let index = function.func_index.expect("registered in pass 1");
		self.exports.export(name, ExportKind::Func, index);
	}

	/// `closure_call_n(h, a1…an)` where the variable h only ever holds closures of one target: that target's entry called
	/// directly with h's captured values, no closure or arity test and no call_ref (what closure_call_n does)
	pub(super) fn emit_direct_closure_call(&mut self, func: &mut Function, helper: &UserFunctionDef, args: &[Node]) -> bool {
		let Some(arity) = closure_call_arity(&helper.name) else { return false };
		let Some((callee, values)) = args.split_first() else { return false };
		let Some((target, entry, captured, _)) = self.single_closure_target(callee, arity) else { return false };
		if self.emit_hoisted_closure_call(func, helper, callee, &target, captured, values) {
			return true;
		}
		let closure = self.closure_type();
		self.emit_node_instructions(func, callee);
		func.instruction(&I::StructGet { struct_type_index: self.type_manager.node_type, field_index: NODE_DATA_FIELD });
		func.instruction(&I::RefCastNonNull(HeapType::Concrete(closure)));
		func.instruction(&I::StructGet { struct_type_index: closure, field_index: CAPTURED_FIELD });
		for (value, param) in values.iter().zip(&helper.params[1..]) {
			self.emit_value_of_kind(func, value, param_kind(param));
		}
		func.instruction(&I::Call(entry));
		if !self.typed_entry(arity) {
			func.instruction(&I::RefAsNonNull);
			self.emit_node_as_kind(func, helper.return_kind);
		}
		true
	}

	/// `closure_call_n(h, a1…an)` after `h·capture·0 = …` (lowering/closures.rs hoist_captures): the target itself called
	/// with the captured values read once and the arguments, as a plain call
	fn emit_hoisted_closure_call(&mut self, func: &mut Function, helper: &UserFunctionDef, callee: &Node, target: &str, captured: usize, values: &[Node]) -> bool {
		let Node::Symbol(variable) = callee.drop_meta() else { return false };
		// a nested function's closure restores its capture globals in its entry (emit_nested_captures_restored)
		if captured == 0 || self.ctx.enclosing_functions.contains_key(target) {
			return false;
		}
		let Some(function) = self.ctx.user_functions.get(target).cloned() else { return false };
		let Some(index) = function.func_index else { return false };
		let locals: Vec<String> = (0..captured).map(|position| crate::closures::capture_local(variable, position)).collect();
		let same_result = function.return_kind == helper.return_kind || (helper.return_kind.is_ref() && !function.return_kind.is_ref());
		if !function.tuple_kinds.is_empty() || !same_result || !locals.iter().all(|local| self.scope.lookup(local).is_some()) {
			return false;
		}
		let arguments = locals.into_iter().map(Node::Symbol).chain(values.iter().cloned());
		for (argument, param) in arguments.zip(&function.params) {
			self.emit_value_of_kind(func, &argument, param_kind(param));
		}
		func.instruction(&I::Call(index));
		if helper.return_kind.is_ref() && !function.return_kind.is_ref() {
			self.emit_primitive_as_node(func, function.return_kind);
		}
		true
	}

	/// `closure_lambda_1·captured·0(h)`: the index-th captured value of the closure h, as the kind of the target's parameter
	pub(super) fn compile_capture_reader(&mut self, name: &str) {
		let reader = self.ctx.user_functions[name].clone();
		let (_, position) = crate::closures::capture_reader(name).expect("a capture reader");
		let closure = self.closure_type();
		let node_type = self.type_manager.node_type;
		self.function_body(vec![], |s, func| {
			func.instruction(&I::LocalGet(0));
			func.instruction(&I::StructGet { struct_type_index: node_type, field_index: NODE_DATA_FIELD });
			func.instruction(&I::RefCastNonNull(HeapType::Concrete(closure)));
			func.instruction(&I::StructGet { struct_type_index: closure, field_index: CAPTURED_FIELD });
			s.emit_nth_cell_data(func, position);
			s.emit_node_as_kind(func, reader.return_kind);
		});
		self.exports.export(name, ExportKind::Func, reader.func_index.expect("registered in pass 1"));
	}

	/// The entry a closure variable always calls: one target of that arity, and no parameter of that name anywhere
	/// (closure_variable_targets joins the assignments to a name across all bodies, parameters are not among them)
	fn single_closure_target(&self, callee: &Node, arity: usize) -> Option<(String, u32, usize, usize)> {
		let Node::Symbol(variable) = callee.drop_meta() else { return None };
		let targets = self.ctx.closure_variable_targets.get(variable)?;
		let [target] = targets.iter().collect::<Vec<_>>()[..] else { return None };
		let is_parameter = self.ctx.user_functions.values().any(|function| function.params.iter().any(|param| param.name == *variable));
		if is_parameter {
			return None;
		}
		self.closures.entry_functions.iter().find(|(name, _, _, entry_arity)| name == target && *entry_arity == arity).cloned()
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
		let captured = match captured.is_empty() {
			true => self.nested_captures_now(func, target),
			false => captured.to_vec(),
		};
		self.emit_kind(func, Kind::Function);
		func.instruction(&I::RefFunc(entry_function));
		if captured.is_empty() {
			self.emit_node_null(func);
		} else {
			self.emit_node_instructions(func, &Node::List(captured.clone(), Bracket::Square, Separator::Space));
		}
		self.ctx.user_globals.retain(|name, _| !name.starts_with(CAPTURE_VALUE_PREFIX));
		func.instruction(&I::StructNew(closure));
		self.emit_string_call(func, target, "new_symbol");
		func.instruction(&I::StructNew(self.type_manager.node_type));
	}
}
