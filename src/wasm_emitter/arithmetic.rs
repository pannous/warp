//! Arithmetic: operators, assignments and compound assignments, int and float operations, logical operators

use super::*;

impl WasmGcEmitter {
	/// Emit arithmetic operation: evaluate operands and apply operator
	pub(super) fn emit_arithmetic(&mut self, func: &mut Function, left: &Node, op: &Op, right: &Node) {
		if let Some(assignment) = self.global_update_as_assignment(left, op, right) {
			self.emit_node_instructions(func, &assignment);
			return;
		}
		if op.is_shift() {
			self.emit_shift(func, left, op, right);
			self.emit_call(func, "new_int");
			return;
		}
		// `s++` of a text is `s += 1`, "a" → "a1" (card inc-text); the int step below holds an i64 local only
		if matches!(op, Op::Inc | Op::Dec) && !matches!(self.get_type(left), Kind::Int | Kind::Float) {
			self.emit_node_instructions(func, &crate::library_words::stepped(left.clone(), *op));
			return;
		}
		if op.is_arithmetic() && self.emit_typed_arithmetic(func, left, op, right) {
			return;
		}
		if let Some(function) = crate::analyzer::node_arithmetic(self.get_type(left), op, self.get_type(right)) {
			self.emit_node_instructions(func, left);
			self.emit_node_instructions(func, right);
			self.emit_call(func, function);
			return;
		}
		let use_float = self.should_use_float(left, right, op);

		if self.emit_assign_or_define(func, left, op, right, use_float) {
			if use_float {
				self.emit_call(func, "new_float");
			} else {
				self.emit_call(func, "new_int");
			}
			return;
		}
		if self.emit_inc_dec(func, left, op) {
			self.emit_call(func, "new_int");
			return;
		}
		if self.emit_compound_assign(func, left, op, right, use_float) {
			if use_float {
				self.emit_call(func, "new_float");
			} else {
				self.emit_call(func, "new_int");
			}
			return;
		}

		let wrap = if use_float {
			if self.emit_float_truthy_logical(func, left, op, right) {
				return;
			}
			self.emit_float_binary(func, left, op, right)
		} else {
			if self.emit_int_truthy_logical(func, left, op, right) {
				return;
			}
			self.emit_int_binary(func, left, op, right)
		};

		self.wrap_arithmetic_result(func, wrap);
	}

	pub(super) fn should_use_float(&self, left: &Node, right: &Node, op: &Op) -> bool {
		self.arithmetic_type(left, op, right).is_float()
	}

	pub(super) fn emit_assign_or_define(
		&mut self,
		func: &mut Function,
		left: &Node,
		op: &Op,
		right: &Node,
		use_float: bool,
	) -> bool {
		if *op != Op::Define && *op != Op::Assign {
			return false;
		}

		// Check for string assignment in WASI mode - skip emit (tracked in Local)
		if self.config.emit_wasi_imports && matches!(right.drop_meta(), Node::Text(_)) {
			// String data stored in Local's data_pointer/data_length, just emit 0
			func.instruction(&I::I64Const(0));
			return true;
		}

		if let Node::Symbol(name) = left.drop_meta() {
			if self.scope.lookup(name).is_none() {
				if let Some(global_kind) = self.emit_global_store(func, name, right) {
					match (global_kind.is_float(), use_float) {
						(true, false) => self.emit_float_in_exact_context(func, name),
						(false, true) => self.emit_int_to_f64(func, None),
						_ => {}
					}
					return true;
				}
			}
		}

		// x:=42 or x=42 → emit value, store to local, return value
		let declared = self.declared_type_of(left);
		if use_float {
			self.emit_declared_value(func, declared.as_ref(), right, Kind::Float);
		} else {
			self.emit_declared_value(func, declared.as_ref(), right, Kind::Int);
			self.emit_fits_declared(func, left);
		}
		if let Node::Symbol(name) = left.drop_meta() {
			if let Some(local) = self.scope.lookup(name) {
				func.instruction(&I::LocalTee(local.position));
			} else {
				self.emit_undefined_variable(func, name);
			}
		} else {
			self.emit_malformed(func, left, "a variable to assign to");
		}
		true
	}

