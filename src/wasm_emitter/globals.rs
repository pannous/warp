//! Raised errors, fetch calls and globals: declarations, updates, storage of a global's kind

use super::*;

impl WasmGcEmitter {
	/// `error("…")` where a number is wanted (the branch of an Int if, analyzer::raises_error): it cannot be an Error value,
	/// so it fails the run with its message through `returned_error`, which `try` catches; true when it was one
	pub(super) fn emit_raised_error(&mut self, func: &mut Function, node: &Node) -> bool {
		let Some(message) = returned_error_message(node) else { return false };
		self.emit_trap_detail(func, message);
		self.emit_runtime_error(func, list_ops::RETURNED_ERROR);
		true
	}

	/// The value of `return x` as the function gives it back: a Node, an f64 or an i64; `return error("…")` from a
	/// function of numbers fails the run with the message, it cannot give back an Error value
	pub(super) fn emit_returned_value(&mut self, func: &mut Function, value: &Node) {
		if self.returns_list {
			self.emit_list_abi_value(func, value);
		} else if let Some(message) = returned_error_message(value).filter(|_| !self.returns_node) {
			self.emit_trap_detail(func, message);
			self.emit_runtime_error(func, list_ops::RETURNED_ERROR);
		} else if self.returns_node {
			self.emit_node_instructions(func, value);
		} else if self.returns_float {
			self.emit_float_value(func, value);
		} else {
			self.emit_numeric_value(func, value);
		}
	}

	/// The one nullable Node local of every function, after its int scratch locals
	pub(super) fn node_scratch(&self) -> u32 {
		self.int_scratch + big_int::INT_SCRATCH_LOCALS
	}

	/// A second Node scratch local: a container held while node_scratch serves its items (`a, b = xs` checking an item)
	pub(super) fn container_scratch(&self) -> u32 {
		self.node_scratch() + 1
	}

	/// Emit a fetch call using the host.fetch import (host.fetch_within for an explicit timeout)
	/// Returns a Text node with the body, or an Error node with the reason: the host marks a failure by a negative length
	pub(super) fn emit_fetch_call(&mut self, func: &mut Function, url_node: &Node, timeout: Option<std::time::Duration>) {
		let url = self.extract_url_string(url_node);
		// a URL known only at run time (a text variable, a parameter, a concatenation), else the URL as written
		if self.is_runtime_text(url_node) {
			self.emit_string_ptr_len(func, url_node);
		} else {
			let (url_ptr, url_len) = self.allocate_string(&url);
			Self::emit_list(func, &[I32Const(url_ptr as i32), I32Const(url_len as i32)]);
		}
		let import = match timeout {
			Some(timeout) => {
				func.instruction(&I::I64Const(timeout.as_millis().min(i64::MAX as u128) as i64));
				"host_fetch_within"
			}
			None => "host_fetch",
		};
		if let Some(f) = self.ctx.func_registry.get(import) {
			func.instruction(&I::Call(f.call_index as u32));
		} else {
			if timeout.is_some() {
				func.instruction(&I::Drop);
			}
			let reason = format!("fetch {url} failed: host imports are not available");
			let (ptr, len) = self.allocate_string(&reason);
			Self::emit_list(func, &[I32Const(ptr as i32), I32Const(-(len as i32))]);
		}
		let (len, ptr) = (self.scratch(0), self.scratch(1));
		Self::emit_list(func, &[I::I64ExtendI32S, I::LocalSet(len), I::I64ExtendI32U, I::LocalSet(ptr)]);
		self.emit_host_text_result(func, len, ptr);
	}


	/// Extract URL string from parsed node tree
	/// Handles patterns like: https://... which parses as Key(Symbol("https"), Colon, ...)
	pub(super) fn extract_url_string(&self, node: &Node) -> String {
		fn node_to_string(n: &Node) -> String {
			match n.drop_meta() {
				Node::Symbol(s) => s.clone(),
				Node::Text(s) => s.clone(),
				Node::Key(left, op, right) => {
					let left_str = node_to_string(left);
					let op_str = match op {
						Op::Colon => ":",
						Op::Div => "/",
						Op::Dot => ".",
						_ => "",
					};
					let right_str = node_to_string(right);
					format!("{}{}{}", left_str, op_str, right_str)
				}
				Node::List(items, _, _) => items.iter().map(node_to_string).collect::<Vec<_>>().join(""),
				Node::Error(inner) => {
					// Handle parse errors - check if it's an "Unexpected character '/'" error
					if let Node::Text(msg) = inner.drop_meta() {
						if msg.contains("Unexpected character '/'") {
							return "/".to_string();
						}
					}
					// Otherwise recurse into the inner node
					node_to_string(inner)
				}
				_ => format!("{:?}", n),
			}
		}
		node_to_string(node)
	}

	/// Emit global variable declaration: global x = value
	/// Creates a mutable WASM global and initializes it, or reassigns existing global
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
		let Node::Symbol(name) = target.drop_meta() else { return None };
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
		kind
	}

	/// Every `global` of the program exists before any function is compiled, so function bodies can change it
	pub(super) fn allocate_declared_globals(&mut self, program: &Node) {
		let mut main = Scope::with_function_kinds(self.user_function_kinds()).with_closure_targets(self.ctx.closure_variable_targets.clone()); // `global g = vec(1, 2)` holds what vec returns
		collect_variables(program, &mut main);
		_ = crate::analyzer::widen_globals_by_functions(&mut main.globals, self.ctx.user_functions.values(), &self.user_function_kinds(), &self.ctx.closure_variable_targets);
		let mut names: Vec<&String> = main.globals.keys().filter(|name| !self.ctx.user_globals.contains_key(*name)).collect();
		names.sort();
		let typed = self.find_typed_globals(program, &main);
		let typed_maps = self.find_typed_map_globals(program, &main.globals);
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
				None => { self.allocate_global(name, main.globals[name].kind); }
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
		self.globals.global(GlobalType { val_type, mutable: true, shared: false }, &init_expr);
		self.next_global_idx += 1;
		self.next_global_idx - 1
	}

	/// Leave `value` in the exported global `trap_detail` (declared on first use), for the runtime error that follows
	/// to name it: the runner reads it after the trap (wasm_reader::with_trap_detail)
	pub(super) fn emit_trap_detail(&mut self, func: &mut Function, value: &Node) {
		self.trap_detail_global();
		self.emit_global_store(func, TRAP_DETAIL_GLOBAL, value);
		func.instruction(&I::Drop);
	}

	/// The index of the exported global `trap_detail`, declared on first use
	pub(super) fn trap_detail_global(&mut self) -> u32 {
		if let Some((index, _)) = self.ctx.user_globals.get(TRAP_DETAIL_GLOBAL) {
			return *index;
		}
		let index = self.declare_mutable_global(Kind::Empty);
		self.exports.export(TRAP_DETAIL, ExportKind::Global, index);
		self.ctx.user_globals.insert(TRAP_DETAIL_GLOBAL.to_string(), (index, Kind::Empty));
		index
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
