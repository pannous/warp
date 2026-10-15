//! Closures at run time (lowering in src/lowering/definitions/closures.rs, notes/closures.md):
//!   (type $Closure (struct (field $entry (ref func)) (field $captured (ref null $Node))))
//!   (type $closure_entry_n (func (param $captured (ref null $Node)) (param (ref null $Node))×n (result (ref null $Node))))
//! A closure value is a $Node of Kind::Function whose data is the $Closure and whose value is the function name as a Symbol.
//! Every closure entry takes and returns Nodes (the universal signature): `closure_entry_<target>` unboxes the captured values
//! (a list) and the arguments to the parameter kinds of the target, calls it and boxes its result. `closure_call_n(f, a1…an)`
//! casts the entry to $closure_entry_n and call_refs it, unboxing the result to the kind all closures of arity n return.

use super::super::WasmGcEmitter;
use crate::analyzer::param_kind;
use crate::closures::closure_call_arity;
use crate::compiler::context::UserFunctionDef;
use crate::node::{Bracket, Node, Separator};
use crate::type_kinds::Kind;
use std::collections::HashMap;
use wasm_encoder::*;
use Instruction as I;
use StorageType::Val;
use ValType::Ref;

/// The export name prefix of a closure's captured variable, `capture·add·k` (tasks copy them to their instance)
pub const CAPTURE_EXPORT_PREFIX: &str = "capture·";

const ENTRY_PREFIX: &str = "closure_entry_";
/// closure_rebuild(name, captured) and closure_captured(f): what a host needs to carry a closure to another instance of
/// the module (a task, src/host/tasks.rs): its target's name and its captured values; the entry is found again by name
pub const CLOSURE_REBUILD: &str = "closure_rebuild";
pub const CLOSURE_CAPTURED: &str = "closure_captured";
/// closure_apply(f, arguments): f called with the items of the list `arguments`, as many as f takes (missing ones ø,
/// extra ones dropped, as JavaScript calls a function): how a foreign runtime calls back a warp function it was given
pub const CLOSURE_APPLY: &str = "closure_apply";
/// function_text(f): how print and a list's text show a function value: a named function by its name, a lambda as
/// written (`x => x + n`) (card print-closure)
pub const FUNCTION_TEXT: &str = "function_text";
/// function_dump(f): how `print f` shows a function value on its own: its definition with the doc as a comment line
/// above it (decision reflection); in a list or a text it stays its function_text
pub const FUNCTION_DUMP: &str = "function_dump";
const FUNCTION_NAME: &str = "function_name";
const FUNCTION_DOC: &str = "function_doc";
const FUNCTION_SIGNATURE: &str = "function_signature";
const FUNCTION_PARAMS: &str = "function_params";
const FUNCTION_RETURN_TYPE: &str = "function_return_type";
const FUNCTION_DEFINITION: &str = "function_definition";
const FUNCTION_FULL: &str = "function_full";
/// function_type(f): `type(f)` of a function value known only at run time, its kind and signature as the type of a
/// named function says it (`function: (x) -> int`, `builtin: (x:number) -> int`; card type-function); ø for no function
pub const FUNCTION_TYPE: &str = "function_type";
/// The reflection words a function value answers at run time (`fs#1.doc`, `methods(math) map .doc`; card
/// module-methods), each a field read's fallback (fields_and_keys.rs FIELD_FALLBACKS): word → its table
pub const FUNCTION_WORDS: [(&str, &str); 7] = [
	("name", FUNCTION_NAME),
	(crate::lowering::reflection::PARAMS_WORDS[0], FUNCTION_PARAMS),
	("doc", FUNCTION_DOC),
	("signature", FUNCTION_SIGNATURE),
	("return_type", FUNCTION_RETURN_TYPE),
	("dump", FUNCTION_DEFINITION),
	("full", FUNCTION_FULL),
];
const FUNCTION_FILE: &str = "function_file";
const FUNCTION_LINE: &str = "function_line";
const FUNCTION_COLUMN: &str = "function_column";
const FUNCTION_ORIGIN: &str = "function_origin";
const FUNCTION_COMPILER: &str = "function_compiler";
const FUNCTION_WASM_NAME: &str = "function_wasm_name";
/// What a function value answers besides in reflection mode (`use reflection`, decision reflection): where it was
/// written, the module it comes from, what compiled it, its function's name in the wasm module (no index: tree
/// shaking renumbers the functions after the emitter, card use-reflection)
const REFLECTION_MODE_WORDS: [(&str, &str); 6] = [
	("file", FUNCTION_FILE),
	("line", FUNCTION_LINE),
	("column", FUNCTION_COLUMN),
	("origin", FUNCTION_ORIGIN),
	("compiler", FUNCTION_COMPILER),
	("wasm_name", FUNCTION_WASM_NAME),
];
/// The origin of a function the program defines itself
const PROGRAM_ORIGIN: &str = "program";

