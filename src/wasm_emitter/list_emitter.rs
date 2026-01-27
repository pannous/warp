//! List node emission - handles all List(items, bracket, separator) patterns

use crate::node::{Bracket, Node, Separator};
use crate::operators::{is_function_keyword, Op};
use crate::normalize::hints as norm;
use wasm_encoder::*;

use super::WasmGcEmitter;

impl WasmGcEmitter {
	/// Emit instructions for List(items, bracket, separator) nodes
	/// Dispatches based on list contents and bracket type
	pub(super) fn emit_list_node(&mut self, func: &mut Function, items: &[Node], bracket: &Bracket) {
		if items.is_empty() {
			self.emit_call(func, "new_empty");
			return;
		}

		if items.len() == 1 {
			// Check for zero-argument function call: (funcname)
			if *bracket == Bracket::Round {
				if let Node::Symbol(fn_name) = items[0].drop_meta() {
					if self.ctx.user_functions.contains_key(fn_name) {
						self.emit_user_function_call(func, fn_name, &[]);
						return;
					}
				}
			}
			self.emit_node_instructions(func, &items[0]);
			return;
		}

		// Check for fetch call: [Symbol("fetch"), url_node]
		if items.len() == 2 && self.config.emit_host_imports {
			if let Node::Symbol(s) = items[0].drop_meta() {
				if s == "fetch" {
					self.emit_fetch_call(func, &items[1]);
					return;
				}
			}
		}

		// Check for return statement: return value
		if items.len() == 2 {
			if let Node::Symbol(s) = items[0].drop_meta() {
				if s == "return" {
					// Emit the return value and return instruction
					self.emit_node_instructions(func, &items[1]);
					func.instruction(&Instruction::Return);
					// Unreachable after return, push dummy value
					func.instruction(&Instruction::Unreachable);
					return;
				}
			}
		}

		// Check for WASI calls: puts, puti, putl, putf, fd_write
		if items.len() >= 2 && self.config.emit_wasi_imports {
			if let Node::Symbol(s) = items[0].drop_meta() {
				match s.as_str() {
					"puts" => {
						self.emit_wasi_puts(func, &items[1]);
						func.instruction(&Instruction::I64ExtendI32S);
						return;
					}
					"puti" | "putl" => {
						self.emit_wasi_puti(func, &items[1]);
						self.emit_numeric_value(func, &items[1]);
						return;
					}
					"putf" => {
						self.emit_wasi_putf(func, &items[1]);
						func.instruction(&Instruction::I64ExtendI32S);
						return;
					}
					"fd_write" if items.len() >= 5 => {
						self.emit_wasi_fd_write_call(func, &items[1..]);
						return;
					}
					_ => {}
				}
			}
		}

		// Check for introspection and math functions
		if items.len() == 2 {
			if let Node::Symbol(fn_name) = items[0].drop_meta() {
				if self.emit_introspection_fn(func, fn_name, &items[1]) {
					return;
				}
			}
		}

		// Check for type constructor calls: int('123'), str(123), char(0x41), etc.
		if items.len() == 2 {
			if let Node::Symbol(type_name) = items[0].drop_meta() {
				let is_typed_decl = matches!(items[1].drop_meta(), Node::Key(_, Op::Assign | Op::Define, _));
				if !is_typed_decl {
					match type_name.as_str() {
						"int" | "float" | "str" | "string" | "String" | "char" | "bool" | "number" => {
							norm::type_constructor(type_name, &items[1].to_string());
							if type_name == "str" || type_name == "String" {
								norm::string_type(type_name);
							}
							self.emit_cast(func, &items[1], &items[0]);
							return;
						}
						_ => {}
					}
				}
			}
		}

		// Check for range function: range start end
		if items.len() == 3 {
			if let Node::Symbol(fn_name) = items[0].drop_meta() {
				if fn_name == "range" {
					self.emit_range(func, &items[1], &items[2], true);
					return;
				}
			}
		}

		// Check for user function call: [Symbol("funcname"), arg1, arg2, ...]
		if items.len() >= 2 {
			if let Node::Symbol(fn_name) = items[0].drop_meta() {
				if self.ctx.user_functions.contains_key(fn_name) {
					if items.len() == 2 && matches!(items[1].drop_meta(), Node::Empty) {
						self.emit_user_function_call(func, fn_name, &[]);
					} else {
						self.emit_user_function_call(func, fn_name, &items[1..]);
					}
					return;
				}
				// Check for FFI function call
				if self.ctx.ffi_imports.contains_key(fn_name) {
					self.emit_ffi_call(func, fn_name, &items[1..], None);
					return;
				}
			}
		}

		// Check if this list contains type definitions
		let has_type_def = items.iter().any(|item| matches!(item.drop_meta(), Node::Type { .. }));
		if has_type_def {
			self.emit_type_def_list(func, items);
			return;
		}

		// Check if this is a statement sequence
		let is_statement_sequence = self.is_statement_sequence(items);

		if is_statement_sequence {
			self.emit_statement_sequence(func, items);
		} else {
			// Check for pure numeric expressions
			let has_arithmetic = items.iter().any(|item| {
				matches!(item.drop_meta(), Node::Key(_, op, _) if op.is_arithmetic())
			});
			if has_arithmetic {
				let node = Node::List(items.to_vec(), bracket.clone(), Separator::None);
				if self.get_type(&node).is_float() {
					self.emit_float_value(func, &node);
					self.emit_call(func, "new_float");
				} else {
					self.emit_numeric_value(func, &node);
					self.emit_call(func, "new_int");
				}
			} else {
				// Build linked list: (first, rest, bracket_info)
				self.emit_list_structure(func, items, bracket);
			}
		}
	}

