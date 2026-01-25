//! FFI (Foreign Function Interface) emission - handles C library imports

use crate::node::Node;
use crate::type_kinds::Kind;
use wasm_encoder::*;

use super::WasmGcEmitter;

impl WasmGcEmitter {
	/// Get the number of string pair arguments for a given FFI function
	pub(super) fn string_pair_arg_count(&self, fn_name: &str) -> usize {
		match fn_name {
			"strcmp" | "strncmp" => 2, // (ptr1, len1, ptr2, len2, ...)
			_ => 0,
		}
	}

	/// Emit arguments for FFI function call with type conversion
	pub(super) fn emit_ffi_args(&mut self, func: &mut Function, fn_name: &str, args: &[Node], sig: &crate::ffi::FfiSignature) {
		let string_pair_count = self.string_pair_arg_count(fn_name);
		let mut arg_idx = 0;
		let mut param_idx = 0;

		// Process string arguments that need (ptr, len) pairs
		for _ in 0..string_pair_count {
			if arg_idx < args.len() && self.is_string_arg(&args[arg_idx]) {
				self.emit_string_ptr_len(func, &args[arg_idx]);
				arg_idx += 1;
				param_idx += 2;
			} else if arg_idx < args.len() {
				self.emit_numeric_value(func, &args[arg_idx]);
				func.instruction(&Instruction::I32WrapI64);
				func.instruction(&Instruction::I32Const(0));
				arg_idx += 1;
				param_idx += 2;
			} else {
				func.instruction(&Instruction::I32Const(0));
				func.instruction(&Instruction::I32Const(0));
				param_idx += 2;
			}
		}

		// Process remaining arguments according to param types
		while param_idx < sig.params.len() {
			if arg_idx < args.len() {
				self.emit_ffi_arg(func, &args[arg_idx], &sig.params[param_idx]);
				arg_idx += 1;
			} else {
				self.emit_ffi_default(func, &sig.params[param_idx]);
			}
			param_idx += 1;
		}
	}

	/// Emit a single FFI argument with appropriate type conversion
	fn emit_ffi_arg(&mut self, func: &mut Function, arg: &Node, param_type: &ValType) {
		match param_type {
			ValType::F64 => self.emit_float_value(func, arg),
			ValType::F32 => {
				self.emit_float_value(func, arg);
				func.instruction(&Instruction::F32DemoteF64);
			}
			ValType::I64 => self.emit_numeric_value(func, arg),
			ValType::I32 => {
				if self.is_string_arg(arg) {
					self.emit_string_ptr_only(func, arg);
				} else {
					self.emit_numeric_value(func, arg);
					func.instruction(&Instruction::I32WrapI64);
				}
			}
			_ => self.emit_numeric_value(func, arg),
		}
	}

	/// Emit default value for missing FFI argument
	fn emit_ffi_default(&mut self, func: &mut Function, param_type: &ValType) {
		match param_type {
			ValType::F64 => func.instruction(&Instruction::F64Const(Ieee64::new(0.0f64.to_bits()))),
			ValType::F32 => func.instruction(&Instruction::F32Const(Ieee32::new(0.0f32.to_bits()))),
			ValType::I64 => func.instruction(&Instruction::I64Const(0)),
			_ => func.instruction(&Instruction::I32Const(0)),
		};
	}

	/// Emit FFI result conversion based on context
	/// None = wrap in Node, Some(Kind::Int) = raw i64, Some(Kind::Float) = raw f64
	pub(super) fn emit_ffi_result(&mut self, func: &mut Function, sig: &crate::ffi::FfiSignature, ctx: Option<Kind>) {
		let result_type = sig.results.first().copied();
		match ctx {
			None => match result_type {
				None => self.emit_call(func, "new_empty"),
				Some(ValType::F64) => self.emit_call(func, "new_float"),
				Some(ValType::F32) => {
					func.instruction(&Instruction::F64PromoteF32);
					self.emit_call(func, "new_float");
				}
				Some(ValType::I64) => self.emit_call(func, "new_int"),
				Some(ValType::I32) => {
					func.instruction(&Instruction::I64ExtendI32S);
					self.emit_call(func, "new_int");
				}
				_ => self.emit_call(func, "new_int"),
			},
			Some(Kind::Float) => match result_type {
				None => { func.instruction(&Instruction::F64Const(Ieee64::new(0.0f64.to_bits()))); }
				Some(ValType::F32) => { func.instruction(&Instruction::F64PromoteF32); }
				Some(ValType::I64) => { func.instruction(&Instruction::F64ConvertI64S); }
				Some(ValType::I32) => {
					func.instruction(&Instruction::I64ExtendI32S);
					func.instruction(&Instruction::F64ConvertI64S);
				}
				_ => {} // F64 already correct
			},
			Some(_) => match result_type { // Int or other → i64
				None => { func.instruction(&Instruction::I64Const(0)); }
				Some(ValType::F64) => { func.instruction(&Instruction::I64TruncF64S); }
				Some(ValType::F32) => {
					func.instruction(&Instruction::F64PromoteF32);
					func.instruction(&Instruction::I64TruncF64S);
				}
				Some(ValType::I32) => { func.instruction(&Instruction::I64ExtendI32S); }
				_ => {} // I64 already correct
			},
		}
	}

