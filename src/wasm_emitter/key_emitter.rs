//! Key node emission - handles all Key(left, op, right) patterns

use crate::node::{Bracket, Node, Separator};
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

		// `k.x = 3` (k = field_with(k, "x", 3)) of a k bound nowhere, like `k#1 = 3`
		if let Node::Symbol(name) = left.drop_meta() {
			if *op == Op::Assign && self.is_unbound(name) && crate::library_words::is_field_update_of(name, right) {
				return self.emit_undefined_variable(func, name);
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

		// text += more → text = text + more, list += [x] of a field or global → list = list + [x]; a list (or ø) += anything
		// else is what list + it is (the type error of `xs + 3`)
		if *op == Op::AddAssign && self.emit_list_extend_assignment(func, left, right) {
			return;
		}
		if op.is_compound_assign() && op.base_op() == Op::Add {
			let concatenation = Node::Key(Box::new(left.clone()), Op::Add, Box::new(right.clone()));
			let list_or_text = |kind| matches!(kind, crate::type_kinds::Kind::Text | crate::type_kinds::Kind::List);
			// `xs = []` holds ø until something is added
			let left_kind = self.get_type(left);
			if list_or_text(self.get_type(&concatenation)) || matches!(left_kind, crate::type_kinds::Kind::List | crate::type_kinds::Kind::Empty) {
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
				let declared = self.declared_type_of(left);
				self.emit_declared_value(func, declared.as_ref(), right, crate::Kind::Empty);
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
				if !self.is_exact_int_element(node_expr, right) {
					return self.emit_index_assignment_read_back(func, left, right, Self::emit_node_instructions);
				}
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
			root @ (Op::Sqrt | Op::Cbrt) => {
				self.emit_float_value(func, right);
				self.emit_float_root(func, root);
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
					self.emit_arithmetic_operand(func, right);
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
				self.emit_new_key(func, op);
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

	/// The runtime counter behind the getter word in `x.size`, `x.count`, `x.bytes`, or the call `x.count()` (`x.chars()`
	/// is the list of characters)
	pub(super) fn counting_getter(&self, property: &Node) -> Option<&'static str> {
		match property.drop_meta() {
			Node::Symbol(word) => crate::analyzer::counting_method(word, &self.ctx),
			Node::List(call, Bracket::Round, _) => match call.as_slice() {
				[word] if crate::analyzer::is_counting_property(&word.drop_meta().name()) => crate::analyzer::counting_method(&word.drop_meta().name(), &self.ctx),
				_ => None,
			},
			_ => None,
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
		self.emit_new_key(func, &Op::Dot);
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
		if let (Node::List(items, Bracket::Curly, _), false) = (right_node, *op == Op::Colon) {
			// the fields as data, nothing run as a block; the instance keeps its braces: `point{x:1 y:2}` (P123)
			match items.is_empty() {
				true => self.emit_call(func, "new_empty"),
				false => self.emit_list_structure(func, items, &Bracket::Curly),
			}
		} else if *op == Op::Colon {
			// the value of an entry is data: unknown words in it stay words (P62)
			let outer = std::mem::replace(&mut self.data_context, true);
			// a field value keeps the mark of its declared type (declared_values::with_declared_field_types)
			let checked = super::declared_values::declared_field_value(right).is_some();
			self.emit_node_instructions(func, if checked { right } else { right_node });
			self.data_context = outer;
			if has_list_children(left, right_node) {
				self.emit_call(func, super::list_ops::ELEMENT_BODY);
			}
		} else {
			self.emit_node_instructions(func, right_node);
		}
		// Preserve the op for roundtrip
		self.emit_new_key(func, op);
	}

	/// `xs += [v]` of a list variable, what `xs.add(v)` lowers to: the list grows in place, every holder of it sees the
	/// new items (P200b); a typed list pushes them onto its array, a Node list adds copies of their cells (list_extend)
	fn emit_list_extend_assignment(&mut self, func: &mut Function, left: &Node, right: &Node) -> bool {
		let Node::Symbol(name) = left.drop_meta() else { return self.emit_member_list_extend(func, left, right) };
		if let Node::List(items, Bracket::Square, _) = right.drop_meta() {
			if self.emit_typed_list_extend(func, name, items) {
				func.instruction(&I::Drop);
				self.emit_typed_list_as_node(func, name);
				return true;
			}
		}
		let is_list = matches!(self.get_type(left), crate::Kind::List | crate::Kind::Empty) && self.get_type(right) == crate::Kind::List;
		let Some(position) = self.scope.lookup(name).filter(|local| local.kind.is_ref()).map(|local| local.position) else { return false };
		if !is_list || !self.should_emit_function(super::list_ops::LIST_EXTEND) {
			return false;
		}
		self.emit_node_instructions(func, left);
		self.emit_node_instructions(func, right);
		self.emit_call(func, super::list_ops::LIST_EXTEND);
		func.instruction(&I::LocalTee(position));
		true
	}

	/// `p.items += [v]` (lowered `p#(items+1) += [v]`), `m#2 += [v]`: the list a field or item holds grows in place too
	/// (P200b), the place taking what list_extend gives back (the new list where it held none)
	fn emit_member_list_extend(&mut self, func: &mut Function, left: &Node, right: &Node) -> bool {
		let is_member = matches!(left.drop_meta(), Node::Key(_, Op::Dot | Op::Hash, _));
		// a field read of no declared element type is of unknown kind (Empty)
		if !is_member || !matches!(self.get_type(left), crate::Kind::List | crate::Kind::Empty) || self.get_type(right) != crate::Kind::List
			|| !self.should_emit_function(super::list_ops::LIST_EXTEND) {
			return false;
		}
		let extended = Node::List(vec![Node::Symbol(super::list_ops::LIST_EXTEND.to_string()), left.clone(), right.clone()], Bracket::Round, Separator::None);
		self.emit_key_node(func, left, &Op::Assign, &extended);
		true
	}
}

/// `ul{ h2{…} [li{f} for f in fruits] }`: an element whose children hold a list, spliced into them at run time
fn has_list_children(tag: &Node, children: &Node) -> bool {
	let is_element = matches!(tag.drop_meta(), Node::Symbol(name) if crate::markup::is_element_tag(name));
	is_element && matches!(children, Node::List(items, Bracket::Curly, _) if items.iter().any(|item| matches!(item.drop_meta(), Node::List(_, Bracket::Square, _))))

}
