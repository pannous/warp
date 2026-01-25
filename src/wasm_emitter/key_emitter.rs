//! Key node emission - handles all Key(left, op, right) patterns

use crate::node::{Bracket, Node};
use crate::operators::Op;
use wasm_encoder::*;

use super::WasmGcEmitter;

impl WasmGcEmitter {
	/// Emit instructions for Key(left, op, right) nodes
	/// Dispatches to appropriate handlers based on the operator and operands
	pub(super) fn emit_key_node(&mut self, func: &mut Function, left: &Node, op: &Op, right: &Node) {
		// Handle global keyword: global:Key(name, =, value)
		if let Node::Symbol(kw) = left.drop_meta() {
			if kw == "global" {
				self.emit_global_declaration(func, right);
				return;
			}
			// Handle fetch URL - call host.fetch and return Text node
			if kw == "fetch" && self.config.emit_host_imports {
				self.emit_fetch_call(func, right);
				return;
			}
		}

		// Handle x = fetch URL pattern: Key(Assign, x, List[fetch, URL])
		if (*op == Op::Assign || *op == Op::Define) && self.config.emit_host_imports {
			if let Node::Symbol(var_name) = left.drop_meta() {
				if let Node::List(items, _, _) = right.drop_meta() {
					if items.len() == 2 {
						if let Node::Symbol(s) = items[0].drop_meta() {
							if s == "fetch" {
								// Emit fetch call - result is a Text node (ref $Node)
								self.emit_fetch_call(func, &items[1]);
								// Store in ref-type local variable
								if let Some(local) = self.scope.lookup(var_name) {
									func.instruction(&Instruction::LocalTee(local.position));
								}
								return;
							}
						}
					}
				}
			}
		}

		// Skip user function definitions - they're already compiled
		if *op == Op::Define {
			if let Node::Symbol(name) = left.drop_meta() {
				if self.ctx.user_functions.contains_key(name) {
					// Function definitions don't produce a value
					return;
				}
			}
		}

		// Route to emit_arithmetic for numeric operations
		let is_numeric_assign = *op == Op::Assign
			&& matches!(left.drop_meta(), Node::Symbol(s) if {
				self.scope.lookup(s).is_some_and(|l| !l.kind.is_ref())
			});
		let is_numeric_define = *op == Op::Define
			&& matches!(left.drop_meta(), Node::Symbol(s) if {
				self.scope.lookup(s).is_some_and(|l| !l.kind.is_ref())
			});

		// Handle ref-type variable assignment (lists, etc.)
		let is_ref_assign = (*op == Op::Assign || *op == Op::Define)
			&& matches!(left.drop_meta(), Node::Symbol(s) if {
				self.scope.lookup(s).is_some_and(|l| l.kind.is_ref())
			});

		if is_ref_assign {
			if let Node::Symbol(name) = left.drop_meta() {
				// Emit the right side as a Node reference
				self.emit_node_instructions(func, right);
				// Store in ref-type local
				if let Some(local) = self.scope.lookup(name) {
					func.instruction(&Instruction::LocalTee(local.position));
				}
			}
			return;
		}

		// Handle index assignment: node#index = value → node_set_at(node, index, value)
		if *op == Op::Assign {
			if let Node::Key(node_expr, Op::Hash, index_expr) = left.drop_meta() {
				// Emit node (string or list ref)
				self.emit_node_instructions(func, node_expr);
				// Emit index (as i64)
				self.emit_numeric_value(func, index_expr);
				// Emit value (as i64)
				self.emit_numeric_value(func, right);
				// Call node_set_at which dispatches to string_set_char_at or list_set_at
				self.emit_call(func, "node_set_at");
				// Wrap result as Node
				self.emit_call(func, "new_int");
				return;
			}
		}

		// For logical ops with non-numeric operands, use Node-returning truthy path
		let both_numeric = self.is_numeric(left) && self.is_numeric(right);
		if op.is_logical() && !both_numeric {
			self.emit_truthy_logical(func, left, op, right);
		} else if op.is_arithmetic()
			|| op.is_comparison()
			|| op.is_logical()
			|| is_numeric_define
			|| is_numeric_assign
			|| op.is_compound_assign()
		{
			self.emit_arithmetic(func, left, op, right);
		} else if *op == Op::Square || *op == Op::Cube {
			self.emit_power_op(func, left, *op);
		} else if op.is_prefix() && matches!(left.drop_meta(), Node::Empty) {
			self.emit_prefix_op(func, right, op);
		} else if *op == Op::Question {
			// Ternary: condition ? then : else
			self.emit_ternary(func, left, right);
		} else if *op == Op::Else {
			self.emit_else_op(func, left, right);
		} else if *op == Op::Then {
			// If-then (no else): (if condition) then then_expr
			let full_node = Node::Key(Box::new(left.clone()), Op::Then, Box::new(right.clone()));
			self.emit_if_then_else(func, &full_node, None);
		} else if *op == Op::Do {
			// While loop: (while condition) do body
			self.emit_while_loop(func, left, right);
		} else if *op == Op::Hash {
			self.emit_hash_op(func, left, right);
		} else if *op == Op::As {
			// Type cast: value as type
			self.emit_cast(func, left, right);
		} else if *op == Op::Dot {
			self.emit_dot_op(func, left, right);
		} else if *op == Op::Range || *op == Op::To {
			// Range operators: 0..3 (exclusive) or 0...3 / 0…3 (inclusive)
			self.emit_range(func, left, right, *op == Op::To);
		} else {
			self.emit_default_key(func, left, right, op);
		}
	}

