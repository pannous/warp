//! Errors found while emitting: type errors, malformed constructs, undefined names and variables, located at the statement being emitted
use super::*;

impl WasmGcEmitter {
	/// The first type error found while emitting; the module must not run then
	pub fn type_error(&self) -> Option<Node> {
		self.type_errors.first().or(self.type_manager.type_errors.first()).map(|message| crate::node::error(message))
	}

	/// A value used as a number that has none (ø, a list, a type…): an error value at its source position, not a panic
	pub(super) fn emit_not_a_number(&mut self, func: &mut Function, located: &Node, value: &Node) {
		let diagnostic = crate::errors::diagnostic::Diagnostic::at(
			located,
			format!("cannot extract a numeric value from {}", value.serialize()),
		);
		self.emit_type_error(func, diagnostic.to_string());
	}

	/// A construct whose shape the emitter cannot handle: an error value at its source position, not a panic
	pub(super) fn emit_malformed(&mut self, func: &mut Function, located: &Node, expected: &str) {
		let diagnostic = crate::errors::diagnostic::Diagnostic::at(
			located,
			format!("expected {expected}, got {}", located.serialize()),
		);
		self.emit_type_error(func, diagnostic.to_string());
	}

	pub(super) fn emit_type_error(&mut self, func: &mut Function, message: String) {
		let message = self.located(message);
		self.type_errors.push(message);
		func.instruction(&I::Unreachable);
	}

	/// The latest source position among the nodes being emitted: where an emitter error points
	pub(super) fn note_position(&mut self, node: &Node) {
		if let Some(info) = node.get_lineinfo() {
			self.source_position = Some(info);
		}
	}

	/// An emitter error at the source position of the statement being emitted, unless it names one already
	pub(super) fn located(&self, message: String) -> String {
		match &self.source_position {
			Some(position) => crate::errors::diagnostic::located_at(message, position),
			None => message,
		}
	}

	/// A variable named like a function of the program is the variable: a parameter (`f(inc) := inc(3)`), and inside a
	/// function its locals too (`angle = 0.5; cos(angle)` beside math's angle), and a loop's variable in main (`[str f for f in xs]`).
	/// main also holds a local for each `sq := it * 2`, which is the function. A nested function's capture of its
	/// enclosing body's local is a variable too (`held = …; part = i => held`)
	pub(super) fn is_variable(&self, name: &str) -> bool {
		let captured = (self.compiling.as_ref().and_then(|function| self.ctx.captures.get(function)))
			.is_some_and(|captures| captures.iter().any(|(captured, _)| captured == name));
		captured
			|| self.scope.lookup(name).is_some_and(|local| {
				local.is_param || self.compiling.is_some() || self.loop_variables.contains(name)
			})
	}

	/// A name that is no variable, global, function or `$n` parameter here
	pub(super) fn is_unbound(&self, name: &str) -> bool {
		!name.starts_with('$')
			&& !crate::type_tests::is_builtin_type_word(name)
			&& self.scope.lookup(name).is_none()
			&& !self.ctx.user_globals.contains_key(name)
			&& !self.ctx.user_functions.contains_key(name)
	}

	/// `x is 5` of a name defined nowhere: `is` compares (P61, user: "educate the user to use the be key word for
	/// definitions"), so the error teaches the definition `x be 5`
	pub(super) fn emit_undefined_comparison(&mut self, func: &mut Function, node: &Node) -> bool {
		let Node::Key(subject, Op::Eq, value) = node.drop_meta() else {
			return false;
		};
		let Node::Symbol(name) = subject.drop_meta() else {
			return false;
		};
		if !self.is_unbound(name) {
			return false;
		}
		let value = crate::type_tests::compared_text(subject)
			.unwrap_or_else(|| value.drop_meta().serialize().trim().to_string());
		let message = format!("undefined variable: {name}; `is` compares, a definition is written `{name} be {value}`");
		let diagnostic = self.diagnostic_here(message).offer(
			"a definition",
			format!("{name} is {value}"),
			format!("{name} be {value}"),
		);
		self.emit_type_error(func, diagnostic.remembered());
		true
	}

	/// An operand of arithmetic as an i64: a character is no number there (P65): a character written or held is the type
	/// error, an element read whose kind shows only at run time (`x#2` of `[1,'a']`) fails with not_a_number
	pub(crate) fn emit_arithmetic_operand(&mut self, func: &mut Function, operand: &Node) {
		match operand.drop_meta() {
			_ if matches!(self.get_type(operand), Kind::Codepoint | Kind::Text) => {
				let kind = self.get_type(operand);
				self.emit_type_error(
					func,
					format!("type error: {kind} in arithmetic: {}", lists::list_ops::CHARACTER_IS_NO_NUMBER),
				);
			}
			// an array of Ints or Floats holds no character: its fast read stays
			Node::Key(list, Op::Hash, index)
				if !list.is_empty_node() && !self.holds_only_numbers(list) && !self.map_is_indexed_by_key(index) =>
			{
				self.emit_list_element_node(func, list, index);
				let held = self.node_scratch();
				Self::emit_list(
					func,
					&[
						I::LocalTee(held),
						I::RefAsNonNull,
						I::StructGet { struct_type_index: self.type_manager.node_type, field_index: 0 },
					],
				);
				Self::emit_kind_test(func, Kind::Codepoint as i64, I::I64Eq);
				self.emit_fail_if(func, lists::list_ops::NOT_A_NUMBER);
				Self::emit_list(func, &[I::LocalGet(held), I::RefAsNonNull]);
				self.emit_call(func, "get_int_value");
			}
			_ => self.emit_numeric_value(func, operand),
		}
	}

	pub(super) fn emit_undefined_variable(&mut self, func: &mut Function, name: &str) {
		// a unit reaches the emitter only where quantities are not computed yet (notes/units_runtime.md)
		let unit_hint = if crate::numbers::units::is_unit(name) { UNIT_AT_RUN_TIME } else { "" };
		let diagnostic =
			self.offer_near_name(self.diagnostic_here(format!("undefined variable: {name}{unit_hint}")), name);
		self.emit_type_error(func, diagnostic.remembered());
	}

	/// A diagnostic at the source position of the statement being emitted
	pub(super) fn diagnostic_here(&self, message: String) -> crate::errors::diagnostic::Diagnostic {
		let position = self.source_position.clone().unwrap_or_default();
		crate::errors::diagnostic::Diagnostic {
			message,
			line: position.line_nr,
			column: position.column,
			file: position.file,
			..Default::default()
		}
	}

	/// The local slot of a defined variable; an undefined one is an error value at the use
	pub(super) fn defined_local_position(&mut self, func: &mut Function, name: &str) -> Option<u32> {
		let position = self.scope.lookup(name).map(|local| local.position);
		if position.is_none() {
			self.emit_undefined_variable(func, name);
		}
		position
	}
}
