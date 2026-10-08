//! User functions: their compilation, signatures, closure captures and calls

use super::*;

/// The kinds a call's argument has for sure when inference names them: what a declared parameter type is checked against
const SCALAR_KINDS: [Kind; 5] = [Kind::Int, Kind::Float, Kind::Text, Kind::Codepoint, Kind::List];

impl WasmGcEmitter {
	// ═══════════════════════════════════════════════════════════════════════════
	// User-defined function compilation (extraction done in analyzer)
	// ═══════════════════════════════════════════════════════════════════════════

	/// Compile all extracted user functions to WASM
	/// Pre-allocate strings from user function bodies before compiling
	pub(super) fn collect_user_function_strings(&mut self) {
		let bodies: Vec<Box<Node>> = self.ctx.user_functions.values()
			.map(|f| f.body.clone())
			.collect();
		for body in bodies {
			self.collect_and_allocate_strings(&body);
		}
	}

	/// Give every outer variable a function reads a global, set from the variable where the function is defined
	pub(super) fn allocate_closure_captures(&mut self, program: &Node) {
		let mut outer = Scope::with_function_kinds(self.user_function_kinds()).with_closure_targets(self.ctx.closure_variable_targets.clone()); // `x = g()` holds what g returns
		collect_variables(program, &mut outer);
		// a typed list of main captured by a function is passed in a global of its array type (a task's instance copies
		// capture globals as Nodes, so not with tasks)
		let tasks = self.ctx.ffi_imports.contains_key(crate::host::TASK_SPAWN_VALUES) || self.ctx.ffi_imports.contains_key(crate::host::TASK_SPAWN);
		let saved_scope = std::mem::replace(&mut self.scope, outer.clone());
		let main_typed = self.find_typed_lists(program);
		self.scope = saved_scope;
		let functions: Vec<UserFunctionDef> = self.ctx.user_functions.values().cloned().collect();
		for function in functions {
			let enclosing = self.enclosing_scope(&function.name, &outer);
			let captured: Vec<(String, Kind)> = match &enclosing {
				// `outer·inner` reads outer's parameters and variables, main's where outer has none of that name
				Some(enclosing) => {
					let mut captured = captured_variables(&function, enclosing);
					captured.extend(captured_variables(&function, &outer).into_iter().filter(|(name, _)| enclosing.lookup(name).is_none()));
					captured
				}
				None => captured_variables(&function, &outer),
			}
				.into_iter()
				.filter(|(name, _)| !self.ctx.user_functions.contains_key(name))
				.collect();
			let bindings = captured.iter().filter_map(|(name, kind)| {
				let binding = enclosing.as_ref().and_then(|enclosing| enclosing.lookup(name)).or_else(|| outer.lookup(name));
				binding.map(|local| Local { kind: *kind, ..local.clone() })
			}).collect();
			self.ctx.capture_bindings.insert(function.name.clone(), bindings);
			let captures: Vec<(String, (u32, Kind))> = captured.into_iter()
				.map(|(name, kind)| match main_typed.get(&name).filter(|_| enclosing.is_none() && !tasks) {
					Some(&list) => {
						let global = self.declare_typed_list_global(list.element);
						self.typed_capture_globals.insert(global, list);
						(name, (global, kind))
					}
					None => (name, (self.declare_mutable_global(kind), kind)),
				})
				.collect();
			// a task's instance gets the values the spawning instance captured (src/tasks.rs copies these globals)
			if self.ctx.ffi_imports.contains_key(crate::host::TASK_SPAWN_VALUES) || self.ctx.ffi_imports.contains_key(crate::host::TASK_SPAWN) {
				for (name, (global, _)) in &captures {
					self.exports.export(&format!("{CAPTURE_EXPORT_PREFIX}{}·{name}", function.name), ExportKind::Global, *global);
				}
			}
			self.ctx.captures.insert(function.name, captures);
		}
		self.main_typed_lists = Some(main_typed);
	}

