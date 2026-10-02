//! WASI (WebAssembly System Interface) emission - handles system I/O

use crate::node::Node;
use crate::type_kinds::Kind;
use wasm_encoder::*;

use super::WasmGcEmitter;

const PRINT_TERMINATOR: &str = "\n";
/// print_value(x): writes the text form of x (as `x as string` has it) and a newline, yields x
pub const PRINT_VALUE: &str = "print_value";
const STDOUT: i32 = 1;
const IOVEC: MemArg = MemArg { offset: 0, align: 2, memory_index: 0 };
/// Bracket info of a square list, in the bits above the kind (see type_kinds)
const SQUARE_LIST_KIND: i64 = (1 << 8) | Kind::List as i64;

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

	/// `print x` / `print(x)`: writes x and a newline to stdout, the value is the printed value. A literal is written as
	/// compiled; any other number, text or character through its runtime text form (print_value). Lists, floats and
	/// other values have no runtime text yet: an error value.
	pub(super) fn emit_print(&mut self, func: &mut Function, value: &Node) {
		let text = match value.drop_meta() {
			Node::Number(number) => number.to_string(),
			Node::Text(text) => text.clone(),
			Node::Char(character) => character.to_string(),
			_ => match self.get_type(value) {
				Kind::Int | Kind::Text | Kind::Codepoint | Kind::Empty => {
					self.emit_node_instructions(func, value);
					self.emit_call(func, PRINT_VALUE);
					return;
				}
				kind => {
					let reason = format!("print of {} has no runtime text yet", crate::analyzer::kind_with_article(kind));
					self.emit_type_error(func, crate::diagnostic::Diagnostic::at(value, reason).to_string());
					return;
				}
			},
		};
		self.emit_wasi_puts(func, &Node::Text(format!("{text}{PRINT_TERMINATOR}")));
		func.instruction(&Instruction::Drop);
		match value.drop_meta() {
			Node::Char(_) => self.emit_node_instructions(func, &Node::Text(text)), // a text, as analyzer::held_kind has it
			_ => self.emit_node_instructions(func, value),
		}
	}

	/// print_value(x): the text of x by list_join of the one-element list [x]; locals: text
	pub(super) fn emit_print_value(&mut self) {
		let Some(fd_write) = self.ctx.func_registry.get("wasi_fd_write").map(|f| f.call_index as u32) else { return };
		if !self.should_emit_function(PRINT_VALUE) {
			return;
		}
		let (node_ref, nullable) = (ValType::Ref(self.node_ref(false)), ValType::Ref(self.node_ref(true)));
		let node_type = self.type_manager.node_type;
		let (empty, _) = self.allocate_string("");
		let newline = self.allocate_string(PRINT_TERMINATOR);
		self.runtime_function(PRINT_VALUE, vec![node_ref], vec![node_ref], vec![nullable], |s, f| {
			let (value, text) = (0, 1);
			let write = |f: &mut Function, push_address: &dyn Fn(&mut Function), push_length: &dyn Fn(&mut Function)| {
				f.instruction(&Instruction::I32Const(0));
				push_address(f);
				f.instruction(&Instruction::I32Store(IOVEC));
				f.instruction(&Instruction::I32Const(4));
				push_length(f);
				f.instruction(&Instruction::I32Store(IOVEC));
				for argument in [STDOUT, 0, 1, 8] {
					f.instruction(&Instruction::I32Const(argument));
				}
				f.instruction(&Instruction::Call(fd_write));
				f.instruction(&Instruction::Drop);
			};
			f.instruction(&Instruction::I64Const(SQUARE_LIST_KIND));
			f.instruction(&Instruction::LocalGet(value));
			f.instruction(&Instruction::RefNull(HeapType::Concrete(node_type)));
			f.instruction(&Instruction::StructNew(node_type));
			f.instruction(&Instruction::I32Const(empty as i32));
			f.instruction(&Instruction::I32Const(0));
			s.call(f, "new_text");
			s.call(f, "list_join");
			f.instruction(&Instruction::LocalSet(text));
			write(f, &|f| s.emit_text_field(f, text, 0), &|f| s.emit_text_field(f, text, 1));
			write(f, &|f| { f.instruction(&Instruction::I32Const(newline.0 as i32)); }, &|f| { f.instruction(&Instruction::I32Const(newline.1 as i32)); });
			f.instruction(&Instruction::LocalGet(value));
		});
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
