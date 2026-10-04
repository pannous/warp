//! Key node emission - handles all Key(left, op, right) patterns

use crate::node::{Bracket, Node};
use crate::operators::Op;
use wasm_encoder::*;
use Instruction as I;

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
				self.emit_fetch_call(func, right, None);
				return;
			}
		}

		// Handle x = fetch URL pattern: Key(Assign, x, List[fetch, URL]), optionally `… timeout SECONDS`
		if (*op == Op::Assign || *op == Op::Define) && self.config.emit_host_imports {
			if let Node::Symbol(var_name) = left.drop_meta() {
				if let Some((url, timeout)) = crate::host::fetch_call(right) {
					// Emit fetch call - result is a Text or Error node (ref $Node)
					self.emit_fetch_call(func, &url, timeout);
					// Store in ref-type local variable
					if let Some(local) = self.scope.lookup(var_name) {
						func.instruction(&I::LocalTee(local.position));
					}
					return;
				}
			}
		}

		// text += more → text = text + more, list += [x] → list = list + [x]
		if op.is_compound_assign() && op.base_op() == Op::Add {
			let concatenation = Node::Key(Box::new(left.clone()), Op::Add, Box::new(right.clone()));
			if matches!(self.get_type(&concatenation), crate::type_kinds::Kind::Text | crate::type_kinds::Kind::List) {
				self.emit_key_node(func, left, &Op::Assign, &concatenation);
				return;
			}
		}

		// Skip user function definitions - they're already compiled; a definition's value is ø: `square := it*it` or
		// `f(x) := x + 1` alone is a program of no value
		let definition = Node::Key(Box::new(left.clone()), *op, Box::new(right.clone()));
		if let Some(name) = self.defined_function_name(&definition).filter(|name| self.ctx.user_functions.contains_key(name)) {
			self.emit_closure_capture(func, &name);
			self.emit_call(func, "new_empty");
			return;
		}

		// Route to emit_arithmetic for numeric operations
		let assigns_numeric_variable = matches!(left.drop_meta(), Node::Symbol(s) if self.is_numeric_variable(s));
		let is_numeric_assign = *op == Op::Assign && assigns_numeric_variable;
		let is_numeric_define = *op == Op::Define && assigns_numeric_variable;

		// Handle ref-type variable assignment (lists, etc.)
		let is_ref_assign = (*op == Op::Assign || *op == Op::Define)
			&& matches!(left.drop_meta(), Node::Symbol(s) if {
				self.scope.lookup(s).is_some_and(|l| l.kind.is_ref())
			});
		let is_ref_global_assign = (*op == Op::Assign || *op == Op::Define)
			&& matches!(left.drop_meta(), Node::Symbol(s) if self.scope.lookup(s).is_none()
				&& self.ctx.user_globals.get(s).is_some_and(|(_, kind)| kind.is_ref()));

		if is_ref_global_assign {
			if let Node::Symbol(name) = left.drop_meta() {
				self.emit_global_store(func, name, right);
			}
			return;
		}
		if is_ref_assign {
			if let Node::Symbol(name) = left.drop_meta() {
				if self.emit_typed_list_store(func, name, right) {
					func.instruction(&I::Drop);
					self.emit_typed_list_as_node(func, name);
					return;
				}
				// Emit the right side as a Node reference
				self.emit_node_instructions(func, right);
				// Store in ref-type local
				if let Some(local) = self.scope.lookup(name) {
					func.instruction(&I::LocalTee(local.position));
				}
			}
			return;
		}

		// Handle index assignment: node#index = value
		if *op == Op::Assign {
			if let Node::Key(node_expr, Op::Hash, index_expr) = left.drop_meta() {
				self.emit_index_assignment(func, node_expr, index_expr, right);
				self.emit_call(func, "new_int");
				return;
			}
		}

		// For logical ops with non-numeric operands, use Node-returning truthy path
		let both_numeric = self.is_numeric(left) && self.is_numeric(right);
		if self.compares_structurally(op, left, right) {
			self.emit_structural_equality(func, left, op, right);
			self.emit_call(func, "new_int");
		} else if self.orders_a_list(op, left, right) {
			self.emit_unordered_list_error(func, left, op, right);
		} else if op.is_logical() && !both_numeric {
			self.emit_truthy_logical(func, left, op, right);
		} else if op.is_arithmetic()
			|| op.is_shift()
			|| op.is_comparison()
			|| op.is_logical()
			|| is_numeric_define
			|| is_numeric_assign
			|| op.is_compound_assign()
			|| matches!(op, Op::Inc | Op::Dec)
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
			func.instruction(&I::F64Mul);
			if op == Op::Cube {
				self.emit_float_value(func, left);
				func.instruction(&I::F64Mul);
			}
			self.emit_call(func, "new_float");
		} else {
			let range = self.int_range(left);
			self.emit_numeric_value(func, left);
			self.emit_numeric_value(func, left);
			self.emit_int_op(func, &Op::Mul, range, range);
			if op == Op::Cube {
				self.emit_numeric_value(func, left);
				let squared = self.int_range(&Node::Key(Box::new(left.clone()), Op::Mul, Box::new(left.clone())));
				self.emit_int_op(func, &Op::Mul, squared, range);
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
				func.instruction(&I::F64Sqrt);
				self.emit_call(func, "new_float");
			}
			Op::Neg => {
				// -x = 0 - x
				let use_float = self.get_type(right).is_float();
				if use_float {
					func.instruction(&I::F64Const(0.0.into()));
					self.emit_float_value(func, right);
					func.instruction(&I::F64Sub);
					self.emit_call(func, "new_float");
				} else {
					self.emit_numeric_value(func, right);
					let range = self.int_range(right);
					self.emit_int_neg(func, range);
					self.emit_call(func, "new_int");
				}
			}
			Op::Not => {
				// not x = x is falsy: 0, ø and errors
				self.emit_condition(func, right, Self::emit_numeric_value);
				func.instruction(&I::I32Eqz);
				func.instruction(&I::I64ExtendI32U);
				self.emit_call(func, "new_int");
			}
			Op::Abs => {
				self.emit_abs_op(func, right);
			}
			_ => {
				// Fallback: emit as Key node
				self.emit_node_instructions(func, &Node::Empty);
				self.emit_node_instructions(func, right);
				func.instruction(&I::I64Const(crate::operators::op_to_code(op)));
				self.emit_call(func, "new_key");
			}
		}
	}

	/// Emit absolute value: ‖x‖
	fn emit_abs_op(&mut self, func: &mut Function, right: &Node) {
		let use_float = self.get_type(right).is_float();
		if use_float {
			self.emit_float_value(func, right);
			func.instruction(&I::F64Abs);
			self.emit_call(func, "new_float");
		} else {
			self.emit_numeric_value(func, right);
			let range = self.int_range(right);
			self.emit_int_abs(func, range);
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
			// Prefix #x = count
			self.emit_list_count(func, right, "node_count");
			// node_count returns i64, wrap in new_int
			self.emit_call(func, "new_int");
		} else {
			// Infix x#y = indexing - dispatches to string_char_at or list_node_at at runtime, or looks up a key
			self.emit_indexed_node(func, left, right);
		}
	}

	/// The runtime counter behind the getter word in `x.size`, `x.count`, `x.bytes`
	pub(super) fn counting_getter(&self, property: &Node) -> Option<&'static str> {
		let Node::Symbol(word) = property.drop_meta() else { return None };
		crate::analyzer::counting_method(word, &self.ctx)
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
			if let Some(counter) = crate::analyzer::counting_method(method, &self.ctx) {
				// obj.count, obj.length: elements, or graphemes of a text; obj.bytes/chars/graphemes
				self.emit_list_count(func, left, counter);
				self.emit_call(func, "new_int");
				return;
			}
		}

		// Default: emit as Key node
		self.emit_node_instructions(func, left);
		self.emit_node_instructions(func, right);
		func.instruction(&I::I64Const(crate::operators::op_to_code(&Op::Dot)));
		self.emit_call(func, "new_key");
	}

	/// Emit default Key node (preserve structure for roundtrip)
	pub(super) fn emit_default_key(&mut self, func: &mut Function, left: &Node, right: &Node, op: &Op) {
		match left.drop_meta() {
			// `{x: x}`: the name of an entry is its name, also when a variable or parameter is called so
			Node::Symbol(name) if *op == Op::Colon => self.emit_string_call(func, name, "new_symbol"),
			_ => self.emit_node_instructions(func, left),
		}
		// For struct instances like Person{...}, emit block as list; the value of an entry `a:{b:1}` stays a map
		let right_node = right.drop_meta();
		if let (Node::List(items, Bracket::Curly, sep), false) = (right_node, *op == Op::Colon) {
			// Convert curly block to square list, preserving inner ops
			let list_node = Node::List(items.clone(), Bracket::Square, sep.clone());
			self.emit_node_instructions(func, &list_node);
		} else {
			self.emit_node_instructions(func, right_node);
		}
		// Preserve the op for roundtrip
		func.instruction(&I::I64Const(crate::operators::op_to_code(op)));
		self.emit_call(func, "new_key");
	}
}