	/// A host that hands values into the program (run_block's result, a task's value) builds an exact number beyond the
	/// fixnums from fixnum pieces with these (tasks.rs Builders)
	pub(super) fn export_exact_builders(&mut self) {
		let hands_values_in = [crate::host::RUN_BLOCK, crate::host::BLOCK_VALUE, crate::host::FOREIGN_CALL, crate::host::STD_PURE, crate::host::STD_IO, crate::host::TASK_SPAWN, crate::host::TASK_AWAIT, crate::host::TASK_SPAWN_VALUES, crate::host::TASK_AWAIT_VALUE].iter().any(|word| self.ctx.ffi_imports.contains_key(*word));
		if !hands_values_in || !self.int_runtime() {
			return;
		}
		for name in EXACT_BUILDERS {
			let index = self.func_index(name);
			self.exports.export(name, ExportKind::Func, index);
		}
	}

	/// The globals a function body sees: the declared ones and the variables it captures, with their types
	pub(super) fn function_globals(&self, function: &str) -> HashMap<String, Local> {
		let mut globals = self.ctx.declared_globals.clone();
		for binding in self.ctx.capture_bindings.get(function).into_iter().flatten() {
			globals.entry(binding.name.clone()).or_insert_with(|| binding.clone());
		}
		globals
	}

	/// The parameters and variables of the function whose body defines `function` (`outer` for `outer·inner`), with the
	/// kinds its compiled body gives them (compile_user_function_body); main's variables behind them, which outer reads
	/// too (`under = paper` copies main's kind, card captured-copy)
	pub(super) fn enclosing_scope(&self, function: &str, main: &Scope) -> Option<Scope> {
		let enclosing = self.ctx.enclosing_functions.get(function).and_then(|name| self.ctx.user_functions.get(name))?;
		let mut scope = Scope::with_function_kinds(self.user_function_kinds()).with_closure_targets(self.ctx.closure_variable_targets.clone());
		scope.parent = Some(Box::new(main.clone()));
		scope.globals = self.ctx.declared_globals.clone();
		for (index, param) in enclosing.params.iter().enumerate() {
			let kind = if self.takes_list_abi(&enclosing.name, index) { Kind::List } else { param_kind(param) };
			scope.define_param(param.name.clone(), kind);
		}
		collect_variables(&enclosing.body, &mut scope);
		Some(scope)
	}

	/// Any call in the body of outer may reach `outer·inner` (directly, through a sibling, or passed as a value:
	/// `apply(inner)`), and inner reads outer's variables as they are now (`nonlocal y`, cards g-qUkY, g-rQ-U): the
	/// capture globals of all of outer's nested functions are set anew before each call
	pub(super) fn refresh_enclosing_captures(&mut self, func: &mut Function) {
		let Some(compiling) = self.compiling.clone() else { return };
		for inner in self.nested_functions(&compiling) {
			self.emit_closure_capture(func, &inner);
		}
	}

	/// At a function definition: snapshot the captured variables, so later reassignment is not seen by the function;
	/// the functions nested in it get the variables of this scope too (`k=5; def outer(){ def inner(){ k } }`)
	pub(super) fn emit_closure_capture(&mut self, func: &mut Function, function_name: &str) {
		let captures = self.ctx.captures.get(function_name).cloned().unwrap_or_default();
		for (name, (global, kind)) in captures {
			if self.emit_typed_capture(func, &name, global) || (kind.is_ref() && self.emit_typed_list_as_node(func, &name)) {
				func.instruction(&I::GlobalSet(global));
				continue;
			}
			let stored_alike = |local: &&Local| self.storage_type(local.kind) == self.storage_type(kind);
			if let Some(local) = self.scope.lookup(&name).filter(stored_alike) {
				func.instruction(&I::LocalGet(local.position));
				func.instruction(&I::GlobalSet(global));
			}
		}
		for nested in self.nested_functions(function_name) {
			self.emit_closure_capture(func, &nested);
		}
	}

	/// The functions defined in the body of `function` (`outer·inner` of outer), in a stable order
	pub(super) fn nested_functions(&self, function: &str) -> Vec<String> {
		let mut nested: Vec<String> = self.ctx.enclosing_functions.iter().filter(|(_, enclosing)| *enclosing == function).map(|(inner, _)| inner.clone()).collect();
		nested.sort();
		nested
	}