/// The reflection words a function value answers in this compile: REFLECTION_MODE_WORDS too in reflection mode
pub fn function_words() -> impl Iterator<Item = (&'static str, &'static str)> {
	let reflection_mode = crate::compiler::compile_context::read(|context| context.reflection_mode);
	FUNCTION_WORDS.into_iter().chain(REFLECTION_MODE_WORDS.into_iter().filter(move |_| reflection_mode))
}

/// What compiles the program: warp of this version
fn compiler() -> String {
	format!("warp {}", env!("CARGO_PKG_VERSION"))
}

/// The file a function was written in: its module's, else the program's (none for inline code)
fn written_in(at: Option<&crate::compiler::meta::LineInfo>) -> Option<String> {
	at.and_then(|at| at.file.clone()).or_else(|| {
		crate::compiler::compile_context::read(|context| context.program_file.clone())
			.map(|file| file.display().to_string())
	})
}

/// Where a function was written (doc_strings.rs, in reflection mode)
fn written_at(function: &UserFunctionDef) -> Option<crate::compiler::meta::LineInfo> {
	crate::lowering::doc_strings::written_at(&function.body)
}

/// What a function table gives a value it has no text for
#[derive(Clone, Copy, PartialEq)]
enum Miss {
	/// the text `function`: function_text and function_dump, called on function values only
	KindName,
	/// ø: a field read's fallback, then the missing field's error
	Null,
}

/// Runtime errors of a closure call: the value called is no closure, or a closure of another arity
pub(in crate::wasm_emitter) const NOT_A_FUNCTION: &str = "not_a_function";
pub(in crate::wasm_emitter) const WRONG_ARGUMENT_COUNT: &str = "wrong_number_of_arguments";
const ENTRY_FIELD: u32 = 0;
/// The name a capture global is read by while a nested function's closure is made (nested_captures_now)
const CAPTURE_VALUE_PREFIX: &str = "·capture·";
const CAPTURED_FIELD: u32 = 1;
const NODE_DATA_FIELD: u32 = 1;
const NODE_VALUE_FIELD: u32 = 2;
/// A closure's kind carries the class of its target's written form above the kind bits, so == compares definitions
/// (card runtime-function); class 0: one of too many to count, compared by name
pub(in crate::wasm_emitter) const FUNCTION_CLASS_SHIFT: i64 = 8;
const FUNCTION_CLASSES: usize = (1 << (crate::type_kinds::ELEMENT_MARK_SHIFT - FUNCTION_CLASS_SHIFT)) - 1;
pub(in crate::wasm_emitter) const FUNCTION_CLASS_MASK: i64 = (FUNCTION_CLASSES as i64) << FUNCTION_CLASS_SHIFT;

/// The types and entry functions of the closures in a module
#[derive(Default, Clone)]
pub(in crate::wasm_emitter) struct ClosureTypes {
	closure: Option<u32>,
	/// Arity → $closure_entry_n
	entries: HashMap<usize, u32>,
	/// Target function → (entry function index, captured count, arity)
	entry_functions: Vec<(String, u32, usize, usize)>,
	/// closure_rebuild and closure_captured, for a program that starts tasks
	carriers: Option<(u32, u32)>,
	/// closure_apply, for a program that calls foreign code
	apply: Option<u32>,
	/// The written forms of the closure targets (function_equality::written_form), class n at n - 1
	written_forms: Vec<String>,
	/// Target function capturing nothing → the global holding its one closure, made on first use
	constants: HashMap<String, u32>,
}

impl ClosureTypes {
	/// The entry functions referenced by `ref.func`: a module must declare them in an element segment
	pub(in crate::wasm_emitter) fn declared_functions(&self) -> Vec<u32> {
		self.entry_functions.iter().map(|(_, index, _, _)| *index).collect()
	}

	pub(in crate::wasm_emitter) fn entry_names(&self) -> Vec<(u32, String)> {
		let carriers = self.carriers.into_iter().flat_map(|(rebuild, captured)| {
			[(rebuild, CLOSURE_REBUILD.to_string()), (captured, CLOSURE_CAPTURED.to_string())]
		});
		let apply = self.apply.map(|apply| (apply, CLOSURE_APPLY.to_string()));
		self.entry_functions
			.iter()
			.map(|(target, index, _, _)| (*index, format!("{ENTRY_PREFIX}{target}")))
			.chain(carriers)
			.chain(apply)
			.collect()
	}
}

impl WasmGcEmitter {
	fn nullable_node(&self) -> ValType {
		Ref(self.node_ref(true))
	}

	/// The kind of a closure of `target`: Function with the class of its written form
	fn function_kind(&mut self, target: &str) -> i64 {
		let class = self.ctx.user_functions.get(target).map_or(0, |function| {
			let parameters: Vec<_> =
				function.params.iter().map(|param| (param.name.clone(), param.annotation.clone())).collect();
			let form = crate::compiler::function_equality::written_form(target, &parameters, &function.body);
			let forms = &mut self.closures.written_forms;
			let index = forms.iter().position(|known| *known == form).unwrap_or_else(|| {
				forms.push(form);
				forms.len() - 1
			});
			if index < FUNCTION_CLASSES {
				index + 1
			} else {
				0
			}
		});
		let builtin =
			if crate::closures::is_builtin_function(target) { crate::type_kinds::BUILTIN_FUNCTION_MARK } else { 0 };
		builtin | ((class as i64) << FUNCTION_CLASS_SHIFT) | Kind::Function as i64
	}

