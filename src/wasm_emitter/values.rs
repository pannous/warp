//! Values as raw numbers: numeric and float values of any node, integer builtins, ranges, list structures

use super::*;

impl WasmGcEmitter {
	pub(super) fn emit_numeric_value(&mut self, func: &mut Function, node: &Node) {
		if let Some(identity) = self.object_identity(node) {
			return self.emit_object_identity(func, identity);
		}
		if let Some(product) = self.numeric_times(node).or_else(|| self.identity_as_equality(node)).or_else(|| crate::analyzer::spelled_number(node)) {
			return self.emit_numeric_value(func, &product);
		}
		self.note_position(node);
		if self.emit_raised_error(func, node) || self.emit_undefined_comparison(func, node) {
			return;
		}
		if let Some((list, sum_loop)) = list_dispatch::list_sum_parts(node) {
			return self.emit_list_sum(func, list, sum_loop, list_dispatch::Wanted::Int);
		}
		if self.emit_loop_jump(func, node) || self.emit_tuple_statement(func, node, Self::emit_numeric_value) || self.emit_type_test_value(func, node) {
			return;
		}
		let located = node;
		let node = node.drop_meta();
		// Handle global declaration: global:Key(name, =, value)
		if let Node::Key(left, Op::Colon, right) = node {
			if let Node::Symbol(kw) = left.drop_meta() {
				if kw == "global" {
					// Emit global and return value on stack
					self.emit_global_numeric(func, right);
					return;
				}
			}
		}
		match node {
			Node::Number(num) => {
				match num {
					Number::Int(_) | Number::BigInt(_) => {
						self.emit_int_literal(func, &num.to_bigint());
						func
					}
					Number::Float(f) => {
						self.emit_decimal_literal(func, *f);
						func
					}
					Number::Quotient(n, d) => {
						self.emit_exact_literal(func, &(*n).into(), &(*d).into());
						func
					}
					Number::BigQuotient(q) => {
						self.emit_exact_literal(func, &q.numerator, &q.denominator);
						func
					}
					Number::Complex(r, _i) => func.instruction(&I::I64Const(*r as i64)),
					// lowered to Float before emission (real.rs), kept total for safety
					Number::Real(r) => func.instruction(&I::I64Const(r.to_f64() as i64)),
					Number::Nan | Number::Inf | Number::NegInf => {
						func.instruction(&I::I64Const(0)) // special values → 0
					}
				};
			}
			Node::True => {
				func.instruction(&I::I64Const(1));
			}
			Node::False => {
				func.instruction(&I::I64Const(0));
			}
			Node::Char(c) => {
				func.instruction(&I::I64Const(*c as i64));
			}
			// Variable definition/assignment: x:=42 or x=42 → store and return value
			Node::Key(left, Op::Define | Op::Assign, right) => self.emit_numeric_assignment(func, located, left, right),
			// `x as int` of an Int, what a declared result type `-> int` lowers to: x, a ratio truncated, without a box
			Node::Key(value, Op::As, target) if self.get_type(value) == Kind::Int && is_int_type_word(target) => self.emit_int_value_truncated(func, value),
			// Increment/decrement: i++ or i--
			Node::Key(left, op, right) if *op == Op::Inc || *op == Op::Dec => self.emit_numeric_step(func, left, op, right),
			// Compound assignment: x += y → x = x + y
			Node::Key(left, op, right) if op.is_compound_assign() => {
				self.emit_compound_assign(func, left, op, right, false);
			}
			// Arithmetic operators
			Node::Key(left, op, right) if op.is_arithmetic() => {
				let kind = self.arithmetic_type(left, op, right);
				if self.emit_arithmetic_type_error(func, left, op, right, kind) {
					return;
				}
				self.emit_int_operands_op(func, left, op, right);
			}
			Node::Key(left, op, right) if op.is_shift() => self.emit_shift(func, left, op, right),
			// Logical operators (and, or) use truthy semantics, xor uses bitwise
			Node::Key(left, Op::And, right) => {
				// Truthy and: if left is 0, return 0; else return right
				self.emit_numeric_value(func, left);
				func.instruction(&I::I64Eqz);
				func.instruction(&I::If(BlockType::Result(ValType::I64)));
				func.instruction(&I::I64Const(0));
				func.instruction(&I::Else);
				self.emit_numeric_value(func, right);
				func.instruction(&I::End);
			}
			Node::Key(left, Op::Or, right) => {
				// Truthy or: if left is non-0, return left; else return right
				self.emit_numeric_value(func, left);
				func.instruction(&I::I64Eqz);
				func.instruction(&I::If(BlockType::Result(ValType::I64)));
				self.emit_numeric_value(func, right);
				func.instruction(&I::Else);
				self.emit_numeric_value(func, left);
				func.instruction(&I::End);
			}
			Node::Key(left, op, right) if self.compares_structurally(op, left, right) => {
				self.emit_structural_equality(func, left, op, right);
			}
			Node::Key(left, op, right) if self.orders_a_list(op, left, right) => self.emit_unordered_list_error(func, left, op, right),
			Node::Key(left, op, right) if *op == Op::Xor || op.is_comparison() => {
				self.emit_int_operands_op(func, left, op, right);
			}
			// Prefix operators: √x, -x, !x, ‖x‖, #x (count)
			Node::Key(left, op, right) if op.is_prefix() && matches!(left.drop_meta(), Node::Empty) => self.emit_numeric_prefix(func, located, op, right),
			// `xs.size`, `xs.count`, `xs.bytes`: the raw i64 of the counting getter
			Node::Key(counted, Op::Dot, property) if self.counting_getter(property).is_some() => {
				let counter = self.counting_getter(property).expect("guarded");
				self.emit_list_count(func, counted, counter);
			}
			// Prefix # means count/length: #list returns element count
			Node::Key(left, Op::Hash, right) if matches!(left.drop_meta(), Node::Empty) => {
				self.emit_list_count(func, right, "node_count");
			}
			// Index operator: list#index (1-based)
			Node::Key(list, Op::Hash, index) => {
				if self.emit_struct_field_int(func, list, index) {
					return;
				}
				if self.map_is_indexed_by_key(index) {
					self.emit_indexed_node(func, list, index);
					self.emit_call(func, "get_int_value");
					return;
				}
				self.emit_list_element_number(func, list, index);
			}
			// Ternary operator: condition ? then_expr : else_expr
			Node::Key(condition, Op::Question, then_else) => {
				self.emit_ternary_numeric(func, condition, then_else);
			}
			// If-then-else: if condition then then_expr else else_expr
			Node::Key(if_then, Op::Else, else_expr) => {
				self.emit_if_then_else_numeric(func, if_then, Some(else_expr));
			}
			// If-then (no else): if condition then then_expr
			Node::Key(if_cond, Op::Then, then_expr) => {
				// Construct node for emit_if_then_else_numeric
				let full_node = Node::Key(if_cond.clone(), Op::Then, then_expr.clone());
				self.emit_if_then_else_numeric(func, &full_node, None);
			}
			// Variable lookup (local or global)
			Node::Symbol(name) => self.emit_numeric_symbol(func, name),
			// Statement sequence or function call
			Node::List(items, bracket, separator) if !items.is_empty() => self.emit_numeric_list(func, node, items, bracket, separator),
			// While loop: emit loop and get numeric result
			Node::Key(left, Op::Do, right) => {
				self.emit_while_loop_value(func, left, right);
			}
			Node::Key(value, Op::As, target) => self.emit_numeric_cast(func, located, value, target),
			other => self.emit_not_a_number(func, located, other),
		}
	}