	pub(super) fn emit_inc_dec(&mut self, func: &mut Function, left: &Node, op: &Op) -> bool {
		if *op != Op::Inc && *op != Op::Dec {
			return false;
		}

		// i++ → i = i + 1 (returns new value)
		// i-- → i = i - 1 (returns new value)
		if let Node::Symbol(name) = left.drop_meta() {
			let Some(local_pos) = self.defined_local_position(func, name) else { return true };
			self.emit_int_step(func, local_pos, self.int_range(left), op);
			self.emit_fits_declared(func, left);
			// Store and return new value
			func.instruction(&I::LocalTee(local_pos));
			true
		} else {
			self.emit_malformed(func, left, "a variable to increment or decrement");
			true
		}
	}

	/// A local holding a float, or a global (`global b` in a function) that no local of the same name shadows
	pub(super) fn is_float_variable(&self, node: &Node) -> bool {
		let Node::Symbol(name) = node.drop_meta() else { return false };
		match self.scope.lookup(name) {
			Some(local) => local.kind.is_float(),
			None => self.ctx.user_globals.get(name).is_some_and(|(_, kind)| kind.is_float()),
		}
	}

	pub(super) fn emit_compound_assign(
		&mut self,
		func: &mut Function,
		left: &Node,
		op: &Op,
		right: &Node,
		use_float: bool,
	) -> bool {
		if !op.is_compound_assign() {
			return false;
		}

		if self.emit_compound_index_assignment(func, left, op, right) {
			return true;
		}
		if let Some(assignment) = self.global_update_as_assignment(left, op, right) {
			if use_float { self.emit_float_value(func, &assignment) } else { self.emit_numeric_value(func, &assignment) }
			return true;
		}
		// x += y → x = x + y
		if let Node::Symbol(name) = left.drop_meta() {
			let Some(local_pos) = self.defined_local_position(func, name) else { return true };
			let base_op = op.base_op();
			if !use_float && self.emit_compound_type_error(func, left, &base_op, right) {
				return true;
			}
			// Get current value of x
			func.instruction(&I::LocalGet(local_pos));
			// Emit y
			if use_float {
				self.emit_float_value(func, right);
			} else {
				self.emit_numeric_value(func, right);
			}
			// Apply base operation
			if use_float {
				self.emit_float_arithmetic(func, &base_op);
			} else {
				self.emit_int_compound_op(func, &base_op, right);
				self.emit_fits_declared(func, left);
			}
			// Store result and leave on stack
			func.instruction(&I::LocalTee(local_pos));
			true
		} else {
			self.emit_malformed(func, left, "a variable to update");
			true
		}
	}

	/// `√x`, `-x`, `!x`, `‖x‖` where an Int is wanted
	pub(super) fn emit_numeric_prefix(&mut self, func: &mut Function, located: &Node, op: &Op, right: &Node) {
		match op {
			Op::Sqrt if self.get_type(right) == Kind::Codepoint => {
				self.emit_type_error(func, format!("type error: √ of a codepoint: {}", list_ops::CHARACTER_IS_NO_NUMBER));
			}
			root @ (Op::Sqrt | Op::Cbrt) => {
				self.emit_float_value(func, right);
				self.emit_float_root(func, root);
				self.emit_float_in_exact_context(func, &located.serialize());
			}
			Op::Neg => {
				self.emit_arithmetic_operand(func, right);
				let range = self.int_range(right);
				self.emit_int_neg(func, range);
			}
			Op::Not => {
				// not x = x is falsy: 0, ø and errors
				self.emit_condition(func, right, Self::emit_numeric_value);
				func.instruction(&I::I32Eqz);
				func.instruction(&I::I64ExtendI32U);
			}
			Op::Abs => {
				self.emit_numeric_value(func, right);
				let range = self.int_range(right);
				self.emit_int_abs(func, range);
			}
			_ => self.emit_type_error(func, format!("`{op}` of an exact number is not supported yet")),
		}
	}

	/// A variable where an Int is wanted: a local, a global or `$n` (the n-th parameter)
	pub(super) fn emit_numeric_symbol(&mut self, func: &mut Function, name: &str) {
		// Handle $n parameter reference (e.g., $0 = first param), unless a parameter is named so: Swift's closure
		// `{ $0 + k }` is lifted to closure_lambda(k, $0)
		if let Some(rest) = name.strip_prefix('$').filter(|_| self.scope.lookup(name).is_none()) {
			if let Ok(idx) = rest.parse::<u32>() {
				func.instruction(&I::LocalGet(idx));
				return;
			}
		}
		if self.emit_typed_list_as_node(func, name) {
			self.emit_call(func, "get_int_value");
		} else if let Some(local) = self.scope.lookup(name) {
			func.instruction(&I::LocalGet(local.position));
			self.emit_as_numeric(func, name, local.kind);
		} else if let Some(&(idx, kind)) = self.ctx.user_globals.get(name) {
			func.instruction(&I::GlobalGet(idx));
			self.emit_as_numeric(func, name, kind);
		} else {
			self.emit_undefined_variable(func, name);
		}
	}

