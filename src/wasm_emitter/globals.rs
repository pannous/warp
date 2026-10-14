//! Raised errors, fetch calls and globals: declarations, updates, storage of a global's kind

use super::*;

/// The export label of a program global copied into a task's instance (after CAPTURE_EXPORT_PREFIX)
const GLOBAL_EXPORT_LABEL: &str = "global·";

impl WasmGcEmitter {
	/// The one nullable Node local of every function, after its int scratch locals
	pub(super) fn node_scratch(&self) -> u32 {
		self.int_scratch + numbers::big_int::INT_SCRATCH_LOCALS
	}

	/// A second Node scratch local: a container held while node_scratch serves its items (`a, b = xs` checking an item)
	pub(super) fn container_scratch(&self) -> u32 {
		self.node_scratch() + 1
	}

	/// A local or declared global holding a primitive number, not a Node reference
	pub(super) fn is_numeric_variable(&self, name: &str) -> bool {
		match self.scope.lookup(name) {
			Some(local) => !local.kind.is_ref(),
			None => self.ctx.user_globals.get(name).is_some_and(|(_, kind)| !kind.is_ref()),
		}
	}

	/// `x += y` and `x++` on a declared global (not shadowed by a local) as the plain assignment `x = x + y`,
	/// so globals resolve exactly like in plain assignment
	pub(super) fn global_update_as_assignment(&self, target: &Node, op: &Op, operand: &Node) -> Option<Node> {
		let Node::Symbol(name) = target.drop_meta() else {
			return None;
		};
		if self.scope.lookup(name).is_some() || !self.ctx.user_globals.contains_key(name) {
			return None;
		}
		let (base_op, operand) = match op {
			Op::Inc => (Op::Add, Node::int(1)),
			Op::Dec => (Op::Sub, Node::int(1)),
			_ if op.is_compound_assign() => (op.base_op(), operand.clone()),
			_ => return None,
		};
		let updated = Node::Key(Box::new(target.clone()), base_op, Box::new(operand));
		Some(Node::Key(Box::new(target.clone()), Op::Assign, Box::new(updated)))
	}

	/// `x = v` for a declared global x: store v in the global's own representation and leave it on the stack
	pub(super) fn emit_global_store(&mut self, func: &mut Function, name: &str, value: &Node) -> Option<Kind> {
		let &(index, kind) = self.ctx.user_globals.get(name)?;
		if self.emit_typed_list_store(func, name, value) {
			func.instruction(&I::Drop);
			self.emit_typed_list_as_node(func, name);
			return Some(kind);
		}
		self.emit_value_of_kind(func, value, kind);
		Self::emit_list(func, &[I::GlobalSet(index), I::GlobalGet(index)]);
		if kind.is_ref() {
			func.instruction(&I::RefAsNonNull);
		}
		Some(kind)
	}

	/// `global x=7` declares x with the value 7; `global x` declares x zero-initialized
	pub(super) fn global_declaration_parts(decl: &Node) -> Option<(String, Node)> {
		match decl.drop_meta() {
			Node::Symbol(name) => Some((name.clone(), Node::int(0))),
			Node::Key(left, Op::Define | Op::Assign, right) => match left.drop_meta() {
				Node::Symbol(name) => Some((name.clone(), right.as_ref().clone())),
				_ => None,
			},
			_ => None,
		}
	}

	/// `global x = value`: x is declared ahead of all code (`allocate_declared_globals`), so this stores the value and
	/// gives the name and kind of x, or None for a malformed declaration (reported)
	fn emit_global_store_declared(&mut self, func: &mut Function, decl: &Node) -> Option<(String, Kind)> {
		let Some((name, value)) = Self::global_declaration_parts(decl) else {
			self.emit_malformed(func, decl, "`global name` or `global name = value`");
			return None;
		};
		let kind = self.declare_global(&name, &value);
		self.emit_global_store(func, &name, &value);
		Some((name, kind))
	}

	/// `global x = value` as a node
	pub(super) fn emit_global_declaration(&mut self, func: &mut Function, decl: &Node) {
		let Some((_, kind)) = self.emit_global_store_declared(func, decl) else {
			return;
		};
		if !kind.is_ref() {
			self.emit_primitive_as_node(func, kind);
		}
	}

	/// The global named `name`, declared with the kind of its first value unless it already exists
	pub(super) fn declare_global(&mut self, name: &str, value: &Node) -> Kind {
		match self.ctx.user_globals.get(name) {
			Some(&(_, kind)) => kind,
			None => self.allocate_global(name, crate::analyzer::held_kind(value, || self.get_type(value))),
		}
	}

	pub(super) fn allocate_global(&mut self, name: &str, kind: Kind) -> Kind {
		let global_idx = self.declare_mutable_global(kind);
		self.ctx.user_globals.insert(name.to_string(), (global_idx, kind));
		// a function a task runs reads the global as the program has it (`sample_rate` of lib/sound.warp in `go { beep }`)
		self.export_to_tasks(&format!("{GLOBAL_EXPORT_LABEL}{name}"), global_idx);
		kind
	}

