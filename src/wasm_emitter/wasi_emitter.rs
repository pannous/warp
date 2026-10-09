//! WASI (WebAssembly System Interface) emission - handles system I/O

use crate::node::Node;
use crate::type_kinds::Kind;
use wasm_encoder::*;
use Instruction as I;

use super::WasmGcEmitter;
use crate::type_kinds::SQUARE_LIST_KIND;
use crate::wasm_emitter::layout::WORD;

const PRINT_TERMINATOR: &str = "\n";
/// print_value(x): writes the text form of x (as `x as string` has it) and a newline, yields x
pub const PRINT_VALUE: &str = "print_value";
/// put_value(x): writes the text form of x without a newline (puti, putl, putf of a run-time value), yields x
pub const PUT_VALUE: &str = "put_value";
const STDOUT: i32 = 1;
/// wasi_environment(): the process environment as one text, each `NAME=value` ended by a NUL byte (environ_get's
/// layout), so a WAGI program (`warp build --wagi`) reads its request: REQUEST_METHOD, PATH_INFO, QUERY_STRING
pub const WASI_ENVIRONMENT: &str = "wasi_environment";
/// The two WASI imports wasi_environment calls, imported only by a program that uses it (the browser has no environment)
pub const ENVIRON_IMPORTS: [&str; 2] = ["environ_sizes_get", "environ_get"];
/// environ_sizes_get writes the variable count and the bytes of their text into the scratch words at 0 and 4
const ENVIRON_COUNT_ADDRESS: i32 = 0;
const ENVIRON_BYTES_ADDRESS: i32 = 4;
/// environ_get's table of pointers, one i32 per variable, before the text in the same allocation
const POINTER_BYTES: i32 = 4;
/// Scratch memory of the stdout writes: the iovec {buf_ptr, buf_len} at 0, fd_write's byte count at 8
const IOVEC_ADDRESS: i32 = 0;
const NWRITTEN_ADDRESS: i32 = 8;

impl WasmGcEmitter {
	/// Writes the text at (ptr, len) to stdout: the iovec {ptr, len} at address IOVEC_ADDRESS, then
	/// `fd_write(STDOUT, iovs, 1, nwritten)`. Leaves fd_write's error code on the stack; false when fd_write is not imported.
	fn emit_stdout_write(&self, func: &mut Function, str_ptr: u32, str_len: u32) -> bool {
		func.instruction(&I::I32Const(IOVEC_ADDRESS));
		func.instruction(&I::I32Const(str_ptr as i32));
		func.instruction(&I::I32Store(WORD));
		func.instruction(&I::I32Const(IOVEC_ADDRESS + 4));
		func.instruction(&I::I32Const(str_len as i32));
		func.instruction(&I::I32Store(WORD));
		func.instruction(&I::I32Const(STDOUT));
		func.instruction(&I::I32Const(IOVEC_ADDRESS));
		func.instruction(&I::I32Const(1));
		func.instruction(&I::I32Const(NWRITTEN_ADDRESS));
		let Some(fd_write) = self.ctx.func_registry.get("wasi_fd_write") else { return false };
		func.instruction(&I::Call(fd_write.call_index as u32));
		true
	}

	/// Emit WASI puts: write string to stdout
	/// Memory layout: [0-3]: buf_ptr, [4-7]: buf_len, [8-11]: nwritten
	pub(super) fn emit_wasi_puts(&mut self, func: &mut Function, arg: &Node) {
		// Get string data pointer and length
		let (str_ptr, str_len) = match arg.drop_meta() {
			Node::Text(s) => self.allocate_string(s),
			Node::Symbol(var_name) => match self.scope.lookup(var_name) {
				// a text variable with its stored data
				Some(local) if local.data_pointer > 0 => (local.data_pointer, local.data_length),
				// any other variable: its value at run time (writing the name, or nothing, was a silent wrong answer)
				Some(_) => return self.emit_runtime_puts(func, arg),
				None => self.allocate_string(var_name),
			},
			_ => return self.emit_runtime_puts(func, arg),
		};

		self.emit_stdout_write(func, str_ptr, str_len);
		// Stack now has i32 (error code), leave it for conversion to Node
	}

	/// puts of a value known only at run time: its text through put_value; fd_write's success code, like a constant write
	fn emit_runtime_puts(&mut self, func: &mut Function, value: &Node) {
		self.emit_put_value(func, value);
		func.instruction(&I::I32Const(0));
	}

	/// `print x` / `print(x)`: writes x and a newline to stdout and gives nothing, ø (user, issue #18). A literal is
	/// written as compiled; any other value through its runtime text form (print_value); a value without one is a
	/// type error.
	pub(super) fn emit_print(&mut self, func: &mut Function, value: &Node) {
		match value.drop_meta() {
			Node::Number(number) => self.emit_print_literal(func, &number.to_string()),
			Node::Text(text) => self.emit_print_literal(func, text),
			Node::Char(character) => self.emit_print_literal(func, &character.to_string()),
			// infer_type calls a literal ø an Int, as constructors' field placeholders need (card print-error)
			Node::Empty => self.emit_print_literal(func, &Node::Empty.serialize()),
			_ => {
				self.emit_written_value(func, value, PRINT_VALUE, "print");
				func.instruction(&I::Drop);
			}
		}
		self.emit_call(func, "new_empty");
	}

