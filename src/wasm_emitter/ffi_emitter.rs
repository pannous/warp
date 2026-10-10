//! FFI (Foreign Function Interface) emission - handles C library imports

use crate::node::Node;
use crate::type_kinds::Kind;
use wasm_encoder::*;
use Instruction as I;

use super::WasmGcEmitter;

impl WasmGcEmitter {
	/// Emit arguments for FFI function call with type conversion
	pub(super) fn emit_ffi_args(&mut self, func: &mut Function, args: &[Node], sig: &crate::ffi::FfiSignature) {
		// a text whose length the call leaves out crosses as (pointer, length): strcmp's, a module's `const void *, size_t`
		let texts_with_lengths = if args.len() < sig.params.len() { crate::wasm_modules::texts_with_lengths(sig) } else { vec![] };
		let mut args = args.iter();
		let mut param_idx = 0;
		while param_idx < sig.params.len() {
			let arg = args.next();
			if texts_with_lengths.contains(&param_idx) {
				match arg {
					Some(arg) if self.is_string_arg(arg) => self.emit_string_ptr_len(func, arg),
					Some(arg) => {
						self.emit_numeric_value(func, arg);
						Self::emit_list(func, &[I::I32WrapI64, I::I32Const(0)]);
					}
					None => {
						Self::emit_list(func, &[I::I32Const(0), I::I32Const(0)]);
					}
				}
				param_idx += 2;
				continue;
			}
			match arg {
				Some(arg) => self.emit_ffi_arg(func, arg, &sig.params[param_idx]),
				None => self.emit_ffi_default(func, &sig.params[param_idx]),
			}
			param_idx += 1;
		}
	}

	/// Emit a single FFI argument with appropriate type conversion
	pub(super) fn emit_ffi_arg(&mut self, func: &mut Function, arg: &Node, param_type: &ValType) {
		match param_type {
			ValType::F64 => self.emit_float_value(func, arg),
			ValType::F32 => {
				self.emit_float_value(func, arg);
				func.instruction(&I::F32DemoteF64);
			}
			ValType::I64 => self.emit_numeric_value(func, arg),
			// a host word taking a value (task_spawn_values): the Node
			ValType::Ref(_) => self.emit_node_instructions(func, arg),
			ValType::I32 => {
				if self.is_string_arg(arg) {
					self.emit_string_ptr_only(func, arg);
				} else {
					self.emit_numeric_value(func, arg);
					func.instruction(&I::I32WrapI64);
				}
			}
			_ => self.emit_numeric_value(func, arg),
		}
	}

	/// Emit default value for missing FFI argument
	fn emit_ffi_default(&mut self, func: &mut Function, param_type: &ValType) {
		match param_type {
			ValType::F64 => func.instruction(&I::F64Const(Ieee64::new(0.0f64.to_bits()))),
			ValType::F32 => func.instruction(&I::F32Const(Ieee32::new(0.0f32.to_bits()))),
			ValType::I64 => func.instruction(&I::I64Const(0)),
			ValType::Ref(_) => func.instruction(&I::RefNull(HeapType::Abstract { shared: false, ty: AbstractHeapType::Any })),
			_ => func.instruction(&I::I32Const(0)),
		};
	}

	/// Emit FFI result conversion based on context
	/// None = wrap in Node, Some(Kind::Int) = raw i64, Some(Kind::Float) = raw f64
	pub(super) fn emit_ffi_result(&mut self, func: &mut Function, sig: &crate::ffi::FfiSignature, ctx: Option<Kind>) {
		let result_type = sig.results.first().copied();
		// a host word giving a value (task_await_value): the Node it built, its number where one is wanted
		if matches!(result_type, Some(ValType::Ref(_))) {
			func.instruction(&I::RefCastNonNull(HeapType::Concrete(self.type_manager.node_type)));
			// a failed adapter gives back its Error (host.rs std_call): raised here, where `try` catches it
			if [crate::host::STD_PURE, crate::host::STD_IO].contains(&sig.name) {
				let answer = self.node_scratch();
				func.instruction(&I::LocalSet(answer));
				self.emit_fail_if_error(func, answer);
				Self::emit_list(func, &[I::LocalGet(answer), I::RefAsNonNull]);
			}
			match ctx {
				None => {}
				Some(Kind::Float) => {
					self.emit_call(func, "get_int_value");
					func.instruction(&I::F64ConvertI64S);
				}
				Some(_) => self.emit_call(func, "get_int_value"),
			}
			return;
		}
		match ctx {
			None => match result_type {
				None => self.emit_call(func, "new_empty"),
				Some(ValType::F64) => self.emit_call(func, "new_float"),
				Some(ValType::F32) => {
					func.instruction(&I::F64PromoteF32);
					self.emit_call(func, "new_float");
				}
				Some(ValType::I64) => self.emit_call(func, "new_int"),
				Some(ValType::I32) => {
					func.instruction(&I::I64ExtendI32S);
					self.emit_call(func, "new_int");
				}
				_ => self.emit_call(func, "new_int"),
			},
			Some(Kind::Float) => match result_type {
				None => { func.instruction(&I::F64Const(Ieee64::new(0.0f64.to_bits()))); }
				Some(ValType::F32) => { func.instruction(&I::F64PromoteF32); }
				Some(ValType::I64) => { func.instruction(&I::F64ConvertI64S); }
				Some(ValType::I32) => {
					Self::emit_list(func, &[I::I64ExtendI32S, I::F64ConvertI64S]);
				}
				_ => {} // F64 already correct
			},
			Some(_) => match result_type { // Int or other → i64
				None => { func.instruction(&I::I64Const(0)); }
				Some(ValType::F64 | ValType::F32) => self.emit_float_in_exact_context(func, sig.name),
				Some(ValType::I32) => { func.instruction(&I::I64ExtendI32S); }
				_ => {} // I64 already correct
			},
		}
	}