	/// Emit introspection functions: type, count, size, ceil, floor, round
	/// Returns true if the function was handled
	fn emit_introspection_fn(&mut self, func: &mut Function, fn_name: &str, arg: &Node) -> bool {
		match fn_name {
			"type" => {
				let kind = self.get_type(arg);
				let type_name = kind.to_string();
				let (ptr, len) = self.allocate_string(&type_name);
				func.instruction(&Instruction::I32Const(ptr as i32));
				func.instruction(&Instruction::I32Const(len as i32));
				self.emit_call(func, "new_symbol");
				true
			}
			"count" => {
				self.emit_node_instructions(func, arg);
				self.emit_call(func, "node_count");
				self.emit_call(func, "new_int");
				true
			}
			"size" => {
				self.emit_node_instructions(func, arg);
				self.emit_call(func, "node_count");
				func.instruction(&Instruction::I64Const(8));
				func.instruction(&Instruction::I64Mul);
				self.emit_call(func, "new_int");
				true
			}
			"ceil" if !self.ctx.ffi_imports.contains_key(fn_name) => {
				self.emit_float_value(func, arg);
				func.instruction(&Instruction::F64Ceil);
				func.instruction(&Instruction::I64TruncF64S);
				self.emit_call(func, "new_int");
				true
			}
			"floor" if !self.ctx.ffi_imports.contains_key(fn_name) => {
				self.emit_float_value(func, arg);
				func.instruction(&Instruction::F64Floor);
				func.instruction(&Instruction::I64TruncF64S);
				self.emit_call(func, "new_int");
				true
			}
			"round" if !self.ctx.ffi_imports.contains_key(fn_name) => {
				self.emit_float_value(func, arg);
				func.instruction(&Instruction::F64Nearest);
				func.instruction(&Instruction::I64TruncF64S);
				self.emit_call(func, "new_int");
				true
			}
			_ => false,
		}
	}

	/// Check if items form a statement sequence
	fn is_statement_sequence(&self, items: &[Node]) -> bool {
		items.iter().any(|item| {
			let item = item.drop_meta();
			match item {
				Node::Key(_, Op::Assign | Op::Define, _) => true,
				Node::Key(_, Op::Hash, _) => true,
				Node::Key(_, op, _) if op.is_compound_assign() => true,
				Node::Key(left, Op::Colon, _) => {
					matches!(left.drop_meta(), Node::Symbol(s) if s == "global")
				}
				Node::List(list_items, _, _) if list_items.len() >= 2 => {
					if let Node::Symbol(s) = list_items[0].drop_meta() {
						is_function_keyword(s) || s == "use" || s == "import"
					} else {
						false
					}
				}
				_ => false,
			}
		})
	}

	/// Emit a list containing type definitions
	fn emit_type_def_list(&mut self, func: &mut Function, items: &[Node]) {
		let last_expr = items
			.iter()
			.rev()
			.find(|item| !matches!(item.drop_meta(), Node::Type { .. }));
		if let Some(expr) = last_expr {
			self.emit_node_instructions(func, expr);
		} else {
			self.emit_call(func, "new_empty");
		}
	}

	/// Emit a statement sequence (filter out functions, execute in order, return last)
	fn emit_statement_sequence(&mut self, func: &mut Function, items: &[Node]) {
		let non_func_items: Vec<_> = items
			.iter()
			.filter(|item| !self.is_function_definition(item))
			.collect();

		for (i, item) in non_func_items.iter().enumerate() {
			self.emit_node_instructions(func, item);
			// Drop intermediate results, keep last
			if i < non_func_items.len() - 1 {
				func.instruction(&Instruction::Drop);
			}
		}
	}

	/// Check if item is a function definition that should be filtered from statement sequences
	fn is_function_definition(&self, item: &Node) -> bool {
		match item.drop_meta() {
			// Pattern: name := body
			Node::Key(left, Op::Define, _) => {
				if let Node::Symbol(name) = left.drop_meta() {
					if self.ctx.user_functions.contains_key(name) {
						return true;
					}
				}
				// Pattern: name(params...) := body
				if let Node::List(items, _, _) = left.drop_meta() {
					if !items.is_empty() {
						if let Node::Symbol(name) = items[0].drop_meta() {
							if self.ctx.user_functions.contains_key(name) {
								return true;
							}
						}
					}
				}
			}
			// Pattern: name(params...) = body
			Node::Key(left, Op::Assign, _) => {
				if let Node::List(items, _, _) = left.drop_meta() {
					if !items.is_empty() {
						if let Node::Symbol(name) = items[0].drop_meta() {
							if self.ctx.user_functions.contains_key(name) {
								return true;
							}
						}
					}
				}
			}
			// Pattern: def/fun/fn name(params...): body
			Node::List(list_items, _, _) => {
				if list_items.len() >= 2 {
					if let Node::Symbol(s) = list_items[0].drop_meta() {
						if is_function_keyword(s) || s == "use" || s == "import" {
							return true;
						}
					}
				}
			}
			_ => {}
		}
		false
	}
}