	/// The print of the program's value an executable ends with (pipeline::prints_result): as emit_print, but a value
	/// only known at run time shows nothing when it is ø, as `warp <file>` shows nothing then
	pub(super) fn emit_result_print(&mut self, func: &mut Function, value: &Node) {
		if !matches!(self.get_type(value), Kind::List | Kind::Key | Kind::Symbol | Kind::Empty | Kind::Data) {
			return self.emit_print(func, value);
		}
		let held = self.emit_dynamic_text(func, value);
		let node_type = self.type_manager.node_type;
		let consumes_text = self.type_manager.function_type(vec![ValType::Ref(self.node_ref(false))], vec![]);
		Self::emit_list(func, &[I::LocalGet(held), I::StructGet { struct_type_index: node_type, field_index: 0 }]);
		Self::emit_list(func, &[I::I64Const(crate::type_kinds::KIND_MASK), I::I64And, I::I64Const(Kind::Empty as i64), I::I64Eq]);
		func.instruction(&I::If(BlockType::FunctionType(consumes_text)));
		func.instruction(&I::Drop);
		func.instruction(&I::Else);
		self.emit_call(func, PRINT_VALUE);
		func.instruction(&I::Drop);
		func.instruction(&I::End);
		self.emit_call(func, "new_empty");
	}

	fn emit_print_literal(&mut self, func: &mut Function, text: &str) {
		self.emit_wasi_puts(func, &Node::Text(format!("{text}{PRINT_TERMINATOR}")));
		func.instruction(&I::Drop);
	}

	/// Write the text of `value` at run time with `writer` (print_value or put_value) and leave the value: a list as the
	/// text `str(xs)` gives, nested lists in brackets (user, P32), a map or an entry likewise; a value of a kind without
	/// a runtime text is a type error named after `word`
	fn emit_written_value(&mut self, func: &mut Function, value: &Node, writer: &'static str, word: &str) {
		match self.get_type(value) {
			// Data: a number whose Int or Float shows at run time (node arithmetic), held as a Node
			Kind::List | Kind::Key | Kind::Symbol | Kind::Empty | Kind::Data => {
				let held = self.emit_dynamic_text(func, value);
				self.emit_call(func, writer);
				Self::emit_list(func, &[I::Drop, I::LocalGet(held), I::RefAsNonNull]);
			}
			Kind::Int | Kind::Float | Kind::Text | Kind::Codepoint => {
				self.emit_node_instructions(func, value);
				self.emit_call(func, writer);
			}
			// a caught Error prints its message (card catch-message); a type error of its own reports itself
			Kind::Error => {
				if !self.reports_own_error(func, value) {
					self.emit_call(func, writer);
				}
			}
			kind => {
				let reason = format!("{word} of {} has no runtime text yet", crate::analyzer::kind_with_article(kind));
				self.emit_type_error(func, crate::diagnostic::Diagnostic::at(value, reason).to_string());
			}
		}
	}

	/// A value typed Error is usually a type error of its own (`print "a"*2`): emitting it reports that error, which is
	/// the one to show; true when it did
	fn reports_own_error(&mut self, func: &mut Function, value: &Node) -> bool {
		let known = self.type_errors.len();
		self.emit_node_instructions(func, value);
		self.type_errors.len() > known
	}

	/// wasi_environment(): environ_sizes_get, then environ_get into a fresh text-heap allocation of the pointer table
	/// (aligned) and the text; the text becomes a text node. Locals: count, bytes, table size, allocation size, address
	pub(super) fn emit_wasi_environment(&mut self) {
		if !self.should_emit_function(WASI_ENVIRONMENT) {
			return;
		}
		let [Some(sizes_get), Some(environ_get)] = ENVIRON_IMPORTS.map(|name| self.ctx.func_registry.get(&format!("wasi_{name}")).map(|f| f.call_index as u32)) else { return };
		self.emit_text_heap_global();
		let node_ref = ValType::Ref(self.node_ref(false));
		self.runtime_function(WASI_ENVIRONMENT, vec![], vec![node_ref], vec![ValType::I32; 5], |s, f| {
			let (count, bytes, table, size, address) = (0, 1, 2, 3, 4);
			Self::emit_list(f, &[
				I::I32Const(ENVIRON_COUNT_ADDRESS), I::I32Const(ENVIRON_BYTES_ADDRESS), I::Call(sizes_get), I::Drop,
				I::I32Const(ENVIRON_COUNT_ADDRESS), I::I32Load(WORD), I::LocalSet(count),
				I::I32Const(ENVIRON_BYTES_ADDRESS), I::I32Load(WORD), I::LocalSet(bytes),
				I::LocalGet(count), I::I32Const(POINTER_BYTES), I::I32Mul, I::LocalSet(table),
				I::LocalGet(table), I::LocalGet(bytes), I::I32Add, I::I32Const(POINTER_BYTES - 1), I::I32Add, I::LocalSet(size),
			]);
			s.emit_text_allocation(f, size, address);
			// environ_get writes the pointer table at a pointer-aligned address
			Self::emit_list(f, &[
				I::LocalGet(address), I::I32Const(POINTER_BYTES - 1), I::I32Add, I::I32Const(-POINTER_BYTES), I::I32And, I::LocalSet(address),
				I::LocalGet(address), I::LocalGet(address), I::LocalGet(table), I::I32Add, I::Call(environ_get), I::Drop,
				I::LocalGet(address), I::LocalGet(table), I::I32Add, I::LocalGet(bytes),
			]);
			s.call(f, "new_text");
		});
	}

