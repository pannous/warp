//! Ternaries, if/then/else and while loops

use super::*;

impl WasmGcEmitter {
	/// Emit ternary expression: condition ? then_expr : else_expr
	/// Returns a Node reference, handling mixed-type branches (numbers, strings, etc.)
	pub(super) fn emit_ternary(&mut self, func: &mut Function, condition: &Node, then_else: &Node) {
		// Structure: condition ? Key(then, Colon, else)
		let Node::Key(then_expr, Op::Colon, else_expr) = then_else.drop_meta() else {
			self.emit_malformed(func, then_else, TERNARY_BRANCHES);
			return;
		};

		// Evaluate condition and convert to i32 for if instruction
		self.emit_condition(func, condition, Self::emit_numeric_value);

		// if (condition) { then_expr } else { else_expr }
		func.instruction(&I::If(BlockType::Result(Ref(self.node_ref(false)))));

		// Then branch - use emit_node_instructions to handle any type (Text, Number, etc.)
		self.emit_node_instructions(func, then_expr);

		func.instruction(&I::Else);

		// Else branch - use emit_node_instructions to handle any type
		self.emit_node_instructions(func, else_expr);

		func.instruction(&I::End);
	}

	/// Emit ternary expression returning i64: condition ? then_expr : else_expr
	pub(super) fn emit_ternary_numeric(&mut self, func: &mut Function, condition: &Node, then_else: &Node) {
		self.emit_ternary_raw(func, condition, then_else, ValType::I64, Self::emit_numeric_value);
	}

	/// `c ? a : b` as a raw number, each branch by `emit` (emit_numeric_value i64, emit_float_value f64)
	pub(super) fn emit_ternary_raw(&mut self, func: &mut Function, condition: &Node, then_else: &Node, value_type: ValType, emit: fn(&mut Self, &mut Function, &Node)) {
		// Structure: condition ? Key(then, Colon, else)
		let Node::Key(then_expr, Op::Colon, else_expr) = then_else.drop_meta() else {
			self.emit_malformed(func, then_else, TERNARY_BRANCHES);
			return;
		};
		self.emit_raw_branches(func, condition, then_expr, Some(else_expr), value_type, emit);
	}

	/// Emit if-then-else returning i64: if condition then then_expr else else_expr
	pub(super) fn emit_if_then_else_numeric(&mut self, func: &mut Function, left: &Node, else_expr: Option<&Node>) {
		self.emit_if_then_else_raw(func, left, else_expr, ValType::I64, Self::emit_numeric_value);
	}

	/// `if c then a else b` as a raw number, each branch by `emit`; without else the value is 0
	pub(super) fn emit_if_then_else_raw(&mut self, func: &mut Function, left: &Node, else_expr: Option<&Node>, value_type: ValType, emit: fn(&mut Self, &mut Function, &Node)) {
		// Structure: Key(Key(Empty, If, condition), Then, then_expr)
		let Some((condition, then_expr)) = if_then_parts(left) else {
			self.emit_malformed(func, left, IF_THEN);
			return;
		};
		self.emit_raw_branches(func, condition, then_expr, else_expr, value_type, emit);
	}

	pub(super) fn emit_raw_branches(&mut self, func: &mut Function, condition: &Node, then_expr: &Node, else_expr: Option<&Node>, value_type: ValType, emit: fn(&mut Self, &mut Function, &Node)) {
		self.emit_condition(func, condition, Self::emit_numeric_value);
		func.instruction(&I::If(BlockType::Result(value_type)));
		emit(self, func, then_expr);
		func.instruction(&I::Else);
		match else_expr {
			Some(else_node) => emit(self, func, else_node),
			None => emit(self, func, &Node::int(0)),
		}
		func.instruction(&I::End);
	}