	/// Emit FFI function call with automatic result handling based on context
	pub(super) fn emit_ffi_call(&mut self, func: &mut Function, fn_name: &str, args: &[Node], ctx: Option<Kind>) {
		let sig = match self.ctx.ffi_imports.get(fn_name) {
			Some(s) => s.clone(),
			None => return,
		};
		self.emit_ffi_args(func, fn_name, args, &sig);
		if let Some(idx) = self.ffi_func_index(fn_name) {
			func.instruction(&Instruction::Call(idx));
		}
		self.emit_ffi_result(func, &sig, ctx);
	}

	/// Check if a node argument should be treated as a string
	pub(super) fn is_string_arg(&self, node: &Node) -> bool {
		match node.drop_meta() {
			Node::Text(_) => true,
			Node::Symbol(name) => {
				if let Some(local) = self.scope.lookup(name) {
					local.data_pointer > 0 // Has string data stored
				} else {
					false
				}
			}
			_ => false,
		}
	}

	/// Emit string pointer and length for FFI calls
	pub(super) fn emit_string_ptr_len(&mut self, func: &mut Function, node: &Node) {
		match node.drop_meta() {
			Node::Text(s) => {
				let (ptr, len) = self.allocate_string(s);
				func.instruction(&Instruction::I32Const(ptr as i32));
				func.instruction(&Instruction::I32Const(len as i32));
			}
			Node::Symbol(name) => {
				if let Some(local) = self.scope.lookup(name) {
					if local.data_pointer > 0 {
						func.instruction(&Instruction::I32Const(local.data_pointer as i32));
						func.instruction(&Instruction::I32Const(local.data_length as i32));
					} else {
						// Fallback: use symbol name
						let (ptr, len) = self.allocate_string(name);
						func.instruction(&Instruction::I32Const(ptr as i32));
						func.instruction(&Instruction::I32Const(len as i32));
					}
				} else {
					// Unknown symbol - use name as string
					let (ptr, len) = self.allocate_string(name);
					func.instruction(&Instruction::I32Const(ptr as i32));
					func.instruction(&Instruction::I32Const(len as i32));
				}
			}
			_ => {
				// For other nodes, try to get a string representation
				let s = node.to_string();
				let (ptr, len) = self.allocate_string(&s);
				func.instruction(&Instruction::I32Const(ptr as i32));
				func.instruction(&Instruction::I32Const(len as i32));
			}
		}
	}

	/// Emit only string pointer for C-style FFI calls (null-terminated strings)
	pub(super) fn emit_string_ptr_only(&mut self, func: &mut Function, node: &Node) {
		match node.drop_meta() {
			Node::Text(s) => {
				// Add null terminator for C string
				let c_str = format!("{}\0", s);
				let (ptr, _) = self.allocate_string(&c_str);
				func.instruction(&Instruction::I32Const(ptr as i32));
			}
			Node::Symbol(name) => {
				if let Some(local) = self.scope.lookup(name) {
					if local.data_pointer > 0 {
						func.instruction(&Instruction::I32Const(local.data_pointer as i32));
					} else {
						// Fallback: use symbol name with null terminator
						let c_str = format!("{}\0", name);
						let (ptr, _) = self.allocate_string(&c_str);
						func.instruction(&Instruction::I32Const(ptr as i32));
					}
				} else {
					// Unknown symbol - use name as string
					let c_str = format!("{}\0", name);
					let (ptr, _) = self.allocate_string(&c_str);
					func.instruction(&Instruction::I32Const(ptr as i32));
				}
			}
			_ => {
				// For other nodes, try to get a string representation
				let s = node.to_string();
				let c_str = format!("{}\0", s);
				let (ptr, _) = self.allocate_string(&c_str);
				func.instruction(&Instruction::I32Const(ptr as i32));
			}
		}
	}
}