	/// The variable `name` of `kind` just read, as a number
	fn emit_as_numeric(&mut self, func: &mut Function, name: &str, kind: Kind) {
		if kind.is_ref() {
			// an optional held as a Node, checked non-ø before use (analyzer::check_null_use); a handler's abort value
			self.emit_call(func, "get_int_value");
		} else if kind.is_float() {
			self.emit_float_in_exact_context(func, name);
		}
	}

	/// A list where an Int is wanted: a call (of a user, FFI or builtin function), `return v`, or statements
	pub(super) fn emit_numeric_list(&mut self, func: &mut Function, node: &Node, items: &[Node], bracket: &Bracket, separator: &Separator) {
		if self.emit_task_check(func, items) {
			func.instruction(&I::I64Const(0));
			return;
		}
		// Check for return statement: [Symbol("return"), value]
		if items.len() == 2 {
			if let Node::Symbol(keyword) = items[0].drop_meta() {
				if keyword == "return" {
					// Emit the return value, as the Node a Node-returning function gives back
					self.emit_returned_value(func, &items[1]);
					func.instruction(&I::Return);
					// After return, emit unreachable to satisfy block types
					func.instruction(&I::I64Const(0));
					return;
				}
			}
		}
		// Integer conversion: int("5") + 3
		if items.len() == 2 && self.get_type(node) == Kind::Int && matches!(items[0].drop_meta(), Node::Symbol(s) if type_word_kind(s) == Some(Kind::Int)) {
			self.emit_cast(func, &items[1], &items[0]);
			self.emit_call(func, "get_int_value");
			return;
		}
		// a call of a user or FFI function: `(f)` without arguments, `f(a, b…)`
		if items.len() >= 2 || *bracket == Bracket::Round {
			if let Node::Symbol(fn_name) = items[0].drop_meta() {
				if self.ctx.user_functions.contains_key(fn_name) {
					self.emit_user_function_call_numeric(func, fn_name, &items[1..]);
					return;
				}
				// Check for FFI function call
				if self.ctx.ffi_imports.contains_key(fn_name) {
					self.emit_ffi_call(func, fn_name, &items[1..], Some(Kind::Int));
					return;
				}
			}
		}
		// Rounding and counting builtins build a node: its Int is the number
		if self.emit_integer_builtin(func, items) || self.reject_unresolved_call(func, items, bracket, separator) {
			return;
		}
		// `puti x`, `print x`: the emitter's own builtins give a node whose Int is the number
		if self.is_output_call(node) {
			self.emit_node_instructions(func, node);
			self.emit_call(func, "get_int_value");
			return;
		}
		// a library word gives a node too: `m.get(k, 0) + 1`, `xs.has(x) + 1`
		if crate::analyzer::call_name(items, bracket, separator).is_some_and(crate::library_words::is_runtime_word) {
			self.emit_node_instructions(func, node);
			self.emit_call(func, "get_int_value");
			return;
		}
		self.emit_statement_sequence(func, items, Self::emit_numeric_value);
	}

	/// A variable where an f64 is wanted: an Int converted
	pub(super) fn emit_float_symbol(&mut self, func: &mut Function, name: &str) {
		if self.emit_typed_list_as_node(func, name) {
			self.emit_call(func, "get_int_value");
			self.emit_int_to_f64(func, None);
		} else if let Some(local) = self.scope.lookup(name) {
			func.instruction(&I::LocalGet(local.position));
			if local.kind.is_ref() {
				self.emit_held_node_as_f64(func); // a Node of run-time kind: a Float stays one
			} else if !local.kind.is_float() {
				self.emit_int_to_f64(func, None);
			}
		} else if let Some(&(idx, kind)) = self.ctx.user_globals.get(name) {
			func.instruction(&I::GlobalGet(idx));
			if !kind.is_float() {
				self.emit_int_to_f64(func, None);
			}
		} else {
			self.emit_undefined_variable(func, name);
		}
	}