	/// `floor(x)`, `count(list)`…: the builtin builds a node, its Int (raw i64 on the stack) is the number
	pub(super) fn emit_integer_builtin(&mut self, func: &mut Function, items: &[Node]) -> bool {
		if let [word, instance, type_name] = items {
			if let (Node::Symbol(name), Node::Text(type_name)) = (word.drop_meta(), type_name.drop_meta()) {
				if name == crate::traits::INSTANCE_OF {
					self.emit_instance_of(func, instance, type_name);
					return true;
				}
			}
		}
		if let [word, left, right, tolerance] = items {
			if crate::library_words::is_similarity_call(&word.drop_meta().name()) {
				self.emit_similarity(func, &word.drop_meta().name(), left, right, tolerance);
				return true;
			}
		}
		if let [word, dividend, divisor] = items {
			if matches!(word.drop_meta(), Node::Symbol(name) if name == crate::warp_parser::FLOOR_QUOTIENT) {
				self.emit_floor_quotient(func, dividend, divisor);
				return true;
			}
		}
		if let [Node::Symbol(fn_name), arguments @ ..] = items {
			if text_builtins::text_builtin_kind(fn_name, arguments.len()) == Some(Kind::Int) {
				self.emit_integer_text_builtin(func, fn_name, arguments);
				return true;
			}
		}
		let [word, argument] = items else { return false };
		let Node::Symbol(fn_name) = word.drop_meta() else { return false }; // `count xs` without parentheses keeps its position
		if self.emit_shadowed_counting(func, fn_name, argument) {
			return true;
		}
		let integer_builtin = ROUNDING_FUNCTIONS.contains(&fn_name.as_str())
			|| fn_name == crate::min_max::EMPTY_EXTREMUM_CALL
			|| fn_name == crate::switch::NO_CASE_CALL
			|| crate::analyzer::counting_function(fn_name, &self.ctx).is_some();
		if !integer_builtin || !self.emit_introspection_fn(func, fn_name, argument) {
			return false;
		}
		self.emit_call(func, "get_int_value");
		true
	}