	/// Every `global` of the program exists before any function is compiled, so function bodies can change it
	pub(super) fn allocate_declared_globals(&mut self, program: &Node) {
		let mut main = Scope::with_function_kinds(self.user_function_kinds())
			.with_closure_targets(self.ctx.closure_variable_targets.clone()); // `global g = vec(1, 2)` holds what vec returns
		collect_variables(program, &mut main);
		_ = crate::analyzer::widen_globals_by_functions(
			&mut main.globals,
			self.ctx.user_functions.values(),
			&self.user_function_kinds(),
			&self.ctx.closure_variable_targets,
		);
		let mut names: Vec<&String> =
			main.globals.keys().filter(|name| !self.ctx.user_globals.contains_key(*name)).collect();
		names.sort();
		// a task's instance copies globals as Nodes, so a program with tasks keeps none in an array or map of its own type
		let (typed, typed_maps) = if self.spawns_tasks() {
			Default::default()
		} else {
			(self.find_typed_globals(program, &main), self.find_typed_map_globals(program, &main.globals))
		};
		for name in names {
			if typed_maps.contains(name) {
				let global = self.declare_typed_map_global();
				self.ctx.user_globals.insert(name.to_string(), (global, main.globals[name].kind));
				self.typed_map_globals.insert(name.to_string(), global);
				continue;
			}
			match typed.get(name) {
				Some(&list) => {
					let global = self.declare_typed_list_global(list.element);
					self.ctx.user_globals.insert(name.to_string(), (global, main.globals[name].kind));
					self.typed_globals.insert(name.to_string(), (global, list));
				}
				None => {
					self.allocate_global(name, main.globals[name].kind);
				}
			}
		}
		self.ctx.declared_globals = main.globals;
	}

	/// The primitive on the stack (i64 or f64 as stored, see `storage_type`) as a Node of its kind
	pub(super) fn emit_primitive_as_node(&mut self, func: &mut Function, kind: Kind) {
		if kind.is_float() {
			self.emit_call(func, "new_float");
		} else if kind == Kind::Codepoint {
			func.instruction(&I::I32WrapI64);
			self.emit_call(func, "new_codepoint");
		} else {
			self.emit_call(func, "new_int");
		}
	}

	/// How a value of `kind` is stored in locals, parameters and globals
	pub(super) fn storage_type(&self, kind: Kind) -> ValType {
		if kind.is_ref() {
			Ref(self.node_ref(false))
		} else if kind.is_float() {
			ValType::F64
		} else {
			ValType::I64
		}
	}

	/// Emit `node` in the representation `storage_type(kind)` expects
	pub(super) fn emit_value_of_kind(&mut self, func: &mut Function, node: &Node, kind: Kind) {
		if kind.is_ref() {
			self.emit_node_instructions(func, node);
		} else if kind.is_float() {
			self.emit_float_value(func, node);
		} else {
			self.emit_numeric_value(func, node);
		}
	}

	/// Declare a zero/null-initialized mutable global holding a value of `kind`
	pub(super) fn declare_mutable_global(&mut self, kind: Kind) -> u32 {
		let init_expr = if kind.is_ref() {
			ConstExpr::ref_null(HeapType::Concrete(self.type_manager.node_type))
		} else if kind.is_float() {
			ConstExpr::f64_const(Ieee64::new(0.0f64.to_bits()))
		} else {
			ConstExpr::i64_const(0)
		};
		let val_type = if kind.is_ref() { Ref(self.node_ref(true)) } else { self.storage_type(kind) };
		self.declare_global_of(val_type, init_expr)
	}

	pub(super) fn declare_global_of(&mut self, val_type: ValType, init_expr: ConstExpr) -> u32 {
		self.add_global(val_type, true, init_expr)
	}

	/// A mutable global listed under `name` in the name section
	pub(super) fn declare_named_global(&mut self, val_type: ValType, init_expr: ConstExpr, name: String) -> u32 {
		let index = self.declare_global_of(val_type, init_expr);
		self.extra_global_names.push((index, name));
		index
	}

	pub(super) fn declare_constant_global(&mut self, val_type: ValType, init_expr: ConstExpr) -> u32 {
		self.add_global(val_type, false, init_expr)
	}

	fn add_global(&mut self, val_type: ValType, mutable: bool, init_expr: ConstExpr) -> u32 {
		self.globals.global(GlobalType { val_type, mutable, shared: false }, &init_expr);
		self.next_global_idx += 1;
		self.next_global_idx - 1
	}

	/// `global x = value` as a number (for emit_numeric_value)
	pub(super) fn emit_global_numeric(&mut self, func: &mut Function, decl: &Node) {
		let Some((name, kind)) = self.emit_global_store_declared(func, decl) else {
			return;
		};
		if kind.is_ref() {
			self.emit_call(func, "get_int_value");
		} else if kind.is_float() {
			self.emit_float_in_exact_context(func, &name);
		}
	}
}