	/// A list where an f64 is wanted: a call of an FFI or user function (`(f)` without arguments too), a builtin, or
	/// statements whose last one gives the value
	pub(super) fn emit_float_list(&mut self, func: &mut Function, items: &[Node], bracket: &Bracket, separator: &Separator) {
		if self.is_library_word_call(items, bracket, separator) {
			return self.emit_node_as_f64(func, &Node::List(items.to_vec(), bracket.clone(), separator.clone()));
		}
		if items.len() >= 2 || *bracket == Bracket::Round {
			if let Node::Symbol(fn_name) = items[0].drop_meta() {
				if self.ctx.ffi_imports.contains_key(fn_name) {
					self.emit_ffi_call(func, fn_name, &items[1..], Some(Kind::Float));
					return;
				}
				if self.ctx.user_functions.contains_key(fn_name) {
					self.emit_user_function_call_float(func, fn_name, &items[1..]);
					return;
				}
				// `float(x)` is `x as float` (a text parses its digits)
				if let [type_word, value] = items {
					if crate::type_kinds::canonical_type_name(fn_name) == "float" {
						return self.emit_float_value(func, &Node::Key(Box::new(value.clone()), Op::As, Box::new(type_word.clone())));
					}
				}
			}
		}
		if self.emit_integer_builtin(func, items) {
			self.emit_int_to_f64(func, None);
			return;
		}
		if self.reject_unresolved_call(func, items, bracket, separator) {
			return;
		}
		// the statements run as numbers, the last one gives the f64
		let (last, statements) = items.split_last().expect("a list of items");
		for statement in statements {
			// a nested definition has no code here, its captures are taken (as in emit_statement_sequence)
			if self.is_definition(statement) {
				if let Some(name) = self.defined_function_name(statement) {
					self.emit_closure_capture(func, &name);
				}
				continue;
			}
			self.emit_discarded_statement(func, statement, Self::emit_numeric_value);
			func.instruction(&I::Drop);
		}
		self.emit_float_value(func, last);
	}

	/// `x = v` where an Int is wanted: the value stored and left on the stack (0 for a Node variable holding a non-number)
	pub(super) fn emit_numeric_assignment(&mut self, func: &mut Function, located: &Node, left: &Node, right: &Node) {
		if let Node::Key(node_expr, Op::Hash, index_expr) = left.drop_meta() {
			self.emit_index_assignment(func, node_expr, index_expr, right);
			return;
		}
		if let Node::Symbol(name) = left.drop_meta() {
			if let Some((position, kind)) = self.scope.lookup(name).map(|local| (local.position, local.kind)) {
				if kind.is_float() {
					let message = format!("{} assigns a float where an exact Int is expected", located.serialize());
					self.emit_type_error(func, message);
					return;
				}
				if self.emit_typed_list_store(func, name, right) {
					func.instruction(&I::Drop);
					func.instruction(&I::I64Const(0));
					return;
				}
				if kind.is_ref() {
					// a variable holding ø, a list or a text: the value lives in the local, a number is also the value of the assignment
					self.emit_node_instructions(func, right);
					func.instruction(&I::LocalSet(position));
					if self.get_type(right) == Kind::Int {
						self.emit_numeric_value(func, left);
					} else {
						func.instruction(&I::I64Const(0));
					}
					return;
				}
				let declared = self.declared_type_of(left);
				self.emit_declared_value(func, declared.as_ref(), right, kind);
				self.emit_fits_declared(func, left);
				func.instruction(&I::LocalTee(position));
			} else if let Some(kind) = self.emit_global_store(func, name, right) {
				if kind.is_float() {
					self.emit_float_in_exact_context(func, name);
				}
			} else {
				self.emit_undefined_variable(func, name);
			}
		} else {
			self.emit_malformed(func, left, "a variable to assign to");
		}
	}

	/// `i++`, `i--` where an Int is wanted: the new value
	pub(super) fn emit_numeric_step(&mut self, func: &mut Function, left: &Node, op: &Op, right: &Node) {
		if let Some(assignment) = self.global_update_as_assignment(left, op, right) {
			self.emit_numeric_value(func, &assignment);
			return;
		}
		if let Node::Symbol(name) = left.drop_meta() {
			let Some(local_pos) = self.defined_local_position(func, name) else { return };
			self.emit_int_step(func, local_pos, self.int_range(left), op);
			self.emit_fits_declared(func, left);
			// Store and return new value
			func.instruction(&I::LocalTee(local_pos));
		} else {
			self.emit_malformed(func, left, "a variable to increment or decrement");
		}
	}