	/// Emit if-then-else expression: if condition then then_expr [else else_expr]
	/// Structure: Key(Key(Key(Empty, If, condition), Then, then_expr), Else, else_expr)
	/// Or for if-then without else: Key(Key(Empty, If, condition), Then, then_expr)
	pub(super) fn emit_if_then_else(&mut self, func: &mut Function, left: &Node, else_expr: Option<&Node>) {
		// Extract condition and then_expr from structure
		// Structure: Key(Key(Empty, If, condition), Then, then_expr)
		let Some((condition, then_expr)) = if_then_parts(left) else {
			self.emit_malformed(func, left, IF_THEN);
			return;
		};

		// A branch yielding a text, character, list or error (`if x {x} else {"offline"}`): both branches are Node values
		let branch_value = |branch: &Node| match branch.drop_meta() {
			Node::List(items, Bracket::Curly, _) if items.len() == 1 => items[0].clone(),
			other => other.clone(),
		};
		let then_value = branch_value(then_expr);
		let else_value = else_expr.map(branch_value);
		// a block yields its last statement; get_type sees capture globals in user_globals, where branch_kind(scope)
		// alone types a free var as Symbol (g-rT0c)
		let branch_type = |emitter: &Self, value: &Node| match value.drop_meta() {
			Node::List(statements, Bracket::Curly, _) if !statements.is_empty() => emitter.get_type(&statements[statements.len() - 1]),
			other => emitter.get_type(other),
		};
		let is_node_valued = |emitter: &Self, value: &Node| {
			let kind = branch_type(emitter, value);
			emitter.is_structured_value(value) || kind.is_ref() || kind == Kind::Codepoint
		};
		if is_node_valued(self, &then_value) || else_value.as_ref().is_some_and(|value| is_node_valued(self, value)) {
			self.emit_condition(func, condition, Self::emit_block_value);
			func.instruction(&I::If(BlockType::Result(Ref(self.node_ref(true))))); // a local holds a nullable ref
			self.emit_node_instructions(func, &then_value);
			func.instruction(&I::Else);
			self.emit_node_instructions(func, else_value.as_ref().unwrap_or(&Node::Empty));
			func.instruction(&I::End);
			func.instruction(&I::RefAsNonNull);
			return;
		}
		// a float branch (`if c then {x} else {0}` of a float x): both branches as floats, the value its Float node
		let is_float_valued = |emitter: &Self, value: &Node| branch_type(emitter, value) == Kind::Float;
		if is_float_valued(self, &then_value) || else_value.as_ref().is_some_and(|value| is_float_valued(self, value)) {
			self.emit_if_then_else_raw(func, left, else_expr, ValType::F64, Self::emit_float_value);
			self.emit_call(func, "new_float");
			return;
		}

		// Evaluate condition and convert to i32 for if instruction
		self.emit_condition(func, condition, Self::emit_block_value);

		// if (condition) { then_expr } else { else_expr }
		func.instruction(&I::If(BlockType::Result(Ref(self.node_ref(false)))));

		// Then branch - extract value from block if needed
		self.emit_block_value(func, then_expr);
		self.emit_call(func, "new_int");

		func.instruction(&I::Else);

		// Else branch - use provided else_expr or default to 0
		if let Some(else_node) = else_expr {
			self.emit_block_value(func, else_node);
		} else {
			func.instruction(&I::I64Const(0));
		}
		self.emit_call(func, "new_int");

		func.instruction(&I::End);
	}

