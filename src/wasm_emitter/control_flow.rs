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
		let then_expr = self.live_then(condition, then_expr, else_expr);
		self.emit_raw_branches(func, condition, &then_expr, else_expr, value_type, emit);
	}

	pub(super) fn emit_raw_branches(&mut self, func: &mut Function, condition: &Node, then_expr: &Node, else_expr: Option<&Node>, value_type: ValType, emit: fn(&mut Self, &mut Function, &Node)) {
		self.emit_condition(func, condition, Self::emit_numeric_value);
		func.instruction(&I::If(BlockType::Result(value_type)));
		self.emit_raw_branch(func, then_expr, emit);
		func.instruction(&I::Else);
		match else_expr {
			Some(else_node) => self.emit_raw_branch(func, else_node, emit),
			None => emit(self, func, &Node::int(0)),
		}
		func.instruction(&I::End);
	}

	/// A ø branch where a number is wanted (`y = if c then 5 else ø`, an emit no handler may answer) fails when it runs,
	/// not when the program compiles: the other branch may be the only one taken
	fn emit_raw_branch(&mut self, func: &mut Function, branch: &Node, emit: fn(&mut Self, &mut Function, &Node)) {
		if matches!(branch.drop_meta(), Node::Empty) {
			self.emit_runtime_error(func, super::list_ops::NOT_A_NUMBER);
		} else {
			emit(self, func, branch);
		}
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
		let then_expr = &self.live_then(condition, then_expr, else_expr);

		// A branch yielding a text, character, list or error (`if x {x} else {"offline"}`): both branches are Node values
		let branch_value = |branch: &Node| match branch.drop_meta() {
			Node::List(items, Bracket::Curly, _) if items.len() == 1 => items[0].clone(),
			_ => branch.clone(), // keeps a construction's Instance mark
		};
		let then_value = branch_value(then_expr);
		let else_value = else_expr.map(branch_value);
		// a block yields its last statement; get_type sees capture globals in user_globals, where branch_kind(scope)
		// alone types a free var as Symbol (g-rT0c)
		let branch_type = |emitter: &Self, value: &Node| emitter.get_type(&crate::analyzer::block_result(value).unwrap_or_else(|| value.clone()));
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
			Self::emit_list(func, &[I::End, I::RefAsNonNull]);
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

	/// The then branch, or the else branch in its place when the condition is a class test the subject can never pass:
	/// a match arm `Circle(r) => r * r` of `s = Dot` is dead, and typed with s (a symbol) `s#r` would be a character
	/// (card match-static). The condition is still evaluated, and false.
	fn live_then(&self, condition: &Node, then_expr: &Node, else_expr: Option<&Node>) -> Node {
		match self.never_an_instance(condition) {
			true => else_expr.cloned().unwrap_or(Node::int(0)),
			false => then_expr.clone(),
		}
	}

	/// `is_type(x, "Circle")` of a declared class, x statically a number, text or symbol
	fn never_an_instance(&self, condition: &Node) -> bool {
		let Node::List(items, _, _) = last_statement(condition).drop_meta() else { return false };
		let [head, subject, spec] = items.as_slice() else { return false };
		let is_class_test = head.drop_meta().name() == crate::type_tests::IS_TYPE
			&& matches!(spec.drop_meta(), Node::Text(class) if self.ctx.type_registry.get_by_name(class).is_some());
		is_class_test && matches!(self.get_type(subject), Kind::Int | Kind::Float | Kind::Text | Kind::Symbol | Kind::Codepoint)
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

		Self::emit_list(func, &[I::I64Const(0), I::LocalSet(result_local), I::I64Const(0), I::LocalSet(ran_local)]);

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
		Self::emit_list(func, &[I::I32Eqz, I::BrIf(1)]);

		Self::emit_list(func, &[I::I64Const(1), I::LocalSet(ran_local)]);
		let (statements, step) = loop_control::split_step(body);
		// a body ending in a text or list: that value is the loop's (P55)
		let value = if wrap_result && self.ends_in_reference(&statements) { self.loop_value(&statements) } else { LoopValue::Number };
		let value_local = match value { LoopValue::Held(local) => Some(local), _ => None };
		// the step of a `for` loop (its counter's increment) runs after a body ending in a number or a held value
		// without becoming the loop's value
		let step_apart = step.is_some() && (value_local.is_some() || matches!(&statements, Node::List(items, _, _) if self.ends_in_number(items)));
		if loop_control::has_jump(body) {
			func.instruction(&I::Block(BlockType::Empty));
			self.enter_loop(break_frame, loop_control::open_control_frames(func));
			self.emit_loop_body(func, &statements, result_local, value_local);
			self.leave_loop();
			func.instruction(&I::End);
		} else {
			self.emit_loop_body(func, if step_apart || step.is_none() { &statements } else { body }, result_local, value_local);
		}
		match &step {
			Some(step) if step_apart => {
				self.emit_block_value(func, step);
				func.instruction(&I::Drop);
			}
			Some(step) if loop_control::has_jump(body) => self.emit_loop_body(func, step, result_local, None),
			_ => {}
		}
		func.instruction(&I::Br(0));

		Self::emit_list(func, &[I::End, I::End]);

		if wrap_result {
			// the value of a loop is its last body value; a loop whose body never ran is empty
			Self::emit_list(func, &[I::LocalGet(ran_local), I::I32WrapI64, I::If(BlockType::Result(Ref(self.node_ref(false))))]);
			match value {
				LoopValue::Held(local) => {
					Self::emit_list(func, &[I::LocalGet(local), I::RefAsNonNull]);
					self.loop_values.depth -= 1;
				}
				LoopValue::Updated(variable) => self.emit_node_instructions(func, &variable),
				LoopValue::Number => {
					func.instruction(&I::LocalGet(result_local));
					self.emit_call(func, "new_int");
				}
			}
			func.instruction(&I::Else);
			self.emit_call(func, "new_empty");
			func.instruction(&I::End);
		} else {
			func.instruction(&I::LocalGet(result_local));
		}
	}

	/// One pass of a loop body; a numeric value becomes the loop's value, a text or list one too when held in
	/// value_local
	pub(super) fn emit_loop_body(&mut self, func: &mut Function, body: &Node, result_local: u32, value_local: Option<u32>) {
		let statements = match body.drop_meta() {
			Node::List(items, Bracket::Curly, _) => items.clone(),
			other => vec![other.clone()],
		};
		if let Some((last, before)) = statements.split_last().filter(|(last, _)| self.is_float_assignment(last)) {
			// `while … { …; x = x * 0.5 }` of a float x: the statements run, the float is the loop's value when held
			for statement in before {
				self.emit_discarded_statement(func, statement, Self::emit_numeric_value);
				func.instruction(&I::Drop);
			}
			match value_local {
				Some(local) => {
					self.emit_node_instructions(func, last);
					func.instruction(&I::LocalSet(local));
				}
				None => {
					self.emit_discarded_statement(func, last, Self::emit_numeric_value);
					func.instruction(&I::Drop);
				}
			}
			return;
		}
		if body.is_nothing() { // `while c {}` only spins: nothing to evaluate
		} else if self.ends_in_reference(body) { // `while c { s += "a" }`: a text or list update, its value is not a number
			self.emit_node_instructions(func, body);
			func.instruction(&value_local.map_or(I::Drop, I::LocalSet));
		} else {
			self.emit_block_value(func, body);
			func.instruction(&I::LocalSet(result_local));
		}
	}

	/// A loop body whose value is a text, character, list, float or other Node, not an exact number (a character held as
	/// a number would show its code point, a float its truncation; card loop-float)
	fn ends_in_reference(&self, body: &Node) -> bool {
		let statements = match body.drop_meta() {
			Node::List(items, Bracket::Curly, _) => items.clone(),
			other => vec![other.clone()],
		};
		let kind = self.get_type(body);
		let ends_in_float = kind == Kind::Float || statements.last().is_some_and(|last| self.get_type(last) == Kind::Float);
		!body.is_nothing() && !loop_control::ends_in_jump(body) && ((kind.is_ref() || ends_in_float) && !self.ends_in_number(&statements) || kind == Kind::Codepoint)
	}

	/// Declares the Node locals the loops of `body` hold their values in, after node_scratch; gives the enclosing
	/// function's, to restore when this one is done
	pub(super) fn declare_loop_values(&mut self, locals: &mut Vec<(u32, ValType)>, body: &Node) -> LoopValues {
		let declared = loop_nesting(body);
		if declared > 0 {
			locals.push((declared, Ref(self.node_ref(true))));
		}
		std::mem::replace(&mut self.loop_values, LoopValues { declared, depth: 0 })
	}

	/// Where the value of a loop whose body ends in a text or list comes from
	fn loop_value(&mut self, statements: &Node) -> LoopValue {
		match last_statement(statements).drop_meta() {
			// `xs = xs + [i]`, `s += c`: the variable after the loop, read once
			Node::Key(variable, op, _) if (*op == Op::Assign || op.is_compound_assign()) && matches!(variable.drop_meta(), Node::Symbol(_)) => {
				LoopValue::Updated(variable.drop_meta().clone())
			}
			_ => self.take_loop_value_local().map_or(LoopValue::Number, LoopValue::Held),
		}
	}

	/// The Node local of the next nesting level, none in a function that declared too few (its loop keeps a number)
	fn take_loop_value_local(&mut self) -> Option<u32> {
		let LoopValues { declared, depth } = self.loop_values;
		(depth < declared).then(|| {
			self.loop_values.depth += 1;
			self.node_scratch() + NODE_SCRATCH_LOCALS + depth
		})
	}

	pub(super) fn emit_while_loop(&mut self, func: &mut Function, left: &Node, body: &Node) {
		self.emit_while_loop_impl(func, left, body, true);
	}

	pub(super) fn emit_while_loop_value(&mut self, func: &mut Function, left: &Node, body: &Node) {
		self.emit_while_loop_impl(func, left, body, false);
	}
}

/// The Node locals holding the values of loops whose bodies end in a text or list, one per nesting level
#[derive(Default, Clone, Copy)]
pub(super) struct LoopValues {
	declared: u32,
	depth: u32,
}

/// The value of a loop: a number (the last numeric body value), a Node held each pass, or an updated variable read
/// after the loop
enum LoopValue {
	Number,
	Held(u32),
	Updated(Node),
}

/// How deeply loops nest in node: the loop value locals it needs
fn loop_nesting(node: &Node) -> u32 {
	match node.drop_meta() {
		Node::Key(left, op, right) => u32::from(*op == Op::Do) + loop_nesting(left).max(loop_nesting(right)),
		Node::List(items, _, _) => items.iter().map(loop_nesting).max().unwrap_or(0),
		_ => 0,
	}
}

/// `(x = s; is_type x "Circle")`: the statement whose value a block is
fn last_statement(node: &Node) -> &Node {
	match node.drop_meta() {
		Node::List(items, _, Separator::Semicolon | Separator::Newline) if !items.is_empty() => last_statement(&items[items.len() - 1]),
		Node::List(items, Bracket::Round, _) if items.len() == 1 => last_statement(&items[0]),
		_ => node,
	}
}