	/// `s += x` of a number s and a text x, or `s -= 1` of a text s (card text-crashes): the type error `s = s op x`
	/// is, never the text's code points added
	pub(super) fn emit_compound_type_error(&mut self, func: &mut Function, left: &Node, base_op: &Op, right: &Node) -> bool {
		let kind = self.arithmetic_type(left, base_op, right);
		let numeric_target = matches!(self.get_type(left), Kind::Int | Kind::Float);
		(kind == Kind::Error || numeric_target && kind == Kind::Text) && self.emit_arithmetic_type_error(func, left, base_op, right, kind)
	}

	/// Stack [x, y] → [x op y] for `x op= y` on Ints; `/=` keeps an integer x an integer
	pub(super) fn emit_int_compound_op(&mut self, func: &mut Function, op: &Op, right: &Node) {
		match op {
			Op::And => {
				func.instruction(&I::I64And);
			}
			Op::Or => {
				func.instruction(&I::I64Or);
			}
			Op::Div => self.emit_call(func, "exact_div_assign"),
			Op::Add | Op::Sub | Op::Mul | Op::Mod | Op::Pow | Op::Xor => {
				let right_range = self.int_range(right);
				self.emit_int_op(func, op, None, right_range);
			}
			_ => self.emit_type_error(func, format!("`{op}=` is not an operator on exact numbers")),
		}
	}

	pub(super) fn emit_float_truthy_logical(
		&mut self,
		func: &mut Function,
		left: &Node,
		op: &Op,
		right: &Node,
	) -> bool {
		if *op != Op::And && *op != Op::Or {
			return false;
		}

		// and: a falsy left is the value, else right; or: a truthy left is the value, else right. Left is evaluated once.
		self.emit_held_left_is_falsy(func, left, true);
		func.instruction(&I::If(BlockType::Result(ValType::F64)));
		self.emit_logical_branch(func, op == &Op::And, right, true);
		func.instruction(&I::Else);
		self.emit_logical_branch(func, op == &Op::Or, right, true);
		func.instruction(&I::End);
		self.emit_call(func, "new_float");
		true
	}

	/// Push i32 1 when `left` is falsy (0 or 0.0), keeping its value in a scratch local (a float as its bits)
	pub(super) fn emit_held_left_is_falsy(&mut self, func: &mut Function, left: &Node, is_float: bool) {
		let held = self.scratch(LOGICAL_SCRATCH);
		if is_float {
			self.emit_float_value(func, left);
			Self::emit_list(func, &[I::I64ReinterpretF64, I::LocalTee(held), I::F64ReinterpretI64]);
			Self::emit_list(func, &[I::F64Const(Ieee64::new(0.0f64.to_bits())), I::F64Eq]);
		} else {
			self.emit_numeric_value(func, left);
			Self::emit_list(func, &[I::LocalTee(held), I::I64Eqz]);
		}
	}

	/// The held left value, raw
	pub(super) fn emit_held_left(&self, func: &mut Function, is_float: bool) {
		func.instruction(&I::LocalGet(self.scratch(LOGICAL_SCRATCH)));
		if is_float {
			func.instruction(&I::F64ReinterpretI64);
		}
	}

	/// One branch of a numeric and/or: the held left value or right, raw
	pub(super) fn emit_logical_branch(&mut self, func: &mut Function, is_left: bool, right: &Node, is_float: bool) {
		match (is_left, is_float) {
			(true, _) => self.emit_held_left(func, is_float),
			(false, true) => self.emit_float_value(func, right),
			(false, false) => self.emit_numeric_value(func, right),
		}
	}

	pub(super) fn emit_int_truthy_logical(
		&mut self,
		func: &mut Function,
		left: &Node,
		op: &Op,
		right: &Node,
	) -> bool {
		if *op != Op::And && *op != Op::Or {
			return false;
		}

		self.emit_held_left_is_falsy(func, left, false);
		func.instruction(&I::If(BlockType::Result(ValType::I64)));
		self.emit_logical_branch(func, op == &Op::And, right, false);
		func.instruction(&I::Else);
		self.emit_logical_branch(func, op == &Op::Or, right, false);
		func.instruction(&I::End);
		self.emit_call(func, "new_int");
		true
	}