	/// Emit while loop: (while condition) do body
	/// If wrap_result is true, wraps result in Node; otherwise returns raw i64
	pub(super) fn emit_while_loop_impl(&mut self, func: &mut Function, left: &Node, body: &Node, wrap_result: bool) {
		let Node::Key(_, Op::While, condition) = left.drop_meta() else {
			self.emit_malformed(func, left, "`while condition`");
			return;
		};

		let result_local = self.next_temp_local;
		self.next_temp_local += 1;

		let ran_local = self.next_temp_local;
		self.next_temp_local += 1;

		func.instruction(&I::I64Const(0));
		func.instruction(&I::LocalSet(result_local));
		func.instruction(&I::I64Const(0));
		func.instruction(&I::LocalSet(ran_local));

		func.instruction(&I::Block(BlockType::Empty));
		let break_frame = loop_control::open_control_frames(func);
		func.instruction(&I::Loop(BlockType::Empty));
		// a program that controls tasks: a task paused in the browser waits here (host task_poll)
		if let Some(poll) = self.ffi_func_index(crate::host::TASK_POLL) {
			func.instruction(&I::Call(poll));
		}
		// a program with `on interrupt {…}`: a ctrl-c runs its handler here (host signal_poll)
		if let Some(poll) = self.ffi_func_index(crate::host::SIGNAL_POLL) {
			func.instruction(&I::Call(poll));
		}

		self.emit_condition(func, condition, Self::emit_block_value);
		func.instruction(&I::I32Eqz);
		func.instruction(&I::BrIf(1));

		func.instruction(&I::I64Const(1));
		func.instruction(&I::LocalSet(ran_local));
		let (statements, step) = loop_control::split_step(body);
		// the step of a `for` loop (its counter's increment) runs after a body ending in a number without becoming
		// the loop's value
		let step_apart = step.is_some() && matches!(&statements, Node::List(items, _, _) if self.ends_in_number(items));
		if loop_control::has_jump(body) {
			func.instruction(&I::Block(BlockType::Empty));
			self.enter_loop(break_frame, loop_control::open_control_frames(func));
			self.emit_loop_body(func, &statements, result_local);
			self.leave_loop();
			func.instruction(&I::End);
		} else {
			self.emit_loop_body(func, if step_apart || step.is_none() { &statements } else { body }, result_local);
		}
		match &step {
			Some(step) if step_apart => {
				self.emit_block_value(func, step);
				func.instruction(&I::Drop);
			}
			Some(step) if loop_control::has_jump(body) => self.emit_loop_body(func, step, result_local),
			_ => {}
		}
		func.instruction(&I::Br(0));

		func.instruction(&I::End);
		func.instruction(&I::End);

		if wrap_result {
			// the value of a loop is its last body value; a loop whose body never ran is empty
			func.instruction(&I::LocalGet(ran_local));
			func.instruction(&I::I32WrapI64);
			func.instruction(&I::If(BlockType::Result(Ref(self.node_ref(false)))));
			func.instruction(&I::LocalGet(result_local));
			self.emit_call(func, "new_int");
			func.instruction(&I::Else);
			self.emit_call(func, "new_empty");
			func.instruction(&I::End);
		} else {
			func.instruction(&I::LocalGet(result_local));
		}
	}

	/// One pass of a loop body; a numeric value becomes the loop's value
	pub(super) fn emit_loop_body(&mut self, func: &mut Function, body: &Node, result_local: u32) {
		let statements = match body.drop_meta() {
			Node::List(items, Bracket::Curly, _) => items.clone(),
			other => vec![other.clone()],
		};
		if statements.last().is_some_and(|last| self.is_float_assignment(last)) {
			// `while … { …; x = x * 0.5 }` of a float x: run the statements, the loop's value stays 0
			for statement in &statements {
				self.emit_discarded_statement(func, statement, Self::emit_numeric_value);
				func.instruction(&I::Drop);
			}
			return;
		}
		if body.is_nothing() { // `while c {}` only spins: nothing to evaluate
		} else if self.get_type(body).is_ref() && !self.ends_in_number(&statements) { // `while c { s += "a" }`: a text or list update, its value is not a number
			self.emit_node_instructions(func, body);
			func.instruction(&I::Drop);
		} else {
			self.emit_block_value(func, body);
			func.instruction(&I::LocalSet(result_local));
		}
	}

	pub(super) fn emit_while_loop(&mut self, func: &mut Function, left: &Node, body: &Node) {
		self.emit_while_loop_impl(func, left, body, true);
	}

	pub(super) fn emit_while_loop_value(&mut self, func: &mut Function, left: &Node, body: &Node) {
		self.emit_while_loop_impl(func, left, body, false);
	}
}