	/// Emit suffix power operators: x², x³
	fn emit_power_op(&mut self, func: &mut Function, left: &Node, op: Op) {
		let use_float = self.get_type(left).is_float();
		if use_float {
			self.emit_float_value(func, left);
			self.emit_float_value(func, left);
			func.instruction(&Instruction::F64Mul);
			if op == Op::Cube {
				self.emit_float_value(func, left);
				func.instruction(&Instruction::F64Mul);
			}
			self.emit_call(func, "new_float");
		} else {
			self.emit_numeric_value(func, left);
			self.emit_numeric_value(func, left);
			func.instruction(&Instruction::I64Mul);
			if op == Op::Cube {
				self.emit_numeric_value(func, left);
				func.instruction(&Instruction::I64Mul);
			}
			self.emit_call(func, "new_int");
		}
	}

	/// Emit prefix operators: √x, -x, !x, ‖x‖
	fn emit_prefix_op(&mut self, func: &mut Function, right: &Node, op: &Op) {
		match op {
			Op::Sqrt => {
				// √x = sqrt(x), returns float
				self.emit_float_value(func, right);
				func.instruction(&Instruction::F64Sqrt);
				self.emit_call(func, "new_float");
			}
			Op::Neg => {
				// -x = 0 - x
				let use_float = self.get_type(right).is_float();
				if use_float {
					func.instruction(&Instruction::F64Const(0.0.into()));
					self.emit_float_value(func, right);
					func.instruction(&Instruction::F64Sub);
					self.emit_call(func, "new_float");
				} else {
					func.instruction(&Instruction::I64Const(0));
					self.emit_numeric_value(func, right);
					func.instruction(&Instruction::I64Sub);
					self.emit_call(func, "new_int");
				}
			}
			Op::Not => {
				// !x = x == 0
				self.emit_numeric_value(func, right);
				func.instruction(&Instruction::I64Eqz);
				func.instruction(&Instruction::I64ExtendI32U);
				self.emit_call(func, "new_int");
			}
			Op::Abs => {
				self.emit_abs_op(func, right);
			}
			_ => {
				// Fallback: emit as Key node
				self.emit_node_instructions(func, &Node::Empty);
				self.emit_node_instructions(func, right);
				func.instruction(&Instruction::I64Const(crate::operators::op_to_code(op)));
				self.emit_call(func, "new_key");
			}
		}
	}