	/// Emit the float value of a node onto the stack (as f64)
	/// Integers are converted to f64 for type upgrading
	pub(super) fn emit_float_value(&mut self, func: &mut Function, node: &Node) {
		self.note_position(node);
		if self.emit_raised_error(func, node) {
			return;
		}
		if let Some((list, sum_loop)) = list_dispatch::list_sum_parts(node) {
			return self.emit_list_sum(func, list, sum_loop, list_dispatch::Wanted::Float);
		}
		if self.emit_loop_jump(func, node) || self.emit_tuple_statement(func, node, Self::emit_float_value) {
			return;
		}
		// `return x` in a branch: the numeric path returns it (as the function's kind); the value after is never reached
		if matches!(node.drop_meta(), Node::List(items, _, _) if items.len() == 2 && matches!(items[0].drop_meta(), Node::Symbol(word) if word == "return")) {
			self.emit_numeric_value(func, node);
			func.instruction(&I::F64ConvertI64S);
			return;
		}
		if let Some(number) = crate::analyzer::spelled_number(node) {
			return self.emit_float_value(func, &number);
		}
		let located = node;
		let node = node.drop_meta();
		match node {
			Node::Number(num) => {
				match num {
					Number::Int(_) | Number::BigInt(_) => {
						let value: f64 = (*num).into();
						func.instruction(&I::F64Const(Ieee64::new(value.to_bits())));
					}
					Number::Float(f) => {
						func.instruction(&I::F64Const(Ieee64::new(f.to_bits())));
					}
					Number::Quotient(n, d) => {
						func.instruction(&I::F64Const(Ieee64::new((*n as f64 / *d as f64).to_bits())));
					}
					Number::BigQuotient(q) => {
						func.instruction(&I::F64Const(Ieee64::new(q.to_f64().to_bits())));
					}
					Number::Complex(r, _i) => {
						func.instruction(&I::F64Const(Ieee64::new(r.to_bits())));
					}
					Number::Real(r) => {
						func.instruction(&I::F64Const(Ieee64::new(r.to_f64().to_bits())));
					}
					Number::Nan => {
						func.instruction(&I::F64Const(Ieee64::new(f64::NAN.to_bits())));
					}
					Number::Inf => {
						func.instruction(&I::F64Const(Ieee64::new(f64::INFINITY.to_bits())));
					}
					Number::NegInf => {
						func.instruction(&I::F64Const(Ieee64::new(f64::NEG_INFINITY.to_bits())));
					}
				};
			}
			// `xs#i = 0.5`: the element's value after the assignment
			Node::Key(left, Op::Assign, right) if matches!(left.drop_meta(), Node::Key(_, Op::Hash, _)) => {
				self.emit_index_assignment_read_back(func, left, right, Self::emit_float_value);
			}
			// Variable definition/assignment: x:=42 or x=42
			Node::Key(left, Op::Define | Op::Assign, right) => {
				if let Node::Symbol(name) = left.drop_meta() {
					if let Some(position) = self.scope.lookup(name).map(|local| local.position) {
						let declared = self.declared_type_of(left);
						self.emit_declared_value(func, declared.as_ref(), right, Kind::Float);
						func.instruction(&I::LocalTee(position));
					} else if let Some(kind) = self.emit_global_store(func, name, right) {
						if !kind.is_float() {
							self.emit_int_to_f64(func, None);
						}
					} else {
						self.emit_float_value(func, right);
						self.emit_undefined_variable(func, name);
					}
				} else {
					self.emit_malformed(func, left, "a variable to assign to");
				}
			}
			// Arithmetic operators with float
			// `s += x` on a float variable, also as a statement of a loop body
			Node::Key(left, op, right) if op.is_compound_assign() && self.is_float_variable(left) => {
				self.emit_compound_assign(func, left, op, right, true);
			}
			Node::Key(left, op, right) if op.is_arithmetic() => {
				let kind = self.arithmetic_type(left, op, right);
				if self.emit_arithmetic_type_error(func, left, op, right, kind) {
					return;
				}
				if kind == Kind::Int && *op == Op::Div && self.int_runtime() && !self.wrapping_ints {
					self.emit_numeric_value(func, left);
					self.emit_numeric_value(func, right);
					self.emit_float_quotient(func);
					return;
				}
				if kind == Kind::Int {
					// a wholly exact expression is computed exactly and rounded once: `float y = 0.1+0.2` is 0.3
					self.emit_numeric_value(func, node);
					let range = self.int_range(node);
					self.emit_int_to_f64(func, range);
					return;
				}
				self.emit_float_value(func, left);
				self.emit_float_value(func, right);
				self.emit_float_arithmetic(func, op);
			}
			// Suffix operators: x² = x*x, x³ = x*x*x (returns f64)
			Node::Key(left, Op::Square, _) => {
				self.emit_float_value(func, left);
				self.emit_float_value(func, left);
				func.instruction(&I::F64Mul);
			}
			Node::Key(left, Op::Cube, _) => {
				self.emit_float_value(func, left);
				self.emit_float_value(func, left);
				func.instruction(&I::F64Mul);
				self.emit_float_value(func, left);
				func.instruction(&I::F64Mul);
			}
			// Prefix operators: √x, ∛x (return f64)
			Node::Key(left, root @ (Op::Sqrt | Op::Cbrt), right) if matches!(left.drop_meta(), Node::Empty) => {
				self.emit_float_value(func, right);
				self.emit_float_root(func, root);
			}
			// Prefix negation: -x (returns f64)
			Node::Key(left, Op::Neg, right) if matches!(left.drop_meta(), Node::Empty) => {
				func.instruction(&I::F64Const(0.0.into()));
				self.emit_float_value(func, right);
				func.instruction(&I::F64Sub);
			}
			// Prefix abs: ‖x‖ (returns f64)
			Node::Key(left, Op::Abs, right) if matches!(left.drop_meta(), Node::Empty) => {
				self.emit_float_value(func, right);
				func.instruction(&I::F64Abs);
			}
			// `v as float` is v's f64; any other cast's exact value converted
			Node::Key(value, Op::As, _) if self.get_type(node).is_float() && matches!(self.get_type(value), Kind::Text | Kind::Empty) && !matches!(value.drop_meta(), Node::Text(_) | Node::Char(_)) => {
				self.emit_node_instructions(func, value);
				self.emit_call(func, list_ops::TEXT_AS_FLOAT);
			}
			Node::Key(value, Op::As, _) if self.get_type(node).is_float() && !matches!(value.drop_meta(), Node::Text(_) | Node::Char(_)) => {
				self.emit_float_value(func, value);
			}
			// `"1.5" as float`: the literal's cast as everywhere (a number, or loud for "x"), its f64
			Node::Key(value, Op::As, _) if self.get_type(node).is_float() && matches!(value.drop_meta(), Node::Text(_)) => self.emit_node_as_f64(func, node),
			Node::Key(list, Op::Hash, index) if self.is_typed_list(list) => self.emit_typed_element_float(func, list, index),
			Node::Key(_, Op::As, _) => {
				self.emit_numeric_value(func, node);
				self.emit_int_to_f64(func, None);
			}
			// Variable lookup (local or global) - convert i64 to f64 if needed
			Node::Symbol(name) => self.emit_float_symbol(func, name),
			Node::List(items, bracket, separator) if !items.is_empty() => self.emit_float_list(func, items, bracket, separator),
			// a loop: run it, its count as the value (a float function leaves it by `return`)
			Node::Key(_, Op::Do, _) => {
				self.emit_numeric_value(func, node);
				func.instruction(&I::F64ConvertI64S);
			}
			Node::Key(condition, Op::Question, then_else) => self.emit_ternary_raw(func, condition, then_else, ValType::F64, Self::emit_float_value),
			Node::Key(if_then, Op::Else, else_expr) => self.emit_if_then_else_raw(func, if_then, Some(else_expr), ValType::F64, Self::emit_float_value),
			Node::Key(_, Op::Then, _) => self.emit_if_then_else_raw(func, node, None, ValType::F64, Self::emit_float_value),
			// an element or field read by name (`v.x` of an instance): a Node, an Int or a Float at run time
			Node::Key(_, Op::Hash, _) => self.emit_node_as_f64(func, node),
			other => self.emit_not_a_number(func, located, other),
		}
	}