	pub(super) fn compile_user_functions(&mut self) {
		// Clone the function names to avoid borrow issues
		let func_names: Vec<String> = self.ctx.user_functions.keys().cloned().collect();

		self.list_abi = self.find_list_abi();
		// PASS 1: Register all function signatures and indices
		// This allows forward references (e.g., is_prime can call check before check is compiled)
		for name in &func_names {
			self.register_user_function_signature(name);
		}
		self.register_closure_entries();

		// PASS 2: Compile all function bodies
		for name in func_names {
			if self.is_closure_call(&name) {
				self.compile_closure_call(&name);
			} else if crate::closures::capture_reader(&name).is_some() {
				self.compile_capture_reader(&name);
			} else {
				self.compile_user_function_body(&name);
			}
		}
		self.compile_closure_entries();
	}

	/// Register a user function's signature and assign it an index (PASS 1)
	pub(super) fn register_user_function_signature(&mut self, name: &str) {
		let user_fn = self.ctx.user_functions.get(name).unwrap().clone();
		let returns_node = user_fn.return_kind.is_ref();  // Text, Symbol, List, etc. return Node refs

		// Create function type: (params...) -> i64 or (ref $Node) depending on return type
		let func_type_idx = self.type_manager.types().len();
		let param_types: Vec<ValType> = user_fn.params.iter().enumerate()
			.map(|(index, param)| match self.struct_parameter(name, index) {
				Some(class) => Ref(self.instance_ref(class)),
				None if self.takes_list_abi(name, index) => self.node_list_type(),
				None => self.storage_type(param_kind(param)),
			})
			.collect();
		let result_types = if let Some(class) = self.struct_results.get(name) {
			vec![Ref(self.instance_ref(class))]
		} else if self.returns_list_abi(name) {
			vec![self.node_list_type()]
		} else if !user_fn.tuple_kinds.is_empty() {
			self.tuple_result_types(&user_fn.tuple_kinds)
		} else if returns_node {
			vec![Ref(self.node_ref(false))]
		} else {
			vec![self.storage_type(user_fn.return_kind)]
		};
		self.type_manager.types_mut().ty().function(param_types, result_types);

		// Register function in function section
		self.functions.function(func_type_idx);
		let func_idx = self.next_func_idx;
		self.next_func_idx += 1;
		if !user_fn.tuple_kinds.is_empty() {
			self.register_tuple_packer(name, &user_fn.tuple_kinds);
		}

		// Store the function index and whether it returns a Node
		if let Some(fn_def) = self.ctx.user_functions.get_mut(name) {
			fn_def.func_index = Some(func_idx);
		}
	}