	/// Apply an arithmetic operator to the two f64 on the stack. Mod is euclidean like the exact Ints, Rem truncates.
	pub(super) fn emit_float_arithmetic(&mut self, func: &mut Function, op: &Op) {
		match op {
			Op::Add => {
				func.instruction(&I::F64Add);
			}
			Op::Sub => {
				func.instruction(&I::F64Sub);
			}
			Op::Mul => {
				func.instruction(&I::F64Mul);
			}
			Op::Div => {
				func.instruction(&I::F64Div);
			}
			Op::Mod | Op::Rem => self.emit_float_remainder(func, *op == Op::Mod),
			Op::Pow => self.emit_float_power(func),
			_ => self.emit_type_error(func, format!("`{op}` is not an operator on floats")),
		}
	}

	pub(super) fn push_float_scratch(&self, func: &mut Function, index: u32) {
		func.instruction(&I::LocalGet(self.scratch(index)));
		func.instruction(&I::F64ReinterpretI64);
	}

	pub(super) fn pop_float_scratch(&self, func: &mut Function, index: u32) {
		func.instruction(&I::I64ReinterpretF64);
		func.instruction(&I::LocalSet(self.scratch(index)));
	}

	/// `a - |b| * floor(a / |b|)` (euclidean) or `a - b * trunc(a / b)`; the f64 operands live in the i64 scratch locals as bits
	pub(super) fn emit_float_remainder(&mut self, func: &mut Function, euclidean: bool) {
		let (dividend, divisor) = (0, 1);
		self.pop_float_scratch(func, divisor);
		self.pop_float_scratch(func, dividend);
		let push_divisor = |emitter: &Self, func: &mut Function| {
			emitter.push_float_scratch(func, divisor);
			if euclidean {
				func.instruction(&I::F64Abs);
			}
		};
		self.push_float_scratch(func, dividend);
		self.push_float_scratch(func, dividend);
		push_divisor(self, func);
		func.instruction(&I::F64Div);
		func.instruction(if euclidean { &I::F64Floor } else { &I::F64Trunc });
		push_divisor(self, func);
		func.instruction(&I::F64Mul);
		func.instruction(&I::F64Sub);
	}

	/// base ^ exponent through libm's pow; a NaN (negative base with a fractional exponent) traps as invalid_number
	/// Call a libm function (`m.pow`), imported once the emitter has found it is needed: false while it is not yet
	pub(super) fn emit_libm_call(&mut self, func: &mut Function, key: &'static str) -> bool {
		let Some(index) = self.ffi_func_index(key) else {
			self.discovered_needs.insert(Need::MathImport(key));
			func.instruction(&I::Unreachable);
			return false;
		};
		func.instruction(&I::Call(index));
		true
	}

	/// f64 → its √ (a wasm instruction) or ∛ (libm's cbrt)
	pub(super) fn emit_float_root(&mut self, func: &mut Function, root: &Op) {
		match root {
			Op::Cbrt => {
				self.emit_libm_call(func, LIBM_CBRT);
			}
			_ => {
				func.instruction(&I::F64Sqrt);
			}
		}
	}

	pub(super) fn emit_float_power(&mut self, func: &mut Function) {
		if !self.emit_libm_call(func, LIBM_POW) {
			return;
		}
		self.pop_float_scratch(func, 0);
		self.push_float_scratch(func, 0);
		self.push_float_scratch(func, 0);
		func.instruction(&I::F64Ne);
		self.emit_fail_if(func, "invalid_number");
		self.push_float_scratch(func, 0);
	}

	pub(super) fn emit_float_binary(&mut self, func: &mut Function, left: &Node, op: &Op, right: &Node) -> ArithmeticWrap {
		self.emit_float_value(func, left);
		self.emit_float_value(func, right);

		match op {
			op if op.is_comparison() => {
				self.emit_float_comparison(func, op);
				return ArithmeticWrap::Int;
			}
			op => self.emit_float_arithmetic(func, op),
		}

		ArithmeticWrap::Float
	}

	pub(super) fn emit_int_binary(&mut self, func: &mut Function, left: &Node, op: &Op, right: &Node) -> ArithmeticWrap {
		self.emit_int_operands_op(func, left, op, right);
		ArithmeticWrap::Int
	}