	/// Emit a range as a list of integers
	/// inclusive: true for .../ (0...3 = [0,1,2,3]), false for .. (0..3 = [0,1,2])
	pub(super) fn emit_range(&mut self, func: &mut Function, start: &Node, end: &Node, inclusive: bool) {
		let (Node::Number(Number::Int(start_val)), Node::Number(Number::Int(end_val))) = (start.drop_meta(), end.drop_meta()) else {
			let bound = if matches!(start.drop_meta(), Node::Number(Number::Int(_))) { end } else { start };
			self.emit_malformed(func, bound, "a constant integer as range bound");
			return;
		};
		let (start_val, end_val) = (*start_val, *end_val);
		let actual_end = if inclusive { end_val + 1 } else { end_val };
		let items: Vec<Node> = (start_val..actual_end).map(|i| Node::Number(Number::Int(i))).collect();
		if items.is_empty() {
			self.emit_call(func, "new_empty");
			return;
		}
		self.emit_list_structure(func, &items, &Bracket::Square);
	}

	/// Emit a list as linked cons cells
	pub(super) fn emit_list_structure(&mut self, func: &mut Function, items: &[Node], bracket: &Bracket) {
		self.emit_list_structure_with(func, items, bracket, Self::emit_node_instructions);
	}

	/// A node as data, nothing evaluated: `data 1+2` is the Key 1+2, `x : a+b` shown is the block a+b
	pub(super) fn emit_literal(&mut self, func: &mut Function, node: &Node) {
		match node.drop_meta() {
			Node::Symbol(name) => self.emit_string_call(func, name, "new_symbol"),
			Node::Key(left, op, right) => {
				self.emit_literal(func, left);
				self.emit_literal(func, right);
				self.emit_new_key(func, op);
			}
			Node::List(items, bracket, _) if !items.is_empty() => self.emit_list_structure_with(func, items, bracket, Self::emit_literal),
			Node::List(..) | Node::Empty => self.emit_call(func, "new_empty"),
			other => self.emit_node_instructions(func, other),
		}
	}