	/// Compile a user function's body (PASS 2)
	pub(super) fn compile_user_function_body(&mut self, name: &str) {
		let user_fn = self.ctx.user_functions.get(name).unwrap().clone();
		let returns_node = user_fn.return_kind.is_ref();  // Text, Symbol, List, etc. return Node refs
		let saved_compiling = self.compiling.replace(name.to_string());

		// Create function scope with parameters
		let function_scope = Scope::with_function_kinds(self.user_function_kinds()).with_closure_targets(self.ctx.closure_variable_targets.clone());
		let saved_scope = std::mem::replace(&mut self.scope, function_scope);
		// declared globals are changed in place, never shadowed by a local of the same name
		self.scope.globals = self.function_globals(name);
		for (index, param) in user_fn.params.iter().enumerate() {
			let kind = if self.takes_list_abi(name, index) { Kind::List } else { param_kind(param) };
			self.scope.define_param(param.name.clone(), kind);
		}

		// a captured typed list of main is read from its capture global (shadowing a typed global of that name)
		let captures = self.ctx.captures.get(name).cloned().unwrap_or_default();
		let shadowed_typed: Vec<_> = captures.iter().map(|(variable, (global, _))| {
			let previous = match self.typed_capture_globals.get(global) {
				Some(&list) => self.typed_globals.insert(variable.clone(), (*global, list)),
				None => self.typed_globals.remove(variable),
			};
			(variable.clone(), previous)
		}).collect();

		// Collect any additional variables in the body, and the temp locals its loops need
		let temp_locals = collect_variables(&user_fn.body, &mut self.scope);
		let mut typed_lists = self.find_typed_lists(&user_fn.body);
		for (index, param) in user_fn.params.iter().enumerate().filter(|(index, _)| self.takes_list_abi(name, *index)) {
			let _ = index;
			typed_lists.insert(param.name.clone(), list_dispatch::TypedList { element: list_dispatch::ElementType::Node, updated: true, aliased: true });
		}
		let saved_typed_lists = std::mem::replace(&mut self.typed_lists, typed_lists);
		let typed_maps = self.find_typed_maps(&user_fn.body);
		let saved_typed_maps = std::mem::replace(&mut self.typed_maps, typed_maps);
		let mut typed_structs = self.find_typed_structs(&user_fn.body);
		typed_structs.extend(user_fn.params.iter().enumerate().filter_map(|(index, param)| Some((param.name.clone(), self.struct_parameter(name, index)?.clone()))));
		let saved_typed_structs = std::mem::replace(&mut self.typed_structs, typed_structs);
		let saved_bounded_counters = std::mem::replace(&mut self.bounded_counters, super::big_int::bounded_counters(&user_fn.body));

		// Declare locals (parameters are already accounted for); temps follow the variables, as in main
		let num_params = user_fn.params.len() as u32;
		let num_locals = self.scope.local_count();

		let saved_scratch = std::mem::replace(&mut self.int_scratch, num_locals + temp_locals);
		let saved_temp_local = std::mem::replace(&mut self.next_temp_local, num_locals);
		let saved_returns_node = std::mem::replace(&mut self.returns_node, returns_node);
		let saved_returns_float = std::mem::replace(&mut self.returns_float, user_fn.return_kind.is_float());
		let returns_list = self.returns_list_abi(name);
		let saved_returns_list = std::mem::replace(&mut self.returns_list, returns_list);
		let saved_loop_labels = self.take_loop_labels();
		let mut locals = self.local_declarations(num_params as usize);
		if temp_locals > 0 {
			locals.push((temp_locals, ValType::I64));
		}
		locals.push((big_int::INT_SCRATCH_LOCALS, ValType::I64));
		locals.push((1, Ref(self.node_ref(true)))); // node_scratch
		let mut func = Function::new(locals);
		self.emit_node_local_defaults(&mut func, &user_fn.body, num_params as usize);

		// Captured variables read their definition-time globals
		let shadowed: Vec<_> = captures.iter()
			.map(|(variable, global)| (variable.clone(), self.ctx.user_globals.insert(variable.clone(), *global)))
			.collect();

		let saved_returned_tuple = std::mem::replace(&mut self.returned_tuple, user_fn.tuple_kinds.clone());
		// Compile the function body - use node instructions for Node-returning functions
		if self.struct_results.contains_key(name) {
			self.emit_struct_result_body(&mut func, &user_fn.body);
		} else if self.returns_list {
			self.emit_list_abi_body(&mut func, &user_fn.body);
		} else if !user_fn.tuple_kinds.is_empty() {
			// every path ends in `return a, b` (tuples::check_definition): the body's own value is never reached
			self.emit_node_instructions(&mut func, &user_fn.body);
			func.instruction(&I::Drop);
			func.instruction(&I::Unreachable);
		} else if returns_node {
			self.emit_node_instructions(&mut func, &user_fn.body);
		} else {
			self.emit_value_of_kind(&mut func, &user_fn.body, user_fn.return_kind);
		}
		func.instruction(&I::End);

		for (variable, global) in shadowed {
			match global {
				Some(global) => self.ctx.user_globals.insert(variable, global),
				None => self.ctx.user_globals.remove(&variable),
			};
		}
		for (variable, previous) in shadowed_typed {
			match previous {
				Some(previous) => self.typed_globals.insert(variable, previous),
				None => self.typed_globals.remove(&variable),
			};
		}

		// Add to code section
		self.code.function(&func);
		if !user_fn.tuple_kinds.is_empty() {
			self.compile_tuple_packer(&user_fn.tuple_kinds);
		}
		self.returned_tuple = saved_returned_tuple;

		// Restore scope
		self.scope = saved_scope;
		self.int_scratch = saved_scratch;
		self.next_temp_local = saved_temp_local;
		self.returns_node = saved_returns_node;
		self.returns_float = saved_returns_float;
		self.returns_list = saved_returns_list;
		self.restore_loop_labels(saved_loop_labels);
		self.typed_lists = saved_typed_lists;
		self.typed_maps = saved_typed_maps;
		self.typed_structs = saved_typed_structs;
		self.bounded_counters = saved_bounded_counters;
		self.compiling = saved_compiling;

		// Export the function (get func_idx from the stored function definition)
		let func_idx = self.ctx.user_functions.get(name).unwrap().func_index.unwrap();
		self.exports.export(name, ExportKind::Func, func_idx);
	}

