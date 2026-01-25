//! WASI (WebAssembly System Interface) emission - handles system I/O

use crate::node::Node;
use crate::type_kinds::Kind;
use wasm_encoder::*;

use super::WasmGcEmitter;

impl WasmGcEmitter {
	/// Emit WASI puts: write string to stdout
	/// Memory layout: [0-3]: buf_ptr, [4-7]: buf_len, [8-11]: nwritten
	pub(super) fn emit_wasi_puts(&mut self, func: &mut Function, arg: &Node) {
		// Get string data pointer and length
		let (str_ptr, str_len) = match arg.drop_meta() {
			Node::Text(s) => self.allocate_string(s),
			Node::Symbol(var_name) => {
				// Check if this is a string variable with stored data
				if let Some(local) = self.scope.lookup(var_name) {
					if local.data_pointer > 0 {
						(local.data_pointer, local.data_length)
					} else {
						// Fallback: use the symbol name itself
						self.allocate_string(var_name)
					}
				} else {
					self.allocate_string(var_name)
				}
			}
			_ => self.allocate_string(""),
		};

		// Set up iovec at address 0: {buf_ptr: i32, buf_len: i32}
		// i32.store at address 0 = str_ptr
		func.instruction(&Instruction::I32Const(0)); // address
		func.instruction(&Instruction::I32Const(str_ptr as i32)); // value
		func.instruction(&Instruction::I32Store(MemArg {
			offset: 0,
			align: 2,
			memory_index: 0,
		}));

		// i32.store at address 4 = str_len
		func.instruction(&Instruction::I32Const(4)); // address
		func.instruction(&Instruction::I32Const(str_len as i32)); // value
		func.instruction(&Instruction::I32Store(MemArg {
			offset: 0,
			align: 2,
			memory_index: 0,
		}));

		// Call fd_write(fd=1, iovs=0, iovs_len=1, nwritten=8)
		func.instruction(&Instruction::I32Const(1)); // fd = stdout
		func.instruction(&Instruction::I32Const(0)); // iovs ptr
		func.instruction(&Instruction::I32Const(1)); // iovs len
		func.instruction(&Instruction::I32Const(8)); // nwritten ptr

		if let Some(f) = self.ctx.func_registry.get("wasi_fd_write") {
			func.instruction(&Instruction::Call(f.call_index as u32));
		}
		// Stack now has i32 (error code), leave it for conversion to Node
	}

	/// Emit WASI puti: write integer to stdout
	/// Converts integer to string and writes via fd_write
	pub(super) fn emit_wasi_puti(&mut self, func: &mut Function, arg: &Node) {
		// For compile-time constants, we can pre-compute the string
		if let Node::Number(n) = arg.drop_meta() {
			let s = format!("{}", n); // Number implements Display
			let (str_ptr, str_len) = self.allocate_string(&s);

			// Set up iovec
			func.instruction(&Instruction::I32Const(0));
			func.instruction(&Instruction::I32Const(str_ptr as i32));
			func.instruction(&Instruction::I32Store(MemArg {
				offset: 0,
				align: 2,
				memory_index: 0,
			}));
			func.instruction(&Instruction::I32Const(4));
			func.instruction(&Instruction::I32Const(str_len as i32));
			func.instruction(&Instruction::I32Store(MemArg {
				offset: 0,
				align: 2,
				memory_index: 0,
			}));

			// Call fd_write
			func.instruction(&Instruction::I32Const(1));
			func.instruction(&Instruction::I32Const(0));
			func.instruction(&Instruction::I32Const(1));
			func.instruction(&Instruction::I32Const(8));

			if let Some(f) = self.ctx.func_registry.get("wasi_fd_write") {
				func.instruction(&Instruction::Call(f.call_index as u32));
				func.instruction(&Instruction::Drop); // Drop return value
			}
		}
		// For runtime values, we'd need itoa - just drop for now
	}

	/// Emit WASI putf: write float to stdout
	pub(super) fn emit_wasi_putf(&mut self, func: &mut Function, arg: &Node) {
		// For compile-time constants, pre-compute the string
		if let Node::Number(n) = arg.drop_meta() {
			let s = format!("{}", n); // Number implements Display
			let (str_ptr, str_len) = self.allocate_string(&s);

			func.instruction(&Instruction::I32Const(0));
			func.instruction(&Instruction::I32Const(str_ptr as i32));
			func.instruction(&Instruction::I32Store(MemArg {
				offset: 0,
				align: 2,
				memory_index: 0,
			}));
			func.instruction(&Instruction::I32Const(4));
			func.instruction(&Instruction::I32Const(str_len as i32));
			func.instruction(&Instruction::I32Store(MemArg {
				offset: 0,
				align: 2,
				memory_index: 0,
			}));

			func.instruction(&Instruction::I32Const(1));
			func.instruction(&Instruction::I32Const(0));
			func.instruction(&Instruction::I32Const(1));
			func.instruction(&Instruction::I32Const(8));

			if let Some(f) = self.ctx.func_registry.get("wasi_fd_write") {
				func.instruction(&Instruction::Call(f.call_index as u32));
			}
		} else {
			// Return 0 for non-constant
			func.instruction(&Instruction::I32Const(0));
		}
	}

	/// Emit fd_write call - auto-detects string arguments and sets up iovec
	pub(super) fn emit_wasi_fd_write_call(&mut self, func: &mut Function, args: &[Node]) {
		// fd_write(fd, iovs_ptr, iovs_len, nwritten_ptr) -> i32
		// Check if second argument is a string (variable or literal)
		// If so, use the puts mechanism to set up iovec automatically
		let second_arg = args.get(1).map(|n| n.drop_meta());
		let is_string_arg = match &second_arg {
			Some(Node::Text(_)) => true,
			Some(Node::Symbol(s)) => {
				// Check if symbol refers to a string variable
				if let Some(local) = self.scope.lookup(s) {
					local.kind == Kind::Text
				} else {
					false
				}
			}
			_ => false,
		};

		if is_string_arg && args.len() >= 4 {
			// Use puts mechanism: set up iovec from string
			self.emit_wasi_puts(func, &args[1]);
			// fd_write already called by emit_wasi_puts, just extend to i64
			func.instruction(&Instruction::I64ExtendI32S);
		} else {
			// Raw numeric mode
			for arg in args.iter().take(4) {
				self.emit_numeric_value(func, arg);
				func.instruction(&Instruction::I32WrapI64);
			}
			if let Some(f) = self.ctx.func_registry.get("wasi_fd_write") {
				func.instruction(&Instruction::Call(f.call_index as u32));
				func.instruction(&Instruction::I64ExtendI32S);
			}
		}
	}
}