	/// Emit absolute value: ‖x‖
	fn emit_abs_op(&mut self, func: &mut Function, right: &Node) {
		let use_float = self.get_type(right).is_float();
		if use_float {
			self.emit_float_value(func, right);
			func.instruction(&Instruction::F64Abs);
			self.emit_call(func, "new_float");
		} else {
			// i64 abs: if x < 0 then -x else x
			self.emit_numeric_value(func, right);
			func.instruction(&Instruction::LocalTee(self.next_temp_local));
			func.instruction(&Instruction::I64Const(0));
			func.instruction(&Instruction::I64LtS);
			func.instruction(&Instruction::If(BlockType::Result(ValType::I64)));
			func.instruction(&Instruction::I64Const(0));
			func.instruction(&Instruction::LocalGet(self.next_temp_local));
			func.instruction(&Instruction::I64Sub);
			func.instruction(&Instruction::Else);
			func.instruction(&Instruction::LocalGet(self.next_temp_local));
			func.instruction(&Instruction::End);
			self.emit_call(func, "new_int");
		}
	}

	/// Emit else operator: handles if-then-else or fallback operator
	fn emit_else_op(&mut self, func: &mut Function, left: &Node, right: &Node) {
		// Check if this is a full if-then-else or just a fallback operator
		if let Node::Key(_, Op::Then, _) = left.drop_meta() {
			// If-then-else: ((if condition) then then_expr) else else_expr
			self.emit_if_then_else(func, left, Some(right));
		} else {
			// Standalone else acts like truthy or: `false else 3` → 3
			self.emit_truthy_logical(func, left, &Op::Or, right);
		}
	}

	/// Emit hash operator: count (#x) or indexing (x#y)
	fn emit_hash_op(&mut self, func: &mut Function, left: &Node, right: &Node) {
		// Check if prefix (count) or infix (index)
		if matches!(left.drop_meta(), Node::Empty) {
			// Prefix #x = count - emit the node and call node_count
			self.emit_node_instructions(func, right);
			self.emit_call(func, "node_count");
			// node_count returns i64, wrap in new_int
			self.emit_call(func, "new_int");
		} else {
			// Infix x#y = indexing - dispatches to string_char_at or list_node_at at runtime
			self.emit_node_instructions(func, left); // emit node (string or list)
			self.emit_numeric_value(func, right); // emit index
			self.emit_call(func, "node_index_at"); // runtime dispatch
		}
	}

	/// Emit dot operator: method calls and property access
	fn emit_dot_op(&mut self, func: &mut Function, left: &Node, right: &Node) {
		// Check for introspection methods: count, number, size
		let method_name = match right.drop_meta() {
			Node::Symbol(s) => Some(s.clone()),
			Node::List(items, _, _) if items.len() == 1 => {
				// Method call: obj.method() parses as Key(obj, Dot, List([method]))
				if let Node::Symbol(s) = items[0].drop_meta() {
					Some(s.clone())
				} else {
					None
				}
			}
			_ => None,
		};

		if let Some(ref method) = method_name {
			match method.as_str() {
				"count" | "number" => {
					// obj.count() or obj.count returns element count
					self.emit_node_instructions(func, left);
					self.emit_call(func, "node_count");
					self.emit_call(func, "new_int");
					return;
				}
				"size" => {
					// obj.size() returns byte count (elements * 8)
					self.emit_node_instructions(func, left);
					self.emit_call(func, "node_count");
					func.instruction(&Instruction::I64Const(8));
					func.instruction(&Instruction::I64Mul);
					self.emit_call(func, "new_int");
					return;
				}
				_ => {}
			}
		}

		// Default: emit as Key node
		self.emit_node_instructions(func, left);
		self.emit_node_instructions(func, right);
		func.instruction(&Instruction::I64Const(crate::operators::op_to_code(&Op::Dot)));
		self.emit_call(func, "new_key");
	}

	/// Emit default Key node (preserve structure for roundtrip)
	fn emit_default_key(&mut self, func: &mut Function, left: &Node, right: &Node, op: &Op) {
		self.emit_node_instructions(func, left);
		// For struct instances like Person{...}, emit block as list
		let right_node = right.drop_meta();
		if let Node::List(items, Bracket::Curly, sep) = right_node {
			// Convert curly block to square list, preserving inner ops
			let list_node = Node::List(items.clone(), Bracket::Square, sep.clone());
			self.emit_node_instructions(func, &list_node);
		} else {
			self.emit_node_instructions(func, right_node);
		}
		// Preserve the op for roundtrip
		func.instruction(&Instruction::I64Const(crate::operators::op_to_code(op)));
		self.emit_call(func, "new_key");
	}
}