	/// Emit a call to a user-defined function (returns Node)
	pub(super) fn emit_user_function_call(&mut self, func: &mut Function, fn_name: &str, args: &[Node]) {
		let Some(user_fn) = self.ctx.user_functions.get(fn_name).cloned() else {
			self.emit_type_error(func, self.ctx.undefined_function_message(fn_name));
			return;
		};
		let returns_node = user_fn.return_kind.is_ref();

		// Emit arguments and call
		self.emit_user_function_call_inner(func, &user_fn, args);

		if self.returns_list_abi(fn_name) {
			self.emit_call(func, "node_list_as_node");
		} else if user_fn.return_kind.is_float() {
			self.emit_call(func, "new_float");
		} else if user_fn.return_kind == Kind::Codepoint {
			// a character is returned as its code point
			func.instruction(&Instruction::I32WrapI64);
			self.emit_call(func, "new_codepoint");
		} else if !returns_node {
			self.emit_call(func, "new_int");
		}
	}

	/// Result kinds of the user functions, of `floor(x)`, `round(x)`…, which build an Int (emit_introspection_fn)
	/// unless libm's f64 version is imported, and of the exports of imported WebAssembly modules (`shout("hi")`, a text)
	pub(super) fn user_function_kinds(&self) -> HashMap<String, Kind> {
		let rounding = ROUNDING_FUNCTIONS.iter().filter(|name| !self.ctx.ffi_imports.contains_key(**name)).map(|name| (name.to_string(), Kind::Int));
		let tuple_values = self.ctx.user_functions.iter().flat_map(|(name, function)| {
			function.tuple_kinds.iter().enumerate().map(|(index, kind)| (crate::tuples::element_key(name, index), *kind))
		});
		let fields = crate::analyzer::declared_field_kinds(&self.ctx.type_registry);
		let module_exports = self.ctx.ffi_imports.iter().filter(|(_, import)| crate::wasm_modules::is_module_path(import.library))
			.map(|(name, import)| (name.clone(), crate::analyzer::signature_kind(import)));
		module_exports.chain(rounding).chain(self.ctx.user_functions.iter().map(|(name, function)| (name.clone(), function.return_kind))).chain(tuple_values).chain(fields).collect()
	}

	/// Emit a call to a user-defined function whose result is needed as f64
	pub(super) fn emit_user_function_call_float(&mut self, func: &mut Function, fn_name: &str, args: &[Node]) {
		let user_fn = self.ctx.user_functions[fn_name].clone();
		if user_fn.return_kind.is_float() {
			self.emit_user_function_call_inner(func, &user_fn, args);
		} else {
			self.emit_user_function_call_numeric(func, fn_name, args);
			self.emit_int_to_f64(func, None);
		}
	}

	/// Emit a call to a user-defined function (returns raw i64)
	/// Note: For Node-returning functions, this extracts the integer value from the Node
	pub(super) fn emit_user_function_call_numeric(&mut self, func: &mut Function, fn_name: &str, args: &[Node]) {
		let Some(user_fn) = self.ctx.user_functions.get(fn_name).cloned() else {
			self.emit_type_error(func, self.ctx.undefined_function_message(fn_name));
			return;
		};
		let returns_node = user_fn.return_kind.is_ref();

		// Emit arguments and call
		self.emit_user_function_call_inner(func, &user_fn, args);

		if self.returns_list_abi(fn_name) {
			self.emit_call(func, "node_list_as_node");
		}
		if user_fn.return_kind.is_float() {
			self.emit_float_in_exact_context(func, fn_name);
		} else if returns_node {
			self.emit_call(func, "get_int_value");
		}
	}

	/// Inner helper for emitting user function calls; a tuple function's values are packed into a list
	pub(super) fn emit_user_function_call_inner(&mut self, func: &mut Function, user_fn: &UserFunctionDef, args: &[Node]) {
		self.emit_user_function_values(func, user_fn, args);
		if !user_fn.tuple_kinds.is_empty() && user_fn.func_index.is_some() {
			self.emit_pack_tuple(func, &user_fn.name);
		}
	}