	pub(super) fn emit_list_structure_with(&mut self, func: &mut Function, items: &[Node], bracket: &Bracket, emit: fn(&mut Self, &mut Function, &Node)) {
		let bracket_info = bracket_info(bracket);

		// Emit first item
		emit(self, func, &items[0]);

		// Emit rest as a proper linked list
		// The value field must always be a list node (or null), never an element directly
		if items.len() > 1 {
			// Recursively build the rest of the list
			// This ensures proper cons-cell structure: (data=first, value=list_node_for_rest)
			self.emit_list_structure_with(func, &items[1..], bracket, emit);
		} else {
			// Single element list: rest is null
			self.emit_node_null(func);
		}

		// bracket_info
		func.instruction(&I::I64Const(bracket_info));

		// Call new_list if available, otherwise inline struct.new
		if self.ctx.func_registry.contains("new_list") {
			self.emit_call(func, "new_list");
		} else {
			// Inline: kind = (bracket_info << 8) | List
			// We need to reconstruct since stack has: first, rest, bracket_info
			// Actually we need to reorder. Let's use locals.
			// For simplicity, always emit new_list function when needed
			self.emit_inline_list(func, bracket_info);
		}
	}

	pub(super) fn emit_inline_list(&mut self, func: &mut Function, _bracket_info: i64) {
		// Stack: first, rest, bracket_info
		// Need: kind, data(first), value(rest)
		// Use struct.new directly with proper ordering

		// This is complex due to stack order. For now, require new_list function.
		// Pop bracket_info (already on stack as i64)
		// Compute kind
		func.instruction(&I::I64Const(8));
		func.instruction(&I::I64Shl);
		self.emit_kind(func, Kind::List);
		func.instruction(&I::I64Or);
		// But now we have: first, rest, kind - wrong order!
		// We need: kind, first, rest
		// This requires locals or restructuring.

		// new_list is always emitted; reaching this is a compiler bug, reported as an error value
		self.emit_type_error(func, "internal error: new_list is not available".to_string());
	}
}

/// `int`, `Int`, `integer`: the type `x as int` converts to
fn is_int_type_word(target: &Node) -> bool {
	matches!(crate::type_kinds::canonical_type_name(&target.drop_meta().name().to_lowercase()), "int" | "integer")
}