	fn closure_type(&mut self) -> u32 {
		if let Some(closure) = self.closures.closure {
			return closure;
		}
		let index = self.type_manager.types().len();
		let entry = FieldType {
			element_type: Val(Ref(RefType { nullable: false, heap_type: HeapType::FUNC })),
			mutable: false,
		};
		let captured = FieldType { element_type: Val(self.nullable_node()), mutable: false };
		self.type_manager.types_mut().ty().struct_(vec![entry, captured]);
		self.closures.closure = Some(index);
		index
	}

	/// Do the closures of `arity` take and return Ints (closures::type_closure_calls): their entries skip the Node boxes
	fn typed_entry(&self, arity: usize) -> bool {
		self.ctx
			.user_functions
			.get(&crate::closures::closure_call_name(arity))
			.is_some_and(crate::closures::is_typed_closure_call)
	}

	fn entry_type(&mut self, arity: usize) -> u32 {
		if let Some(entry) = self.closures.entries.get(&arity) {
			return *entry;
		}
		let node = self.nullable_node();
		let index = if self.typed_entry(arity) {
			let helper = &self.ctx.user_functions[&crate::closures::closure_call_name(arity)];
			let number = |kind: Kind| if kind.is_float() { ValType::F64 } else { ValType::I64 };
			let params = std::iter::once(node)
				.chain(helper.params[1..].iter().map(|param| number(param_kind(param))))
				.collect::<Vec<_>>();
			let result = number(helper.return_kind);
			self.type_manager.function_type(params, vec![result])
		} else {
			self.type_manager.function_type(vec![node; arity + 1], vec![node])
		};
		self.closures.entries.insert(arity, index);
		index
	}