	/// Arguments and the call: the function's own results on the stack, several for a tuple function
	pub(super) fn emit_user_function_values(&mut self, func: &mut Function, user_fn: &UserFunctionDef, args: &[Node]) {
		let Some(func_index) = user_fn.func_index else {
			self.emit_type_error(func, format!("function {} is used before it is compiled", user_fn.name));
			return;
		};
		self.refresh_enclosing_captures(func);
		if self.emit_direct_closure_call(func, user_fn, args) {
			return;
		}

		if args.len() > user_fn.params.len() {
			let count = |n: usize| if n == 1 { "1 argument".to_string() } else { format!("{n} arguments") };
			self.emit_type_error(func, format!("{} takes {}, got {}", user_fn.name, count(user_fn.params.len()), args.len()));
			return;
		}
		// Emit arguments; a missing argument evaluates its default anew at every call
		// a default reads the parameters before it as the values given for them: `f(a, b = a * 2)`
		let mut given: HashMap<String, Node> = HashMap::new();
		for (i, param) in user_fn.params.iter().enumerate() {
			let defaulted = param.default.as_ref().map(|default| crate::law::substitute(default, &given));
			let Some(argument) = args.get(i).or(defaulted.as_ref()).cloned() else {
				self.emit_type_error(func, format!("{} needs a value for parameter {} (it has no default)", user_fn.name, param.name));
				return;
			};
			given.insert(param.name.clone(), argument.clone());
			let argument = &argument;
			// `f(xs: texts)` refuses a list of ints (W0: list int ≰ list text)
			let element = param.annotation.as_ref().and_then(crate::analyzer::declared_element_type);
			let list_type = crate::analyzer::list_type_name(argument, &self.scope);
			if let Some(element) = element.filter(|element| !crate::analyzer::elements_fit(element, &list_type)) {
				let message = format!("{} needs a list of {element} for parameter {}, got {} (a {list_type})", user_fn.name, param.name, argument.serialize());
				self.emit_type_error(func, message);
				return;
			}
			if self.takes_list_abi(&user_fn.name, i) {
				self.emit_list_abi_value(func, argument);
				continue;
			}
			if let Some(class) = self.struct_parameter(&user_fn.name, i).cloned() {
				self.emit_struct_argument(func, argument, &class);
				continue;
			}
			let expected = param_kind(param);
			let given = self.get_type(argument);
			// a declared list (`xs:list`, `xs:ints`) refuses a number or text loudly (wiki/Footguns.md "Type annotations not enforced loudly")
			let declared_list = expected == Kind::List && param.annotation.is_some();
			let refused: &[Kind] = if declared_list { &[Kind::Int, Kind::Float, Kind::Text, Kind::Codepoint] } else { &[Kind::List, Kind::Text] };
			// a declared builtin type takes only its subtypes, `f(x: text)` refuses 3 (W0, notes/type_theory.md)
			let declared_misfit = param.annotation.as_ref().and_then(crate::analyzer::annotated_builtin_type).is_some_and(|type_name| SCALAR_KINDS.contains(&given) && !crate::analyzer::admits(type_name, given));
			// `f(b: bool)` refuses 2 but takes yes, no, 1 and 0 (card bool-assign, P199)
			let annotated_bool = param.annotation.as_ref().map(Node::name).filter(|type_name| crate::analyzer::is_bool_type(type_name));
			if annotated_bool.is_some_and(|type_name| crate::analyzer::literal_misfit(&type_name, argument).is_some()) {
				let message = format!("{} needs a bool for parameter {}, got {} ({})", user_fn.name, param.name, argument.serialize(), kind_with_article(given));
				self.emit_type_error(func, message);
				return;
			}
			// `[]` is ø here: the empty list fits every declared list
			let is_empty_list = matches!(argument.drop_meta(), Node::Empty);
			if declared_misfit || ((!expected.is_ref() || declared_list) && refused.contains(&given) && given != expected && !is_empty_list) {
				let message = format!("{} needs {} for parameter {}, got {} ({})", user_fn.name, kind_with_article(expected), param.name, argument.serialize(), kind_with_article(given));
				self.emit_type_error(func, message);
				return;
			}
			self.emit_declared_value(func, param.annotation.as_ref(), argument, expected);
			if expected == Kind::Text && given == Kind::Codepoint { // f("a") for f(t:text): "a" lexes as a character
				self.emit_call(func, text_builtins::TEXT_OF);
			}
		}

		// Call the function
		func.instruction(&I::Call(func_index));
	}
}