	/// Emit FFI function call with automatic result handling based on context
	pub(super) fn emit_ffi_call(&mut self, func: &mut Function, fn_name: &str, args: &[Node], ctx: Option<Kind>) {
		if self.emit_import_call(func, fn_name, args, ctx) {
			return;
		}
		let sig = match self.ctx.ffi_imports.get(fn_name) {
			Some(s) => s.clone(),
			None => return,
		};
		self.emit_ffi_args(func, args, &sig);
		if let Some(idx) = self.ffi_func_index(fn_name) {
			func.instruction(&I::Call(idx));
		}
		self.emit_ffi_result(func, &sig, ctx);
	}

	/// Check if a node argument should be treated as a string
	pub(super) fn is_string_arg(&self, node: &Node) -> bool {
		match node.drop_meta() {
			Node::Text(_) | Node::Char(_) => true,
			Node::Symbol(name) if self.scope.lookup(name).is_some_and(|local| local.data_pointer > 0) => true,
			_ => self.is_runtime_text(node),
		}
	}

	/// A text (or one-character text) whose letters exist only at run time: a list element, a text variable
	pub(super) fn is_runtime_text(&self, node: &Node) -> bool {
		!matches!(node.drop_meta(), Node::Text(_) | Node::Char(_)) && matches!(self.get_type(node), Kind::Text | Kind::Codepoint)
	}

	/// The text node of a run-time text in the node scratch local, as a Text (a character becomes one)
	fn emit_runtime_text_node(&mut self, func: &mut Function, node: &Node) -> u32 {
		self.emit_node_instructions(func, node);
		self.emit_call(func, super::text_builtins::TEXT_OF);
		let text = self.node_scratch();
		func.instruction(&I::LocalSet(text));
		text
	}

	/// Emit string pointer and length for FFI calls
	pub(super) fn emit_string_ptr_len(&mut self, func: &mut Function, node: &Node) {
		if self.is_runtime_text(node) {
			let text = self.emit_runtime_text_node(func, node);
			self.emit_text_field(func, text, 0);
			self.emit_text_field(func, text, 1);
			return;
		}
		let (pointer, length) = match self.compile_time_string(node) {
			CompileTimeString::Stored(pointer, length) => (pointer, length),
			CompileTimeString::Spelled(text) => self.allocate_string(&text),
		};
		Self::emit_list(func, &[I::I32Const(pointer as i32), I::I32Const(length as i32)]);
	}

	/// Emit only string pointer for C-style FFI calls (null-terminated strings)
	pub(super) fn emit_string_ptr_only(&mut self, func: &mut Function, node: &Node) {
		if self.is_runtime_text(node) {
			self.emit_node_instructions(func, node);
			self.emit_call(func, super::text_builtins::C_STRING);
			return;
		}
		let pointer = match self.compile_time_string(node) {
			CompileTimeString::Stored(pointer, _) => pointer,
			CompileTimeString::Spelled(text) => self.allocate_string(&format!("{text}\0")).0,
		};
		func.instruction(&I::I32Const(pointer as i32));
	}

	/// The bytes of a string local, else the text a character, text, symbol or other node spells
	fn compile_time_string(&self, node: &Node) -> CompileTimeString {
		match node.drop_meta() {
			Node::Char(c) => CompileTimeString::Spelled(c.to_string()),
			Node::Text(text) => CompileTimeString::Spelled(text.clone()),
			Node::Symbol(name) => match self.scope.lookup(name) {
				Some(local) if local.data_pointer > 0 => CompileTimeString::Stored(local.data_pointer, local.data_length),
				_ => CompileTimeString::Spelled(name.clone()),
			},
			_ => CompileTimeString::Spelled(node.to_string()),
		}
	}

	/// `task·check(task_join(job), task_failure(job))` (resolve_tasks): when the joined task failed, its message becomes
	/// the trap detail of a `returned_error`, which `try` catches and the runner reports as that message. True when
	/// `items` is that check (it leaves nothing on the stack)
	pub(super) fn emit_task_check(&mut self, func: &mut Function, items: &[Node]) -> bool {
		let [word, joined, failure] = items else { return false };
		if !matches!(word.drop_meta(), Node::Symbol(name) if name == crate::host::TASK_CHECK) {
			return false;
		}
		self.emit_numeric_value(func, joined);
		Self::emit_list(func, &[I::I32WrapI64, I::If(BlockType::Empty)]);
		self.emit_trap_detail(func, failure);
		self.emit_runtime_error(func, super::list_ops::RETURNED_ERROR);
		func.instruction(&I::End);
		true
	}
}

/// A string argument known at compile time
enum CompileTimeString {
	/// pointer and length of bytes already in linear memory
	Stored(u32, u32),
	/// a text still to be allocated
	Spelled(String),
}