	/// Emit both operands as Ints and apply an arithmetic, xor or comparison operator
	pub(super) fn emit_int_operands_op(&mut self, func: &mut Function, left: &Node, op: &Op, right: &Node) {
		// `s < "b"`, `c >= "0"` with a text: ordered by code points (node_order, as sort orders them); a value held as a
		// Node (a cell's, another runtime's: `time.time() > 0`) by its value, an Int or a Float decided at run time
		// (an element `xs#2` that is a character compares by its code point with a number, `x#2 > 50`)
		let [left_kind, right_kind] = [left, right].map(|side| self.get_type(side));
		let held = |kind: Kind| matches!(kind, Kind::Empty | Kind::Data);
		if op.is_ordering() && [left_kind, right_kind].iter().any(|&kind| kind == Kind::Text || held(kind)) {
			for (side, kind, other_kind) in [(left, left_kind, right_kind), (right, right_kind, left_kind)] {
				self.emit_node_instructions(func, side);
				if held(kind) && !matches!(other_kind, Kind::Text | Kind::Codepoint) && matches!(side.drop_meta(), Node::Key(_, Op::Hash, _)) {
					self.emit_codepoint_as_int_node(func);
				}
			}
			if self.emit_bare_uncertain_order(func, left, *op, right) {
				return;
			}
			self.emit_call(func, library_ops::NODE_ORDER);
			let sign_test = match op {
				Op::Lt => I::I32LtS,
				Op::Gt => I::I32GtS,
				Op::Le => I::I32LeS,
				_ => I::I32GeS,
			};
			Self::emit_list(func, &[I::I32Const(0), sign_test, I::I64ExtendI32U]);
			return;
		}
		if op.is_comparison() && self.should_use_float(left, right, op) {
			self.emit_float_value(func, left);
			self.emit_float_value(func, right);
			self.emit_float_comparison(func, op);
			return;
		}
		// comparisons keep a character's code point (`c >= '0'`); arithmetic refuses it (P65)
		let operand = if op.is_comparison() { Self::emit_numeric_value } else { Self::emit_arithmetic_operand };
		operand(self, func, left);
		operand(self, func, right);
		let (left_range, right_range) = (self.int_range(left), self.int_range(right));
		if op.is_comparison() {
			self.emit_int_compare(func, op, left_range, right_range);
			func.instruction(&I::I64ExtendI32U);
		} else {
			self.emit_int_op(func, op, left_range, right_range);
			let width = crate::fixed_width::wider(self.declared_fixed_width(left), self.declared_fixed_width(right));
			self.emit_fixed_width_check(func, width);
		}
	}

	/// `local ± 1` for i++ / i--
	pub(super) fn emit_int_step(&mut self, func: &mut Function, local_pos: u32, range: super::big_int::IntRange, op: &Op) {
		func.instruction(&I::LocalGet(local_pos));
		func.instruction(&I::I64Const(1));
		let step = if *op == Op::Inc { Op::Add } else { Op::Sub };
		self.emit_int_op(func, &step, range, Some((1, 1)));
	}

	pub(super) fn wrap_arithmetic_result(&mut self, func: &mut Function, wrap: ArithmeticWrap) {
		match wrap {
			ArithmeticWrap::Int => self.emit_call(func, "new_int"),
			ArithmeticWrap::Float => self.emit_call(func, "new_float"),
		}
	}