	/// Pass 1 of compile_user_functions: a function index for the entry of every closure target
	pub(in crate::wasm_emitter) fn register_closure_entries(&mut self) {
		for (target, captured) in self.ctx.closure_targets.clone() {
			let Some(function) = self.ctx.user_functions.get(&target) else {
				continue;
			};
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
		self.function_body(
			vec![Ref(RefType { nullable: true, heap_type: HeapType::Concrete(closure) }), nullable_node],
			|s, func| {
				#[rustfmt::skip]
				Self::emit_list(func, &[
					I::LocalGet(0), I::RefAsNonNull,
					I::StructGet { struct_type_index: node_type, field_index: NODE_DATA_FIELD },
					I::RefTestNonNull(HeapType::Concrete(closure)),
				]);
				s.emit_fail_unless(func, NOT_A_FUNCTION);
				#[rustfmt::skip]
				Self::emit_list(func, &[
					I::LocalGet(0), I::StructGet { struct_type_index: node_type, field_index: NODE_DATA_FIELD },
					I::RefCastNonNull(HeapType::Concrete(closure)), I::LocalSet(closure_local),
				]);
				let mut arities = s.closures.entries.keys().copied().collect::<Vec<_>>();
				arities.sort();
				for arity in arities {
					let entry = s.entry_type(arity);
					let typed = s.typed_entry(arity);
					let helper = s.ctx.user_functions.get(&crate::closures::closure_call_name(arity)).cloned();
					s.emit_closure_entry(func, closure_local, closure);
					#[rustfmt::skip]
					Self::emit_list(func, &[
						I::RefTestNonNull(HeapType::Concrete(entry)), I::If(BlockType::Empty),
							I::LocalGet(closure_local),
							I::StructGet { struct_type_index: closure, field_index: CAPTURED_FIELD },
					]);
					for index in 0..arity {
						s.emit_next_argument(func, arguments, item);
						if let Some(helper) = helper.as_ref().filter(|_| typed) {
							func.instruction(&I::RefAsNonNull);
							s.emit_node_as_kind(func, param_kind(&helper.params[1 + index]));
						}
					}
					s.emit_closure_entry(func, closure_local, closure);
					Self::emit_list(func, &[I::RefCastNonNull(HeapType::Concrete(entry)), I::CallRef(entry)]);
					if let Some(helper) = helper.filter(|_| typed) {
						s.emit_primitive_as_node(func, helper.return_kind);
					}
					Self::emit_list(func, &[I::Return, I::End]);
				}
				s.emit_runtime_error(func, WRONG_ARGUMENT_COUNT);
			},
		);
	}

	fn emit_closure_entry(&self, func: &mut Function, closure: u32, closure_type: u32) {
		Self::emit_list(
			func,
			&[I::LocalGet(closure), I::StructGet { struct_type_index: closure_type, field_index: ENTRY_FIELD }],
		);
	}

	/// The first item of the list in local `arguments` (ø when it has none), the list advanced to its rest
	fn emit_next_argument(&mut self, func: &mut Function, arguments: u32, item: u32) {
		let node_type = self.type_manager.node_type;
		let nullable_node = BlockType::Result(self.nullable_node());
		let field_or_null = |field_index| {
			[
				I::LocalGet(arguments),
				I::RefIsNull,
				I::If(nullable_node),
				I::RefNull(HeapType::Concrete(node_type)),
				I::Else,
				I::LocalGet(arguments),
				I::StructGet { struct_type_index: node_type, field_index },
				I::RefCastNullable(HeapType::Concrete(node_type)),
				I::End,
			]
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
				s.emit_call(rebuild, super::super::reflection::equality::VALUES_EQUAL);
				rebuild.instruction(&I::If(BlockType::Empty));
				Self::emit_list(
					rebuild,
					&[I::I64Const(s.function_kind(&target)), I::RefFunc(entry), I::LocalGet(1), I::StructNew(closure)],
				);
				// the name from this module's string table, as closure_new makes it (function_text finds it there)
				s.emit_string_call(rebuild, &target, "new_symbol");
				Self::emit_list(rebuild, &[I::StructNew(node_type), I::Return, I::End]);
			}
			s.emit_call(rebuild, "new_empty");
		});
		self.function_body(vec![], |_, captured| {
			Self::emit_list(
				captured,
				&[
					I::LocalGet(0),
					I::RefAsNonNull,
					I::StructGet { struct_type_index: node_type, field_index: NODE_DATA_FIELD },
					I::RefCastNonNull(HeapType::Concrete(closure)),
					I::StructGet { struct_type_index: closure, field_index: CAPTURED_FIELD },
				],
			)
		});
	}

	/// Pass 2: the entry bodies, in the order of their registration
	pub(in crate::wasm_emitter) fn compile_closure_entries(&mut self) {
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
						Self::emit_list(func, &[I::LocalGet((1 + index - captured) as u32), I::RefAsNonNull]);
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
		captures
			.into_iter()
			.map(|(_, (global, kind))| {
				let name = format!("{CAPTURE_VALUE_PREFIX}{global}");
				self.ctx.user_globals.insert(name.clone(), (global, kind));
				Node::Symbol(name)
			})
			.collect()
	}

	/// The cons list on the stack → its `index`-th (0-based) item: the data of that cell
	fn emit_nth_cell_data(&self, func: &mut Function, index: usize) {
		let node_type = self.type_manager.node_type;
		for _ in 0..index {
			func.instruction(&I::StructGet { struct_type_index: node_type, field_index: NODE_VALUE_FIELD });
		}
		Self::emit_list(
			func,
			&[
				I::StructGet { struct_type_index: node_type, field_index: NODE_DATA_FIELD },
				I::RefCastNonNull(HeapType::Concrete(node_type)),
			],
		);
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
			Self::emit_list(
				func,
				&[
					I::StructGet { struct_type_index: self.type_manager.node_type, field_index: NODE_DATA_FIELD },
					I::RefCastNonNull(HeapType::Concrete(self.type_manager.f64_box_type)),
					I::StructGet { struct_type_index: self.type_manager.f64_box_type, field_index: 0 },
				],
			);
		} else if kind == Kind::Codepoint {
			self.emit_codepoint_of_node(func);
		} else {
			self.emit_call(func, "get_int_value");
		}
	}

	/// Stack [Codepoint node] → [i64 code point]
	pub(in crate::wasm_emitter) fn emit_codepoint_of_node(&self, func: &mut Function) {
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::StructGet { struct_type_index: self.type_manager.node_type, field_index: NODE_DATA_FIELD },
			I::RefCastNonNull(HeapType::I31), I::I31GetU, I::I64ExtendI32U,
		]);
	}

	/// Is `name` a helper `closure_call_n` whose body the emitter writes
	pub(in crate::wasm_emitter) fn is_closure_call(&self, name: &str) -> bool {
		closure_call_arity(name).is_some()
			&& self.ctx.user_functions.get(name).is_some_and(|function| function.body.is_empty())
	}

	/// `closure_call_n(f, a1…an)`: the entry of f's $Closure, cast to its arity, called with the captured values and the arguments
	pub(in crate::wasm_emitter) fn compile_closure_call(&mut self, name: &str) {
		let function = self.ctx.user_functions[name].clone();
		let arity = closure_call_arity(name).expect("a closure call");
		let (closure, entry) = (self.closure_type(), self.entry_type(arity));
		let closure_local = (arity + 1) as u32;
		self.function_body(vec![Ref(RefType { nullable: true, heap_type: HeapType::Concrete(closure) })], |s, func| {
			Self::emit_list(
				func,
				&[
					I::LocalGet(0),
					I::StructGet { struct_type_index: s.type_manager.node_type, field_index: NODE_DATA_FIELD },
					I::RefTestNonNull(HeapType::Concrete(closure)),
				],
			);
			s.emit_fail_unless(func, NOT_A_FUNCTION);
			#[rustfmt::skip]
			Self::emit_list(func, &[
				I::LocalGet(0),
				I::StructGet { struct_type_index: s.type_manager.node_type, field_index: NODE_DATA_FIELD },
				I::RefCastNonNull(HeapType::Concrete(closure)), I::LocalSet(closure_local),
			]);
			s.emit_closure_entry(func, closure_local, closure);
			func.instruction(&I::RefTestNonNull(HeapType::Concrete(entry)));
			s.emit_fail_unless(func, WRONG_ARGUMENT_COUNT);
			Self::emit_list(
				func,
				&[I::LocalGet(closure_local), I::StructGet { struct_type_index: closure, field_index: CAPTURED_FIELD }],
			);
			for argument in 1..=arity {
				func.instruction(&I::LocalGet(argument as u32));
			}
			s.emit_closure_entry(func, closure_local, closure);
			Self::emit_list(func, &[I::RefCastNonNull(HeapType::Concrete(entry)), I::CallRef(entry)]);
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
	pub(in crate::wasm_emitter) fn emit_direct_closure_call(
		&mut self,
		func: &mut Function,
		helper: &UserFunctionDef,
		args: &[Node],
	) -> bool {
		let Some(arity) = closure_call_arity(&helper.name) else {
			return false;
		};
		let Some((callee, values)) = args.split_first() else {
			return false;
		};
		let Some((target, entry, captured, _)) = self.single_closure_target(callee, arity) else {
			return false;
		};
		if self.emit_hoisted_closure_call(func, helper, callee, &target, captured, values) {
			return true;
		}
		let closure = self.closure_type();
		self.emit_node_instructions(func, callee);
		Self::emit_list(
			func,
			&[
				I::StructGet { struct_type_index: self.type_manager.node_type, field_index: NODE_DATA_FIELD },
				I::RefCastNonNull(HeapType::Concrete(closure)),
				I::StructGet { struct_type_index: closure, field_index: CAPTURED_FIELD },
			],
		);
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

	/// `closure_call_n(h, a1…an)` after `h·capture·0 = …` (lowering/definitions/closures.rs hoist_captures): the target itself called
	/// with the captured values read once and the arguments, as a plain call
	fn emit_hoisted_closure_call(
		&mut self,
		func: &mut Function,
		helper: &UserFunctionDef,
		callee: &Node,
		target: &str,
		captured: usize,
		values: &[Node],
	) -> bool {
		let Node::Symbol(variable) = callee.drop_meta() else {
			return false;
		};
		// a nested function's closure restores its capture globals in its entry (emit_nested_captures_restored)
		if captured == 0 || self.ctx.enclosing_functions.contains_key(target) {
			return false;
		}
		let Some(function) = self.ctx.user_functions.get(target).cloned() else {
			return false;
		};
		let Some(index) = function.func_index else { return false };
		let locals: Vec<String> =
			(0..captured).map(|position| crate::closures::capture_local(variable, position)).collect();
		let same_result = function.return_kind == helper.return_kind
			|| (helper.return_kind.is_ref() && !function.return_kind.is_ref());
		if !function.tuple_kinds.is_empty()
			|| !same_result
			|| !locals.iter().all(|local| self.scope.lookup(local).is_some())
		{
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
	pub(in crate::wasm_emitter) fn compile_capture_reader(&mut self, name: &str) {
		let reader = self.ctx.user_functions[name].clone();
		let (_, position) = crate::closures::capture_reader(name).expect("a capture reader");
		let closure = self.closure_type();
		let node_type = self.type_manager.node_type;
		self.function_body(vec![], |s, func| {
			#[rustfmt::skip]
			Self::emit_list(func, &[
				I::LocalGet(0), I::StructGet { struct_type_index: node_type, field_index: NODE_DATA_FIELD },
				I::RefCastNonNull(HeapType::Concrete(closure)),
				I::StructGet { struct_type_index: closure, field_index: CAPTURED_FIELD },
			]);
			s.emit_nth_cell_data(func, position);
			s.emit_node_as_kind(func, reader.return_kind);
		});
		self.exports.export(name, ExportKind::Func, reader.func_index.expect("registered in pass 1"));
	}

	/// The entry a closure variable always calls: one target of that arity, and no parameter of that name anywhere
	/// (closure_variable_targets joins the assignments to a name across all bodies, parameters are not among them)
	fn single_closure_target(&self, callee: &Node, arity: usize) -> Option<(String, u32, usize, usize)> {
		let Node::Symbol(variable) = callee.drop_meta() else {
			return None;
		};
		let targets = self.ctx.closure_variable_targets.get(variable)?;
		let [target] = targets.iter().collect::<Vec<_>>()[..] else {
			return None;
		};
		let is_parameter = self
			.ctx
			.user_functions
			.values()
			.any(|function| function.params.iter().any(|param| param.name == *variable));
		if is_parameter {
			return None;
		}
		self.closures
			.entry_functions
			.iter()
			.find(|(name, _, _, entry_arity)| name == target && *entry_arity == arity)
			.cloned()
	}

	/// The runtime error `error` unless the i32 on the stack is true
	fn emit_fail_unless(&mut self, func: &mut Function, error: &'static str) {
		Self::emit_list(func, &[I::I32Eqz, I::If(BlockType::Empty)]);
		self.emit_runtime_error(func, error);
		func.instruction(&I::End);
	}

	/// `closure_new(target, captured…)`: the Node holding the $Closure of the target's entry and the captured values
	pub(in crate::wasm_emitter) fn emit_closure_new(&mut self, func: &mut Function, target: &str, captured: &[Node]) {
		let Some(entry_function) =
			self.closures.entry_functions.iter().find(|(name, ..)| name == target).map(|(_, index, ..)| *index)
		else {
			self.emit_type_error(func, format!("{target} is not a function value"));
			return;
		};
		let captured = match captured.is_empty() {
			true => self.nested_captures_now(func, target),
			false => captured.to_vec(),
		};
		if !captured.is_empty() {
			self.emit_new_closure(func, entry_function, target, &captured);
			return;
		}
		// a closure capturing nothing is the same value every time: made once, so a loop passing a function allocates none
		let constant = self.closure_constant(target);
		Self::emit_list(func, &[I::GlobalGet(constant), I::RefIsNull, I::If(BlockType::Empty)]);
		self.emit_new_closure(func, entry_function, target, &[]);
		Self::emit_list(func, &[I::GlobalSet(constant), I::End, I::GlobalGet(constant), I::RefAsNonNull]);
	}

	fn closure_constant(&mut self, target: &str) -> u32 {
		if let Some(global) = self.closures.constants.get(target) {
			return *global;
		}
		let node = HeapType::Concrete(self.type_manager.node_type);
		let global =
			self.declare_named_global(Ref(self.node_ref(true)), ConstExpr::ref_null(node), format!("closure·{target}"));
		self.closures.constants.insert(target.to_string(), global);
		global
	}

	fn emit_new_closure(&mut self, func: &mut Function, entry_function: u32, target: &str, captured: &[Node]) {
		let kind = self.function_kind(target);
		Self::emit_list(func, &[I::I64Const(kind), I::RefFunc(entry_function)]);
		if captured.is_empty() {
			self.emit_node_null(func);
		} else {
			self.emit_node_instructions(func, &Node::List(captured.to_vec(), Bracket::Square, Separator::Space));
		}
		self.ctx.user_globals.retain(|name, _| !name.starts_with(CAPTURE_VALUE_PREFIX));
		func.instruction(&I::StructNew(self.closure_type()));
		self.emit_string_call(func, target, "new_symbol");
		func.instruction(&I::StructNew(self.type_manager.node_type));
	}
}

impl WasmGcEmitter {
	/// In values_equal, two closures of the same kind (so of the same class, Self::function_kind): equal when they
	/// captured equal values; of class 0 only of the same function
	pub(in crate::wasm_emitter) fn emit_closures_equal(&mut self, f: &mut Function, recurse: u32) {
		let (node, closure) = (self.type_manager.node_type, self.closure_type());
		let (left, right, left_kind) = (0, 1, 2);
		#[rustfmt::skip]
		Self::emit_list(f, &[
			I::LocalGet(left_kind), I::I64Const(crate::type_kinds::KIND_MASK), I::I64And,
			I::I64Const(Kind::Function as i64), I::I64Eq, I::If(BlockType::Empty),
				I::LocalGet(left_kind), I::I64Const(FUNCTION_CLASS_MASK), I::I64And, I::I64Eqz, I::If(BlockType::Empty),
		]);
		Self::field(f, left, node, NODE_VALUE_FIELD);
		Self::field(f, right, node, NODE_VALUE_FIELD);
		Self::emit_list(f, &[I::Call(recurse), I::I32Eqz]);
		Self::return_if(f, 0);
		f.instruction(&I::End);
		for side in [left, right] {
			Self::field(f, side, node, NODE_DATA_FIELD);
			Self::emit_list(
				f,
				&[
					I::RefCastNonNull(HeapType::Concrete(closure)),
					I::StructGet { struct_type_index: closure, field_index: CAPTURED_FIELD },
				],
			);
		}
		Self::emit_list(f, &[I::Call(recurse), I::Return, I::End]);
	}
}

impl WasmGcEmitter {
	/// A program whose function values may be shown needs function_text
	pub(in crate::wasm_emitter) fn require_function_text(&mut self) {
		let required = &mut self.ctx.required_functions;
		let shows_values =
			[super::super::host::wasi_emitter::PRINT_VALUE, super::super::host::wasi_emitter::PUT_VALUE, "list_join"]
				.iter()
				.any(|name| required.contains(name));
		if shows_values && !self.ctx.closure_targets.is_empty() {
			required.extend([FUNCTION_TEXT, FUNCTION_DUMP]);
		}
	}

	/// The text of a closure target: a named function's name (without its enclosing function's), a lambda's
	/// parameters and body
	fn function_text_of(&self, target: &str, captured: usize) -> String {
		let Some(function) =
			self.ctx.user_functions.get(target).filter(|_| crate::closures::is_lambda_function(target))
		else {
			return self.function_name(target).to_string();
		};
		let params: Vec<&str> = function.params.iter().skip(captured).map(|param| param.name.as_str()).collect();
		let params = match params.as_slice() {
			[one] => one.to_string(),
			more => format!("({})", more.join(", ")),
		};
		format!("{params} => {}", function.body.serialize())
	}

	/// The name a function value shows: `sum` of a class's method that became the witness `sum·P`, the last part of a
	/// nested `outer·inner`
	fn function_name<'a>(&self, target: &'a str) -> &'a str {
		match target.rsplit_once(crate::traits::WITNESS_SEPARATOR) {
			Some((operation, class)) if self.ctx.type_registry.get_by_name(class).is_some() => operation,
			Some((_, last)) => last,
			None => target,
		}
	}

	/// Whether function_text is emitted: a program that shows values and has function values (an emit without a
	/// program, which emits everything, has none to show)
	pub(in crate::wasm_emitter) fn shows_functions(&self) -> bool {
		!self.ctx.closure_targets.is_empty() && self.should_emit_function(FUNCTION_TEXT)
	}

	pub(in crate::wasm_emitter) fn emit_function_text(&mut self) {
		if self.shows_functions() {
			let text =
				|emitter: &Self, target: &str, captured: usize| Node::Text(emitter.function_text_of(target, captured));
			self.emit_function_table(FUNCTION_TEXT, text, Miss::KindName);
		}
		if self.shows_functions() && self.should_emit_function(FUNCTION_DUMP) {
			let printed =
				|emitter: &Self, target: &str, captured: usize| Node::Text(emitter.printed_function(target, captured));
			self.emit_function_table(FUNCTION_DUMP, printed, Miss::KindName);
		}
		if self.types_functions() {
			let typed = |emitter: &Self, target: &str, _: usize| emitter.function_type(target);
			self.emit_function_table(FUNCTION_TYPE, typed, Miss::Null);
		}
		for (_, table) in function_words() {
			if self.should_emit_function(table) {
				let word =
					move |emitter: &Self, target: &str, captured: usize| emitter.function_word(table, target, captured);
				self.emit_function_table(table, word, Miss::Null);
			}
		}
	}

	/// Whether function_type is emitted: a program that asks type() and has function values
	pub(in crate::wasm_emitter) fn types_functions(&self) -> bool {
		!self.ctx.closure_targets.is_empty() && self.should_emit_function(FUNCTION_TYPE)
	}

	/// `function: (x) -> int` of a closure target, `builtin: (t:text) -> text` of a built-in's
	fn function_type(&self, target: &str) -> Node {
		use crate::lowering::reflection::{builtin_type, function_type, signature};
		match (builtin_signature(target), self.ctx.user_functions.get(target)) {
			(Some((_, signature)), _) => builtin_type(&signature),
			(None, Some(function)) => function_type(&signature(function)),
			(None, None) => Node::Text(Kind::Function.to_string()),
		}
	}

	/// `print f`: the definition, the doc as a comment line above it (decision reflection), in reflection mode a comment
	/// line saying where and how it was made; a lambda as written
	fn printed_function(&self, target: &str, captured: usize) -> String {
		// a built-in's: its word and signature, `upper(t:text) -> text`
		if let Some((word, signature)) = builtin_signature(target) {
			return format!("{word}{signature}");
		}
		match self.ctx.user_functions.get(target) {
			Some(function) if !crate::closures::is_lambda_function(target) => {
				let made = function_words()
					.any(|(_, table)| table == FUNCTION_WASM_NAME)
					.then(|| format!("# {}\n", self.made_by(target, function)));
				format!("{}{}", made.unwrap_or_default(), crate::lowering::reflection::printed(function))
			}
			_ => self.function_text_of(target, captured),
		}
	}

	/// `program:2:1 · warp 1.2.6 · wasm twice`: where a function was written, what compiled it, its wasm function
	fn made_by(&self, target: &str, function: &UserFunctionDef) -> String {
		let at = written_at(function);
		let place = written_in(at.as_ref()).unwrap_or_else(|| self.origin(target, None));
		let position = at.map(|at| format!(":{}:{}", at.line_nr, at.column)).unwrap_or_default();
		format!("{place}{position} · {} · wasm {target}", compiler())
	}

	/// The value of the reflection word `table` of a function value at run time (function_words)
	fn function_word(&self, table: &str, target: &str, captured: usize) -> Node {
		let function = self.ctx.user_functions.get(target);
		let at = function.and_then(written_at);
		let text = |text: String| Node::Text(text);
		match (table, function, builtin_signature(target)) {
			(FUNCTION_SIGNATURE, _, Some((_, signature))) => text(signature),
			(FUNCTION_PARAMS, _, Some((word, _))) => {
				let params = crate::lowering::library_signatures::library_signature(word).map(|(params, _)| params);
				crate::lowering::reflection::text_list(&params.unwrap_or_default())
			}
			(FUNCTION_PARAMS, Some(function), _) => crate::lowering::reflection::text_list(
				&function.params.iter().map(|param| param.name.clone()).collect::<Vec<_>>(),
			),
			(FUNCTION_DOC, Some(function), _) => text(function.doc.clone().unwrap_or_default()),
			(FUNCTION_SIGNATURE, Some(function), _) => text(crate::lowering::reflection::signature(function)),
			(FUNCTION_RETURN_TYPE, Some(function), _) => text(crate::lowering::reflection::result_type(function)),
			(FUNCTION_DEFINITION, Some(function), _) => text(crate::lowering::reflection::definition(function)),
			(FUNCTION_FULL, ..) => text(self.printed_function(target, captured)),
			(FUNCTION_FILE, ..) => text(written_in(at.as_ref()).unwrap_or_default()),
			(FUNCTION_LINE, ..) => Node::int(at.as_ref().map_or(0, |at| at.line_nr) as i64),
			(FUNCTION_COLUMN, ..) => Node::int(at.as_ref().map_or(0, |at| at.column) as i64),
			(FUNCTION_ORIGIN, ..) => text(self.origin(target, at.and_then(|at| at.file))),
			(FUNCTION_COMPILER, ..) => text(compiler()),
			(FUNCTION_WASM_NAME, ..) => text(target.to_string()),
			_ => text(self.function_text_of(target, captured)),
		}
	}

	/// The module a function comes from: a standard module, a module file's name, else the program
	fn origin(&self, target: &str, file: Option<String>) -> String {
		let module_file = file.map(|file| {
			std::path::Path::new(&file).file_stem().map_or(file.clone(), |stem| stem.to_string_lossy().into_owned())
		});
		crate::modules::std_module_defining(self.function_name(target))
			.map(str::to_string)
			.or(module_file)
			.unwrap_or_else(|| PROGRAM_ORIGIN.to_string())
	}

	/// A run-time table over the program's closure targets: `name(f)` is the text `text_of` gives f's target, found by
	/// the target's name in the string table (closure_new, closure_rebuild: one place each). A value that is no function
	/// or a target of another module gives `miss`
	fn emit_function_table(&mut self, name: &'static str, value_of: impl Fn(&Self, &str, usize) -> Node, miss: Miss) {
		let (node, string) = (self.type_manager.node_type, self.type_manager.string_type);
		let result = Ref(self.node_ref(miss == Miss::Null));
		let values: Vec<((u32, u32), Node)> = self
			.ctx
			.closure_targets
			.clone()
			.into_iter()
			.map(|(target, captured)| (self.allocate_string(&target), value_of(self, &target, captured)))
			.collect();
		let name_local = 1;
		let locals = vec![Ref(RefType { nullable: true, heap_type: HeapType::Concrete(string) })];
		let node_ref = Ref(self.node_ref(false));
		self.runtime_function(name, vec![node_ref], vec![result], locals, |s, f| {
			let emit_miss = |s: &mut Self, f: &mut Function| match miss {
				Miss::Null => {
					f.instruction(&I::RefNull(HeapType::Concrete(node)));
				}
				Miss::KindName => {
					s.emit_string_call(f, crate::type_kinds::Kind::Function.to_string().as_str(), "new_text")
				}
			};
			if miss == Miss::Null {
				Self::field(f, 0, node, 0);
				Self::emit_list(
					f,
					&[
						I::I64Const(crate::type_kinds::KIND_MASK),
						I::I64And,
						I::I64Const(crate::type_kinds::Kind::Function as i64),
						I::I64Ne,
						I::If(BlockType::Empty),
					],
				);
				emit_miss(s, f);
				Self::emit_list(f, &[I::Return, I::End]);
			}
			Self::field(f, 0, node, NODE_VALUE_FIELD);
			Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(node))]);
			f.instruction(&I::StructGet { struct_type_index: node, field_index: NODE_DATA_FIELD });
			Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(string)), I::LocalSet(name_local)]);
			for ((pointer, length), value) in &values {
				Self::field(f, name_local, string, 0);
				Self::emit_list(f, &[I::I32Const(*pointer as i32), I::I32Eq]);
				Self::field(f, name_local, string, 1);
				Self::emit_list(f, &[I::I32Const(*length as i32), I::I32Eq, I::I32And, I::If(BlockType::Empty)]);
				match value {
					Node::Text(text) => s.emit_string_call(f, text, "new_text"),
					value => s.emit_node_instructions(f, value),
				}
				Self::emit_list(f, &[I::Return, I::End]);
			}
			emit_miss(s, f);
		});
	}
}

/// The word and signature of a closure target made for a built-in (closures::builtin_word)
fn builtin_signature(target: &str) -> Option<(&str, String)> {
	let word = crate::closures::builtin_word(target)?;
	Some((word, crate::lowering::library_signatures::library_signature(word)?.1))
}