	/// print_value(x) and put_value(x): the text of x by list_join of the one-element list [x]; locals: text
	pub(super) fn emit_print_value(&mut self) {
		self.emit_value_writer(PRINT_VALUE, true);
		self.emit_value_writer(PUT_VALUE, false);
	}

	fn emit_value_writer(&mut self, name: &'static str, ends_line: bool) {
		let Some(fd_write) = self.ctx.func_registry.get("wasi_fd_write").map(|f| f.call_index as u32) else { return };
		if !self.should_emit_function(name) {
			return;
		}
		let (node_ref, nullable) = (ValType::Ref(self.node_ref(false)), ValType::Ref(self.node_ref(true)));
		let node_type = self.type_manager.node_type;
		let (empty, _) = self.allocate_string("");
		let newline = self.allocate_string(PRINT_TERMINATOR);
		self.runtime_function(name, vec![node_ref], vec![node_ref], vec![nullable], |s, f| {
			let (value, text) = (0, 1);
			let write = |f: &mut Function, push_address: &dyn Fn(&mut Function), push_length: &dyn Fn(&mut Function)| {
				f.instruction(&I::I32Const(0));
				push_address(f);
				f.instruction(&I::I32Store(WORD));
				f.instruction(&I::I32Const(4));
				push_length(f);
				f.instruction(&I::I32Store(WORD));
				for argument in [STDOUT, 0, 1, 8] {
					f.instruction(&I::I32Const(argument));
				}
				f.instruction(&I::Call(fd_write));
				f.instruction(&I::Drop);
			};
			// [x] or [x, "\n"] joined: a printed line is one write, so lines of concurrent tasks do not interleave
			f.instruction(&I::I64Const(SQUARE_LIST_KIND));
			f.instruction(&I::LocalGet(value));
			if ends_line {
				f.instruction(&I::I64Const(SQUARE_LIST_KIND));
				f.instruction(&I::I32Const(newline.0 as i32));
				f.instruction(&I::I32Const(newline.1 as i32));
				s.call(f, "new_text");
				f.instruction(&I::RefNull(HeapType::Concrete(node_type)));
				f.instruction(&I::StructNew(node_type));
			} else {
				f.instruction(&I::RefNull(HeapType::Concrete(node_type)));
			}
			f.instruction(&I::StructNew(node_type));
			f.instruction(&I::I32Const(empty as i32));
			f.instruction(&I::I32Const(0));
			s.call(f, "new_text");
			s.call(f, "list_join");
			f.instruction(&I::LocalSet(text));
			write(f, &|f| s.emit_text_field(f, text, 0), &|f| s.emit_text_field(f, text, 1));
			f.instruction(&I::LocalGet(value));
		});
	}

	/// Emit WASI puti: write an integer to stdout, without a newline; a run-time value through put_value
	pub(super) fn emit_wasi_puti(&mut self, func: &mut Function, arg: &Node) {
		if let Node::Number(n) = arg.drop_meta() {
			let (str_ptr, str_len) = self.allocate_string(&n.to_string());
			if self.emit_stdout_write(func, str_ptr, str_len) {
				func.instruction(&I::Drop);
			}
		} else {
			self.emit_put_value(func, arg);
		}
	}

	fn emit_put_value(&mut self, func: &mut Function, value: &Node) {
		self.emit_written_value(func, value, PUT_VALUE, "puts");
		func.instruction(&I::Drop);
	}

	/// Emit WASI putf: write float to stdout
	pub(super) fn emit_wasi_putf(&mut self, func: &mut Function, arg: &Node) {
		// For compile-time constants, pre-compute the string
		if let Node::Number(n) = arg.drop_meta() {
			let s = format!("{}", n); // Number implements Display
			let (str_ptr, str_len) = self.allocate_string(&s);

			self.emit_stdout_write(func, str_ptr, str_len);
		} else {
			self.emit_runtime_puts(func, arg);
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
			func.instruction(&I::I64ExtendI32S);
		} else {
			// Raw numeric mode
			for arg in args.iter().take(4) {
				self.emit_numeric_value(func, arg);
				func.instruction(&I::I32WrapI64);
			}
			if let Some(f) = self.ctx.func_registry.get("wasi_fd_write") {
				func.instruction(&I::Call(f.call_index as u32));
				func.instruction(&I::I64ExtendI32S);
			}
		}
	}
}