	/// Emit truthy logical operations (and/or) when operands may be non-numeric
	/// Returns a Node reference based on short-circuit evaluation
	pub(super) fn emit_truthy_logical(&mut self, func: &mut Function, left: &Node, op: &Op, right: &Node) {
		let node_ref = RefType {
			nullable: false,
			heap_type: HeapType::Concrete(self.type_manager.node_type),
		};

		// A call of number kind is tested at run time like a numeric variable, and evaluated once
		let is_call = matches!(left.drop_meta(), Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))));
		let left_is_numeric = self.is_numeric(left) || (is_call && matches!(self.get_type(left), Kind::Int | Kind::Float));

		if left_is_numeric {
			let is_float = self.get_type(left).is_float();
			self.emit_held_left_is_falsy(func, left, is_float);
			if *op == Op::Or {
				func.instruction(&I::I32Eqz); // or: a truthy left is the value
			}
			func.instruction(&I::If(BlockType::Result(Ref(node_ref))));
			self.emit_held_left(func, is_float);
			self.emit_call(func, if is_float { "new_float" } else { "new_int" });
			func.instruction(&I::Else);
			self.emit_node_instructions(func, right);
			func.instruction(&I::End);
		} else if Self::is_run_time_value(left) || Self::is_operation(left) {
			// a variable, call, element or operation (`a or b`) holding a Node: its truthiness is known at run time only
			let held = self.node_scratch();
			self.emit_node_instructions(func, left);
			func.instruction(&I::LocalTee(held));
			self.emit_call(func, IS_TRUTHY);
			if *op == Op::And {
				func.instruction(&I::I32Eqz); // and: a falsy left is the value
			}
			func.instruction(&I::If(BlockType::Result(Ref(node_ref))));
			Self::emit_list(func, &[I::LocalGet(held), I::RefAsNonNull, I::Else]);
			self.emit_node_instructions(func, right);
			func.instruction(&I::End);
		} else {
			// a literal left is falsy or not at compile time: `[] or 3`, `"a" and 4`
			let left_is_value = left.is_falsy() == (*op == Op::And);
			self.emit_node_instructions(func, if left_is_value { left } else { right });
		}
	}

	/// `a or b`, `(x is rgb or 0)`: an operation, also in parentheses, computed at run time; a pair `a: 1` is data
	fn is_operation(node: &Node) -> bool {
		match node.drop_meta() {
			Node::Key(_, op, _) => *op != Op::Colon,
			Node::List(items, Bracket::Round, _) => matches!(items.as_slice(), [single] if Self::is_operation(single)),
			_ => false,
		}
	}

	/// A name, call, element or field: its value exists only at run time
	pub(super) fn is_run_time_value(node: &Node) -> bool {
		match node.drop_meta() {
			Node::Symbol(_) => true,
			Node::List(items, Bracket::Round, _) => matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))),
			Node::Key(_, Op::Hash | Op::Dot, _) => true,
			_ => false,
		}
	}

	/// The f64 of a number Node computed at run time: a Float's value, an Int converted
	pub(super) fn emit_node_as_f64(&mut self, func: &mut Function, node: &Node) {
		self.emit_node_instructions(func, node);
		self.emit_held_node_as_f64(func);
	}

	/// The Node on the stack as f64: a Float's value, an Int's converted
	pub(super) fn emit_held_node_as_f64(&mut self, func: &mut Function) {
		let (held, node_type, float_box) = (self.node_scratch(), self.type_manager.node_type, self.type_manager.f64_box_type);
		Self::emit_list(func, &[I::LocalTee(held), I::StructGet { struct_type_index: node_type, field_index: 0 }]);
		Self::emit_list(func, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::Float as i64), I::I64Eq]);
		Self::emit_list(func, &[I::If(BlockType::Result(ValType::F64)), I::LocalGet(held), I::RefAsNonNull]);
		Self::emit_list(func, &[I::StructGet { struct_type_index: node_type, field_index: 1 }, I::RefCastNonNull(HeapType::Concrete(float_box))]);
		Self::emit_list(func, &[I::StructGet { struct_type_index: float_box, field_index: 0 }, I::Else, I::LocalGet(held), I::RefAsNonNull]);
		self.emit_call(func, "get_int_value");
		self.emit_int_to_f64(func, None);
		func.instruction(&I::End);
	}

	/// The Node on the stack, a character replaced by the Int of its code point: ordered against a number (`x#2 > 50`)
	fn emit_codepoint_as_int_node(&mut self, func: &mut Function) {
		let (held, node_type) = (self.node_scratch(), self.type_manager.node_type);
		let node_ref = RefType { nullable: false, heap_type: HeapType::Concrete(node_type) };
		Self::emit_list(func, &[I::LocalTee(held), I::StructGet { struct_type_index: node_type, field_index: 0 }]);
		Self::emit_list(func, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::Codepoint as i64), I::I64Eq]);
		Self::emit_list(func, &[I::If(BlockType::Result(Ref(node_ref))), I::LocalGet(held), I::RefAsNonNull]);
		self.emit_call(func, "get_int_value");
		self.emit_call(func, "new_int");
		Self::emit_list(func, &[I::Else, I::LocalGet(held), I::RefAsNonNull, I::End]);
	}
}
