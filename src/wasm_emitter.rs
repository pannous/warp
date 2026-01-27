use crate::analyzer::{analyze_required_functions, collect_all_types, collect_variables, extract_ffi_imports, extract_user_functions, infer_type, Scope};
use crate::context::{Context, UserFunctionDef};
use crate::extensions::numbers::Number;
use crate::function::{Function as FuncDef, Signature};
use crate::gc_traits::GcObject as ErgonomicGcObject;
use crate::node::{Bracket, Node};
use crate::normalize::hints as norm;
use crate::operators::{is_function_keyword, op_to_code, Op};
use crate::type_kinds::{any_heap_type, field_def_to_val_type, FieldDef, Kind, RawFieldValue, TypeDef, TypeRegistry};
use crate::util::gc_engine;
use crate::wasm_reader::read_bytes;
use crate::wasp_parser::WaspParser;
use log::{trace, warn};
use std::collections::HashMap;
use std::thread::scope;
use wasm_ast::instruction;
use wasm_encoder::*;
use wasmparser::{Validator, WasmFeatures};
use Instruction::I32Const;
use StorageType::Val;
use ValType::Ref;



/// Compact 3-field Node struct for WASM GC:
/// ```wat
/// (type $String (struct (field $ptr i32) (field $len i32)))
/// (type $Node (struct
///   (field $kind i64)              ;; Type tag (0=Empty,1=Int,2=Float,3=Text,...)
///   (field $data (ref null any))   ;; Payload: i31ref, (ref $String), (ref $Node), boxed numbers
///   (field $value (ref null $Node)) ;; Child/value node (for Key, Pair, List, etc.)
/// ))
/// ```
pub struct WasmGcEmitter {
	module: Module,
	types: TypeSection,
	imports: ImportSection,
	functions: FunctionSection,
	code: CodeSection,
	exports: ExportSection,
	names: NameSection,
	memory: MemorySection,
	data: DataSection,
	globals: GlobalSection,
	// Type indices
	pub(crate) string_type: u32,
	i64_box_type: u32,
	f64_box_type: u32,
	pub(crate) node_type: u32,
	next_type_idx: u32,
	next_func_idx: u32,
	next_global_idx: u32,

	emit_all_functions: bool,
	emit_kind_globals: bool,
	emit_host_imports: bool,
	emit_wasi_imports: bool,
	emit_ffi_imports: bool,

	next_data_offset: u32,
	next_temp_local: u32,

	// Compilation context
	pub(crate) ctx: Context, // module scope: globals, functions , types, etc.

	scope: Scope, // current function scope
}

impl Default for WasmGcEmitter {
	fn default() -> Self {
		Self::new()
	}
}

impl WasmGcEmitter {
	pub fn new() -> Self {
		WasmGcEmitter {
			module: Module::new(),
			types: TypeSection::new(),
			imports: ImportSection::new(),
			functions: FunctionSection::new(),
			code: CodeSection::new(),
			exports: ExportSection::new(),
			names: NameSection::new(),
			memory: MemorySection::new(),
			data: DataSection::new(),
			globals: GlobalSection::new(),
			string_type: 0,
			i64_box_type: 0,
			f64_box_type: 0,
			node_type: 0,
			next_type_idx: 0,
			next_func_idx: 0,
			next_global_idx: 0,
			emit_all_functions: true,
			emit_kind_globals: true,
			emit_host_imports: false,
			emit_wasi_imports: false,
			emit_ffi_imports: false,
			next_data_offset: 0,
			next_temp_local: 0,
			ctx: Context::new(),

			scope: Default::default(),
		}
	}

	/// Enable/disable emitting Kind globals for documentation
	pub fn set_emit_kind_globals(&mut self, enabled: bool) {
		self.emit_kind_globals = enabled;
	}

	pub fn set_tree_shaking(&mut self, enabled: bool) {
		self.emit_all_functions = !enabled;
	}

	/// Enable/disable host function imports (fetch, run)
	pub fn set_host_imports(&mut self, enabled: bool) {
		self.emit_host_imports = enabled;
	}

	/// Emit host function imports: fetch(url) -> string, run(wasm) -> i64
	/// Must be called before emit_gc_types() since type indices need to be correct
	fn emit_host_imports(&mut self) {
		if !self.emit_host_imports {
			return;
		}

		// Type for fetch: (i32, i32) -> (i32, i32)
		// Takes (url_ptr, url_len), returns (result_ptr, result_len)
		let fetch_type_idx = self.next_type_idx;
		self.types
			.ty()
			.function(vec![ValType::I32, ValType::I32], vec![ValType::I32, ValType::I32]);
		self.next_type_idx += 1;

		// Type for run: (i32, i32) -> i64
		// Takes (wasm_ptr, wasm_len), returns result value
		let run_type_idx = self.next_type_idx;
		self.types
			.ty()
			.function(vec![ValType::I32, ValType::I32], vec![ValType::I64]);
		self.next_type_idx += 1;

		// Import fetch from "host" module
		self.imports
			.import("host", "fetch", EntityType::Function(fetch_type_idx));
		self.register_import("host_fetch");

		// Import run from "host" module
		self.imports.import("host", "run", EntityType::Function(run_type_idx));
		self.register_import("host_run");
	}

	/// Enable/disable WASI imports (fd_write for puts, puti, etc.)
	pub fn set_wasi_imports(&mut self, enabled: bool) {
		self.emit_wasi_imports = enabled;
	}

	/// Emit WASI imports (fd_write from wasi_snapshot_preview1)
	/// Must be called before emit_gc_types() since type indices need to be correct
	fn emit_wasi_imports(&mut self) {
		if !self.emit_wasi_imports {
			return;
		}

		// Type for fd_write: (fd: i32, iovs: i32, iovs_len: i32, nwritten: i32) -> i32
		let fd_write_type_idx = self.next_type_idx;
		self.types.ty().function(
			vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
			vec![ValType::I32],
		);
		self.next_type_idx += 1;

		// Import fd_write from wasi_snapshot_preview1
		self.imports.import(
			"wasi_snapshot_preview1",
			"fd_write",
			EntityType::Function(fd_write_type_idx),
		);
		self.register_import("wasi_fd_write");
	}

	/// Enable/disable FFI imports (libc, libm functions)
	pub fn set_ffi_imports(&mut self, enabled: bool) {
		self.emit_ffi_imports = enabled;
	}

	/// Emit FFI imports for all registered FFI functions
	/// Must be called before emit_gc_types() since type indices need to be correct
	fn emit_ffi_imports(&mut self) {
		if !self.emit_ffi_imports || self.ctx.ffi_imports.is_empty() {
			return;
		}

		// Clone data to avoid borrow conflict - collect (name, library, params, results) tuples
		let mut imports: Vec<(String, String, Vec<ValType>, Vec<ValType>)> =
			self.ctx.ffi_imports.iter()
				.map(|(name, sig)| (name.clone(), sig.library.to_string(), sig.params.clone(), sig.results.clone()))
				.collect();
		imports.sort_by(|(a, _, _, _), (b, _, _, _)| a.cmp(b));

		for (name, library, params, results) in imports {
			let type_idx = self.next_type_idx;
			self.types.ty().function(params, results);
			self.next_type_idx += 1;

			// Import from library module (e.g., "m" for libm, "c" for libc)
			self.imports.import(&library, &name, EntityType::Function(type_idx));

			// Register as an import function - use a leaked string for static lifetime
			let static_name: &'static str = Box::leak(format!("ffi_{}", name).into_boxed_str());
			self.register_import(static_name);
		}
	}

	/// Get FFI function call index by name
	fn ffi_func_index(&self, name: &str) -> Option<u32> {
		let ffi_name = format!("ffi_{}", name);
		self.ctx.func_registry.get(&ffi_name).map(|f| f.call_index as u32)
	}

	/// Register an import function
	fn register_import(&mut self, name: &'static str) -> u32 {
		let func = FuncDef::host(name);
		let idx = self.ctx.func_registry.register(func);
		self.next_func_idx = self.ctx.func_registry.import_count() + self.ctx.func_registry.code_count();
		idx
	}

	/// Register a code function
	fn register_func(&mut self, name: &'static str) -> u32 {
		let func = FuncDef::builtin(name);
		let idx = self.ctx.func_registry.register(func);
		self.next_func_idx = self.ctx.func_registry.import_count() + self.ctx.func_registry.code_count();
		idx
	}

	/// Get function call index by name
	fn func_index(&self, name: &str) -> u32 {
		// First check user functions
		if let Some(user_fn) = self.ctx.user_functions.get(name) {
			if let Some(idx) = user_fn.func_index {
				return idx;
			}
		}
		// Then check registry (builtins/imports)
		self.ctx.func_registry
			.get(name)
			.map(|f| f.call_index as u32)
			.unwrap_or_else(|| panic!("Unknown function: {}", name))
	}

	// ═══════════════════════════════════════════════════════════════════════════
	// User-defined function compilation (extraction done in analyzer)
	// ═══════════════════════════════════════════════════════════════════════════

	/// Compile all extracted user functions to WASM
	/// Pre-allocate strings from user function bodies before compiling
	fn collect_user_function_strings(&mut self) {
		let bodies: Vec<Box<Node>> = self.ctx.user_functions.values()
			.map(|f| f.body.clone())
			.collect();
		for body in bodies {
			self.collect_and_allocate_strings(&body);
		}
	}

	fn compile_user_functions(&mut self) {
		// Clone the function names to avoid borrow issues
		let func_names: Vec<String> = self.ctx.user_functions.keys().cloned().collect();

		for name in func_names {
			self.compile_user_function(&name);
		}
	}

	/// Compile a single user function to WASM
	fn compile_user_function(&mut self, name: &str) {
		let user_fn = self.ctx.user_functions.get(name).unwrap().clone();
		let returns_node = user_fn.return_kind.is_ref();  // Text, Symbol, List, etc. return Node refs

		// Create function type: (params...) -> i64 or (ref $Node) depending on return type
		let func_type_idx = self.types.len();
		let param_types: Vec<ValType> = user_fn.params.iter().map(|_| ValType::I64).collect();
		if returns_node {
			let node_ref = self.node_ref(false);
			self.types.ty().function(param_types, vec![Ref(node_ref)]);
		} else {
			self.types.ty().function(param_types, vec![ValType::I64]);
		}

		// Register function in function section
		self.functions.function(func_type_idx);
		let func_idx = self.next_func_idx;
		self.next_func_idx += 1;

		// Store the function index and whether it returns a Node
		if let Some(fn_def) = self.ctx.user_functions.get_mut(name) {
			fn_def.func_index = Some(func_idx);
		}

		// Create function scope with parameters
		let saved_scope = std::mem::replace(&mut self.scope, Scope::new());
		for (param_name, _default) in user_fn.params.iter() {
			self.scope.define(param_name.clone(), None, Kind::Int);
		}

		// Collect any additional variables in the body
		collect_variables(&user_fn.body, &mut self.scope);

		// Declare locals (parameters are already accounted for)
		let num_params = user_fn.params.len() as u32;
		let num_locals = self.scope.local_count();
		let extra_locals = num_locals.saturating_sub(num_params);

		let mut func = Function::new(vec![(extra_locals, ValType::I64)]);

		// Compile the function body - use node instructions for Node-returning functions
		if returns_node {
			self.emit_node_instructions(&mut func, &user_fn.body);
		} else {
			self.emit_numeric_value(&mut func, &user_fn.body);
		}
		func.instruction(&Instruction::End);

		// Add to code section
		self.code.function(&func);

		// Restore scope
		self.scope = saved_scope;

		// Export the function
		self.exports.export(name, ExportKind::Func, func_idx);
	}

	/// Emit a call to a user-defined function (returns Node)
	fn emit_user_function_call(&mut self, func: &mut Function, fn_name: &str, args: &[Node]) {
		let user_fn = match self.ctx.user_functions.get(fn_name) {
			Some(f) => f.clone(),
			None => panic!("Unknown user function: {}", fn_name),
		};
		let returns_node = user_fn.return_kind.is_ref();

		// Emit arguments and call
		self.emit_user_function_call_inner(func, &user_fn, args);

		// If function returns i64, wrap in new_int for Node context
		if !returns_node {
			self.emit_call(func, "new_int");
		}
	}

	/// Emit a call to a user-defined function (returns raw i64)
	/// Note: For Node-returning functions, this extracts the integer value from the Node
	fn emit_user_function_call_numeric(&mut self, func: &mut Function, fn_name: &str, args: &[Node]) {
		let user_fn = match self.ctx.user_functions.get(fn_name) {
			Some(f) => f.clone(),
			None => panic!("Unknown user function: {}", fn_name),
		};
		let returns_node = user_fn.return_kind.is_ref();

		// Emit arguments and call
		self.emit_user_function_call_inner(func, &user_fn, args);

		// If function returns Node but we need i64, extract the value
		if returns_node {
			// Call get_int_value to extract integer from Node
			self.emit_call(func, "get_int_value");
		}
	}

	/// Inner helper for emitting user function calls
	fn emit_user_function_call_inner(&mut self, func: &mut Function, user_fn: &UserFunctionDef, args: &[Node]) {
		let func_index = match user_fn.func_index {
			Some(idx) => idx,
			None => panic!("User function not yet compiled: {}", user_fn.name),
		};

		// Emit arguments, using defaults for missing ones
		for (i, (_param_name, default_value)) in user_fn.params.iter().enumerate() {
			if i < args.len() {
				// Use provided argument
				self.emit_numeric_value(func, &args[i]);
			} else if let Some(default) = default_value {
				// Use default value
				self.emit_numeric_value(func, default);
			} else {
				panic!("Missing argument {} for function {} (no default)", i, user_fn.name);
			}
		}

		// Call the function
		func.instruction(&Instruction::Call(func_index));
	}

	// ═══════════════════════════════════════════════════════════════════════════
	// Helper methods for clean, DRY code
	// ═══════════════════════════════════════════════════════════════════════════

	/// Create a RefType for node references
	fn node_ref(&self, nullable: bool) -> RefType {
		RefType {
			nullable,
			heap_type: HeapType::Concrete(self.node_type),
		}
	}

	/// Emit string lookup from table and call constructor
	fn emit_string_call(&mut self, func: &mut Function, s: &str, constructor: &'static str) {
		let (ptr, len) = self.ctx
			.string_table
			.get(s)
			.map(|&offset| (offset, s.len() as u32))
			.unwrap_or((0, s.len() as u32));
		func.instruction(&I32Const(ptr as i32));
		func.instruction(&I32Const(len as i32));
		self.emit_call(func, constructor);
	}

	/// Append String struct field names (ptr, len) to an IndirectNameMap
	fn append_string_field_names(type_field_names: &mut IndirectNameMap, type_idx: u32) {
		let mut names = NameMap::new();
		names.append(0, "ptr");
		names.append(1, "len");
		type_field_names.append(type_idx, &names);
	}

	/// Infer the Kind for an expression
	/// Extends analyzer::infer_type with user_globals knowledge
	fn get_type(&self, node: &Node) -> Kind {
		let node = node.drop_meta();
		match node {
			// Check user_globals for symbols
			Node::Symbol(name) => {
				if let Some(&(_, kind)) = self.ctx.user_globals.get(name) {
					return kind;
				}
				// Fall back to scope lookup
				if let Some(local) = self.scope.lookup(name) {
					return local.kind;
				}
				Kind::Symbol
			}
			// Arithmetic: recursively check operands with our get_type
			Node::Key(left, op, right) if op.is_arithmetic() => {
				let left_kind = self.get_type(left);
				let right_kind = self.get_type(right);
				if left_kind == Kind::Float || right_kind == Kind::Float {
					Kind::Float
				} else {
					Kind::Int
				}
			}
			// For other nodes, use analyzer's infer_type
			_ => infer_type(node, &self.scope),
		}
	}

	/// Check if an expression is numeric (int, float, or bool)
	fn is_numeric(&self, node: &Node) -> bool {
		let node = node.drop_meta();
		match node {
			Node::Number(_) | Node::True | Node::False => true,
			Node::Key(_left, op, _right) if op.is_arithmetic() || op.is_comparison() => true,
			Node::Key(left, op, right) if op.is_logical() => self.is_numeric(left) && self.is_numeric(right),
			Node::Key(_, Op::Define | Op::Assign, right) => self.is_numeric(right),
			Node::Symbol(name) => {
				// Check if symbol is a known numeric variable
				if let Some(local) = self.scope.lookup(name) {
					matches!(local.kind, Kind::Int | Kind::Float)
				} else if let Some(&(_, kind)) = self.ctx.user_globals.get(name) {
					matches!(kind, Kind::Int | Kind::Float)
				} else {
					false
				}
			}
			_ => false,  // Empty, Text, Char, List, etc. are not numeric
		}
	}

	/// Emit comparison operator for i64 (result is i32, extended to i64)
	fn emit_comparison(&self, func: &mut Function, op: &Op) {
		let cmp = match op {
			Op::Eq => Instruction::I64Eq,
			Op::Ne => Instruction::I64Ne,
			Op::Lt => Instruction::I64LtS,
			Op::Gt => Instruction::I64GtS,
			Op::Le => Instruction::I64LeS,
			Op::Ge => Instruction::I64GeS,
			_ => unreachable!("Not a comparison op: {:?}", op),
		};
		func.instruction(&cmp);
		func.instruction(&Instruction::I64ExtendI32U);
	}

	/// Emit comparison operator for f64 (result is i32, extended to i64)
	fn emit_float_comparison(&self, func: &mut Function, op: &Op) {
		let cmp = match op {
			Op::Eq => Instruction::F64Eq,
			Op::Ne => Instruction::F64Ne,
			Op::Lt => Instruction::F64Lt,
			Op::Gt => Instruction::F64Gt,
			Op::Le => Instruction::F64Le,
			Op::Ge => Instruction::F64Ge,
			_ => unreachable!("Not a comparison op: {:?}", op),
		};
		func.instruction(&cmp);
		func.instruction(&Instruction::I64ExtendI32U);
	}

	fn should_emit_function(&self, name: &str) -> bool {
		self.emit_all_functions || self.ctx.required_functions.contains(name)
	}

	/// Generate all type definitions and functions
	pub fn emit(&mut self) {
		self.memory.memory(MemoryType {
			minimum: 1,
			maximum: None,
			memory64: false,
			shared: false,
			page_size_log2: None,
		});
		self.exports.export("memory", ExportKind::Memory, 0);
		// Host imports must come before GC types (imports section comes before types in WASM)
		self.emit_host_imports();
		self.emit_wasi_imports();
		self.emit_ffi_imports();
		self.emit_gc_types();
		// Emit user-defined struct types from type_registry (must come after gc_types, before functions)
		self.emit_registered_user_types();
		if self.emit_kind_globals {
			self.emit_kind_globals();
		}
		self.emit_constructors();
		// Emit constructors for registered user types
		self.emit_registered_user_type_constructors();
	}

	/// Emit user types from internal type_registry
	fn emit_registered_user_types(&mut self) {
		let types: Vec<TypeDef> = self.ctx.type_registry.types().to_vec();
		for type_def in &types {
			self.emit_single_user_type(type_def);
		}
	}

	/// Emit a single user-defined struct type
	fn emit_single_user_type(&mut self, type_def: &TypeDef) {
		let fields: Vec<FieldType> = type_def
			.fields
			.iter()
			.map(|f| self.field_def_to_wasm_field(f))
			.collect();

		self.types.ty().struct_(fields);
		self.ctx.user_type_indices.insert(type_def.name.clone(), self.next_type_idx);
		self.next_type_idx += 1;
	}

	/// Emit constructors for registered user types
	fn emit_registered_user_type_constructors(&mut self) {
		let types: Vec<TypeDef> = self.ctx.type_registry.types().to_vec();
		for type_def in &types {
			self.emit_user_type_constructor(type_def);
		}
	}

	/// Emit Kind constants as immutable globals
	/// JIT compilers constant-fold these, so global.get is equally fast
	fn emit_kind_globals(&mut self) {
		let tags = [
			("kind_empty", Kind::Empty),
			("kind_int", Kind::Int),
			("kind_float", Kind::Float),
			("kind_text", Kind::Text),
			("kind_codepoint", Kind::Codepoint),
			("kind_symbol", Kind::Symbol),
			("kind_key", Kind::Key),
			("kind_block", Kind::Block),
			("kind_list", Kind::List),
			("kind_data", Kind::Data),
			("kind_meta", Kind::Meta),
			("kind_error", Kind::Error),
			("kind_type", Kind::TypeDef),
		];

		for (name, tag) in tags {
			self.globals.global(
				GlobalType {
					val_type: ValType::I64,
					mutable: false,
					shared: false,
				},
				&ConstExpr::i64_const(tag as i64),
			);
			self.exports.export(name, ExportKind::Global, self.next_global_idx);
			self.ctx.kind_global_indices.insert(tag, self.next_global_idx);
			self.next_global_idx += 1;
		}
	}

	/// Emit instruction to get a Kind kind value
	fn emit_kind(&self, func: &mut Function, tag: Kind) {
		if let Some(&idx) = self.ctx.kind_global_indices.get(&tag) {
			func.instruction(&Instruction::GlobalGet(idx));
		} else {
			func.instruction(&Instruction::I64Const(tag as i64));
		}
	}

	pub fn emit_for_node(&mut self, node: &Node) {
		self.emit_all_functions = false;
		// First pass: register all types (forward reference support)
		collect_all_types(&mut self.ctx.type_registry, node);
		// Analyze: Extract FFI imports, user functions, and required functions
		extract_ffi_imports(&mut self.ctx, node);
		extract_user_functions(&mut self.ctx, node);
		analyze_required_functions(&mut self.ctx, node);
		// Set emit flag based on whether any FFI imports were found
		self.emit_ffi_imports = !self.ctx.ffi_imports.is_empty();
		let len = self.ctx.required_functions.len();
		trace!(
			"tree-shaking: {} functions required: {:?}",
			len,
			self.ctx.required_functions
		);
		self.emit();
		// Pre-allocate strings from user function bodies before compiling
		self.collect_user_function_strings();
		// Compile user functions after builtin infrastructure is set up
		self.compile_user_functions();
		self.emit_node_main(node);
	}

	/// Emit the compact 3-field GC types
	fn emit_gc_types(&mut self) {
		// Type 0: $String = (struct (field $ptr i32) (field $len i32))
		self.types.ty().struct_(vec![
			FieldType {
				element_type: Val(ValType::I32),
				mutable: false,
			}, // ptr
			FieldType {
				element_type: Val(ValType::I32),
				mutable: false,
			}, // len
		]);
		self.string_type = self.next_type_idx;
		self.next_type_idx += 1;

		// 1: (struct $Node (field $kind i64) (field $data anyref) (field $value (ref null $Node)))
		let node_type_idx = self.next_type_idx;
		self.next_type_idx += 1;

		let node_ref = RefType {
			nullable: true,
			heap_type: HeapType::Concrete(node_type_idx),
		};
		let any_ref = RefType {
			nullable: true,
			heap_type: any_heap_type(),
		};

		self.types.ty().struct_(vec![
			FieldType {
				element_type: Val(ValType::I64),
				mutable: false,
			}, // kind
			FieldType {
				element_type: Val(Ref(any_ref)),
				mutable: true, // mutable for index assignment
			}, // data
			FieldType {
				element_type: Val(Ref(node_ref)),
				mutable: false,
			}, // value
		]);
		self.node_type = node_type_idx;

		// Type 2: $i64box = (struct (field i64)) for boxed integers
		self.types.ty().struct_(vec![FieldType {
			element_type: Val(ValType::I64),
			mutable: false,
		}]);
		self.i64_box_type = self.next_type_idx;
		self.next_type_idx += 1;

		// Type 3: $f64box = (struct (field f64)) for boxed floats
		self.types.ty().struct_(vec![FieldType {
			element_type: Val(ValType::F64),
			mutable: false,
		}]);
		self.f64_box_type = self.next_type_idx;
		self.next_type_idx += 1;
	}

	/// Emit user-defined struct types from TypeRegistry
	pub fn emit_user_types(&mut self, registry: &TypeRegistry) {
		for type_def in registry.types() {
			let fields: Vec<FieldType> = type_def
				.fields
				.iter()
				.map(|f| self.field_def_to_wasm_field(f))
				.collect();

			self.types.ty().struct_(fields);
			self.ctx.user_type_indices.insert(type_def.name.clone(), self.next_type_idx);
			self.next_type_idx += 1;
		}
	}

	/// Convert a FieldDef to a WASM FieldType
	fn field_def_to_wasm_field(&self, field: &FieldDef) -> FieldType {
		let element_type = match field.type_name.as_str() {
			// Node-mode: map wasp types to WASM types
			"Int" | "i64" | "long" => Val(ValType::I64),
			"Float" | "f64" | "double" => Val(ValType::F64),
			"i32" | "int" => Val(ValType::I32),
			"f32" | "float" => Val(ValType::F32),
			"Text" | "String" | "string" => Val(Ref(RefType {
				nullable: true,
				heap_type: HeapType::Concrete(self.string_type),
			})),
			"Node" => Val(Ref(RefType {
				nullable: true,
				heap_type: HeapType::Concrete(self.node_type),
			})),
			// User-defined types
			other => {
				if let Some(&type_idx) = self.ctx.user_type_indices.get(other) {
					Val(Ref(RefType {
						nullable: true,
						heap_type: HeapType::Concrete(type_idx),
					}))
				} else {
					// Fallback to anyref for unknown types
					Val(Ref(RefType {
						nullable: true,
						heap_type: any_heap_type(),
					}))
				}
			}
		};
		FieldType {
			element_type,
			mutable: false,
		}
	}

	/// Get the WASM type index for a user-defined type
	pub fn get_user_type_idx(&self, name: &str) -> Option<u32> {
		self.ctx.user_type_indices.get(name).copied()
	}

	/// Emit with user-defined types from a TypeRegistry
	/// Order: memory, gc_types, user_types, kind_globals, constructors, user_constructors
	pub fn emit_with_types(&mut self, registry: &TypeRegistry) {
		// Memory
		self.memory.memory(MemoryType {
			minimum: 1,
			maximum: None,
			memory64: false,
			shared: false,
			page_size_log2: None,
		});
		self.exports.export("memory", ExportKind::Memory, 0);

		// Core GC types (String, Node, i64box, f64box)
		self.emit_gc_types();

		// User-defined struct types (before any functions!)
		self.emit_user_types(registry);

		// Kind globals
		if self.emit_kind_globals {
			self.emit_kind_globals();
		}

		// Core Node constructors
		self.emit_constructors();

		// User type constructors
		self.emit_user_type_constructors(registry);
	}

	/// Emit constructor functions for user-defined types
	fn emit_user_type_constructors(&mut self, registry: &TypeRegistry) {
		for type_def in registry.types() {
			self.emit_user_type_constructor(type_def);
		}
	}

	/// Emit a constructor function for a single user type: new_TypeName(fields...) -> ref $TypeName
	fn emit_user_type_constructor(&mut self, type_def: &TypeDef) {
		let type_idx = match self.ctx.user_type_indices.get(&type_def.name) {
			Some(&idx) => idx,
			None => return,
		};

		let type_ref = RefType {
			nullable: false,
			heap_type: HeapType::Concrete(type_idx),
		};

		// Build parameter types
		let params: Vec<ValType> = type_def.fields.iter().map(|f| field_def_to_val_type(f, self)).collect();

		// Function type: (params...) -> (ref $TypeName)
		let func_type = self.types.len();
		self.types.ty().function(params.clone(), vec![Ref(type_ref)]);
		self.functions.function(func_type);

		// Function body: get all params, struct.new
		let mut func = Function::new(vec![]);
		for i in 0..type_def.fields.len() {
			func.instruction(&Instruction::LocalGet(i as u32));
		}
		func.instruction(&Instruction::StructNew(type_idx));
		func.instruction(&Instruction::End);

		self.code.function(&func);

		// Export as new_TypeName
		let func_name = format!("new_{}", type_def.name);
		// Leak the string to get a 'static str for the export
		let func_name_static: &'static str = Box::leak(func_name.clone().into_boxed_str());
		self.exports
			.export(func_name_static, ExportKind::Func, self.next_func_idx);
		self.next_func_idx += 1;
	}


	/// Emit constructor functions for the compact Node
	fn emit_constructors(&mut self) {
		let node_ref = self.node_ref(false);
		let node_ref_nullable = self.node_ref(true);

		// new_empty() -> (ref $Node)
		if self.should_emit_function("new_empty") {
			let func_type = self.types.len();
			self.types.ty().function(vec![], vec![Ref(node_ref)]);
			self.functions.function(func_type);
			let mut func = Function::new(vec![]);
			self.emit_kind(&mut func, Kind::Empty);
			func.instruction(&Instruction::RefNull(any_heap_type()));
			func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_type)));
			func.instruction(&Instruction::StructNew(self.node_type));
			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("new_empty");
			self.exports.export("new_empty", ExportKind::Func, idx);
		}

		// new_int(i64) -> (ref $Node) - box the i64 in $i64box
		if self.should_emit_function("new_int") {
			let func_type = self.types.len();
			self.types.ty().function(vec![ValType::I64], vec![Ref(node_ref)]);
			self.functions.function(func_type);
			let mut func = Function::new(vec![]);
			self.emit_kind(&mut func, Kind::Int);
			// Box the i64: create $i64box struct
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::StructNew(self.i64_box_type));
			func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_type)));
			func.instruction(&Instruction::StructNew(self.node_type));
			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("new_int");
			self.exports.export("new_int", ExportKind::Func, idx);
		}

		// new_float(f64) -> (ref $Node) - box the f64 in $f64box
		if self.should_emit_function("new_float") {
			let func_type = self.types.len();
			self.types.ty().function(vec![ValType::F64], vec![Ref(node_ref)]);
			self.functions.function(func_type);
			let mut func = Function::new(vec![]);
			self.emit_kind(&mut func, Kind::Float);
			// Box the f64: create $f64box struct
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::StructNew(self.f64_box_type));
			func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_type)));
			func.instruction(&Instruction::StructNew(self.node_type));
			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("new_float");
			self.exports.export("new_float", ExportKind::Func, idx);
		}

		// new_codepoint(i32) -> (ref $Node) - use i31ref for codepoint
		if self.should_emit_function("new_codepoint") {
			let func_type = self.types.len();
			self.types.ty().function(vec![ValType::I32], vec![Ref(node_ref)]);
			self.functions.function(func_type);
			let mut func = Function::new(vec![]);
			self.emit_kind(&mut func, Kind::Codepoint);
			// Convert i32 to i31ref
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::RefI31);
			func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_type)));
			func.instruction(&Instruction::StructNew(self.node_type));
			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("new_codepoint");
			self.exports.export("new_codepoint", ExportKind::Func, idx);
		}

		// new_text(ptr: i32, len: i32) -> (ref $Node)
		// Use $String struct for string data
		if self.should_emit_function("new_text") {
			let func_type = self.types.len();
			self.types
				.ty()
				.function(vec![ValType::I32, ValType::I32], vec![Ref(node_ref)]);
			self.functions.function(func_type);
			let mut func = Function::new(vec![]);
			self.emit_kind(&mut func, Kind::Text);
			// Create $String struct with ptr and len
			func.instruction(&Instruction::LocalGet(0)); // ptr
			func.instruction(&Instruction::LocalGet(1)); // len
			func.instruction(&Instruction::StructNew(self.string_type));
			func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_type)));
			func.instruction(&Instruction::StructNew(self.node_type));
			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("new_text");
			self.exports.export("new_text", ExportKind::Func, idx);
		}

		// new_symbol(ptr: i32, len: i32) -> (ref $Node)
		// Use $String struct for string data
		if self.should_emit_function("new_symbol") {
			let func_type = self.types.len();
			self.types
				.ty()
				.function(vec![ValType::I32, ValType::I32], vec![Ref(node_ref)]);
			self.functions.function(func_type);
			let mut func = Function::new(vec![]);
			self.emit_kind(&mut func, Kind::Symbol);
			// Create $String struct with ptr and len
			func.instruction(&Instruction::LocalGet(0)); // ptr
			func.instruction(&Instruction::LocalGet(1)); // len
			func.instruction(&Instruction::StructNew(self.string_type));
			func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_type)));
			func.instruction(&Instruction::StructNew(self.node_type));
			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("new_symbol");
			self.exports.export("new_symbol", ExportKind::Func, idx);
		}

		// new_key(key: ref $Node, value: ref $Node, op_info: i64) -> (ref $Node)
		// data = key node (cast to any), value = value node
		// kind = (op_info << 8) | Kind::Key
		if self.should_emit_function("new_key") {
			let func_type = self.types.len();
			self.types.ty().function(
				vec![Ref(node_ref_nullable), Ref(node_ref_nullable), ValType::I64],
				vec![Ref(node_ref)],
			);
			self.functions.function(func_type);
			let mut func = Function::new(vec![]);
			// Compute kind: (op_info << 8) | Key
			func.instruction(&Instruction::LocalGet(2)); // op_info
			func.instruction(&Instruction::I64Const(8));
			func.instruction(&Instruction::I64Shl);
			self.emit_kind(&mut func, Kind::Key);
			func.instruction(&Instruction::I64Or);
			func.instruction(&Instruction::LocalGet(0)); // key node as data (auto-cast to any)
			func.instruction(&Instruction::LocalGet(1)); // value node
			func.instruction(&Instruction::StructNew(self.node_type));
			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("new_key");
			self.exports.export("new_key", ExportKind::Func, idx);
		}

		// new_type(name: ref $Node, body: ref $Node) -> (ref $Node)
		// data = name node (cast to any), value = body node (fields)
		if self.should_emit_function("new_type") {
			let func_type = self.types.len();
			self.types.ty().function(
				vec![Ref(node_ref_nullable), Ref(node_ref_nullable)],
				vec![Ref(node_ref)],
			);
			self.functions.function(func_type);
			let mut func = Function::new(vec![]);
			self.emit_kind(&mut func, Kind::TypeDef);
			func.instruction(&Instruction::LocalGet(0)); // name node as data (auto-cast to any)
			func.instruction(&Instruction::LocalGet(1)); // body node (fields)
			func.instruction(&Instruction::StructNew(self.node_type));
			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("new_type");
			self.exports.export("new_type", ExportKind::Func, idx);
		}

		// new_list(first: ref $Node, rest: ref $Node, bracket_info: i64) -> (ref $Node)
		// kind = List + bracket encoding, data = first, value = rest
		if self.should_emit_function("new_list") {
			let func_type = self.types.len();
			self.types.ty().function(
				vec![Ref(node_ref_nullable), Ref(node_ref_nullable), ValType::I64],
				vec![Ref(node_ref)],
			);
			self.functions.function(func_type);
			let mut func = Function::new(vec![]);
			// kind = (bracket_info << 8) | Kind::List
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::I64Const(8));
			func.instruction(&Instruction::I64Shl);
			self.emit_kind(&mut func, Kind::List);
			func.instruction(&Instruction::I64Or);
			func.instruction(&Instruction::LocalGet(0)); // first as data
			func.instruction(&Instruction::LocalGet(1)); // rest as value
			func.instruction(&Instruction::StructNew(self.node_type));
			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("new_list");
			self.exports.export("new_list", ExportKind::Func, idx);
		}

		// list_at(list: ref $Node, index: i64) -> i64
		// Get the numeric value of the element at index (1-based)
		// Traverses linked list: for index N, follow value pointer N-1 times, return data
		if self.should_emit_function("list_at") {
			let func_type = self.types.len();
			self.types
				.ty()
				.function(vec![Ref(node_ref), ValType::I64], vec![ValType::I64]);
			self.functions.function(func_type);

			// Locals: 0=list, 1=index, 2=current (loop variable)
			let mut func = Function::new(vec![(1, Ref(node_ref_nullable))]);

			// current = list
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalSet(2));

			// Loop while index > 1: current = current.value, index--
			func.instruction(&Instruction::Block(BlockType::Empty));
			func.instruction(&Instruction::Loop(BlockType::Empty));

			// if index <= 1, break
			func.instruction(&Instruction::LocalGet(1));
			func.instruction(&Instruction::I64Const(1));
			func.instruction(&Instruction::I64LeS);
			func.instruction(&Instruction::BrIf(1)); // break to outer block

			// current = current.value (field 2)
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.node_type,
				field_index: 2,
			});
			func.instruction(&Instruction::LocalSet(2));

			// index = index - 1
			func.instruction(&Instruction::LocalGet(1));
			func.instruction(&Instruction::I64Const(1));
			func.instruction(&Instruction::I64Sub);
			func.instruction(&Instruction::LocalSet(1));

			// continue loop
			func.instruction(&Instruction::Br(0));
			func.instruction(&Instruction::End); // end loop
			func.instruction(&Instruction::End); // end block

			// Get current.data (which is a ref to the element Node, cast from anyref)
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.node_type,
				field_index: 1, // data field (anyref holding ref $Node)
			});
			// Cast anyref to ref $Node
			func.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(self.node_type)));
			// Get the inner node's data field (which holds the i64_box)
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.node_type,
				field_index: 1, // data field of the element node
			});
			// Cast to ref $i64box and get the i64 value
			func.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(self.i64_box_type)));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.i64_box_type,
				field_index: 0,
			});

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("list_at");
			self.exports.export("list_at", ExportKind::Func, idx);
		}

		// list_node_at(list: ref $Node, index: i64) -> ref $Node
		// Get the element node at index (1-based), returns the node itself (for symbol/text lists)
		if self.should_emit_function("list_node_at") {
			let func_type = self.types.len();
			self.types
				.ty()
				.function(vec![Ref(node_ref), ValType::I64], vec![Ref(node_ref)]);
			self.functions.function(func_type);

			// Locals: 0=list, 1=index, 2=current (loop variable)
			let mut func = Function::new(vec![(1, Ref(node_ref_nullable))]);

			// current = list
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalSet(2));

			// Loop while index > 1: current = current.value, index--
			func.instruction(&Instruction::Block(BlockType::Empty));
			func.instruction(&Instruction::Loop(BlockType::Empty));

			// if index <= 1, break
			func.instruction(&Instruction::LocalGet(1));
			func.instruction(&Instruction::I64Const(1));
			func.instruction(&Instruction::I64LeS);
			func.instruction(&Instruction::BrIf(1)); // break to outer block

			// current = current.value (field 2)
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.node_type,
				field_index: 2,
			});
			func.instruction(&Instruction::LocalSet(2));

			// index = index - 1
			func.instruction(&Instruction::LocalGet(1));
			func.instruction(&Instruction::I64Const(1));
			func.instruction(&Instruction::I64Sub);
			func.instruction(&Instruction::LocalSet(1));

			// continue loop
			func.instruction(&Instruction::Br(0));
			func.instruction(&Instruction::End); // end loop
			func.instruction(&Instruction::End); // end block

			// Get current.data (which is a ref to the element Node, cast from anyref)
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.node_type,
				field_index: 1, // data field (anyref holding ref $Node)
			});
			// Cast anyref to ref $Node and return it directly
			func.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(self.node_type)));

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("list_node_at");
			self.exports.export("list_node_at", ExportKind::Func, idx);
		}

		// node_count(node: ref $Node) -> i64
		// Count the number of elements in a list/block by traversing the value chain
		if self.should_emit_function("node_count") {
			let func_type = self.types.len();
			self.types.ty().function(vec![Ref(node_ref)], vec![ValType::I64]);
			self.functions.function(func_type);

			// Locals: 0=node, 1=count, 2=current
			let mut func = Function::new(vec![(1, ValType::I64), (1, Ref(node_ref_nullable))]);

			// count = 0
			func.instruction(&Instruction::I64Const(0));
			func.instruction(&Instruction::LocalSet(1));

			// current = node
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalSet(2));

			// Loop: while current is not null, count++ and current = current.value
			func.instruction(&Instruction::Block(BlockType::Empty));
			func.instruction(&Instruction::Loop(BlockType::Empty));

			// if current is null, break
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::RefIsNull);
			func.instruction(&Instruction::BrIf(1)); // break to outer block

			// count = count + 1
			func.instruction(&Instruction::LocalGet(1));
			func.instruction(&Instruction::I64Const(1));
			func.instruction(&Instruction::I64Add);
			func.instruction(&Instruction::LocalSet(1));

			// current = current.value (field 2)
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.node_type,
				field_index: 2,
			});
			func.instruction(&Instruction::LocalSet(2));

			// continue loop
			func.instruction(&Instruction::Br(0));
			func.instruction(&Instruction::End); // end loop
			func.instruction(&Instruction::End); // end block

			// return count
			func.instruction(&Instruction::LocalGet(1));

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("node_count");
			self.exports.export("node_count", ExportKind::Func, idx);
		}

		// string_char_at(node: ref $Node, index: i64) -> ref $Node
		// Get the character at index (1-based) from a Text/Symbol node, returns Codepoint node
		if self.should_emit_function("string_char_at") {
			let func_type = self.types.len();
			self.types
				.ty()
				.function(vec![Ref(node_ref), ValType::I64], vec![Ref(node_ref)]);
			self.functions.function(func_type);

			let mut func = Function::new(vec![]);

			// Get node.data (which is a ref $String)
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.node_type,
				field_index: 1, // data field
			});
			// Cast anyref to ref $String
			func.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(self.string_type)));

			// Get ptr from $String
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.string_type,
				field_index: 0, // ptr
			});

			// Add (index - 1) to ptr for 1-based indexing
			func.instruction(&Instruction::LocalGet(1)); // index
			func.instruction(&Instruction::I32WrapI64);
			func.instruction(&I32Const(1));
			func.instruction(&Instruction::I32Sub);
			func.instruction(&Instruction::I32Add); // ptr + (index - 1)

			// Load byte from memory at that address
			func.instruction(&Instruction::I32Load8U(MemArg {
				offset: 0,
				align: 0,
				memory_index: 0,
			}));

			// Create Codepoint node with this value
			self.emit_call(&mut func, "new_codepoint");

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("string_char_at");
			self.exports.export("string_char_at", ExportKind::Func, idx);
		}

		// node_index_at(node: ref $Node, index: i64) -> ref $Node
		// Runtime dispatch: for Text/Symbol call string_char_at, for List/Block call list_node_at
		if self.should_emit_function("node_index_at") {
			let func_type = self.types.len();
			self.types
				.ty()
				.function(vec![Ref(node_ref), ValType::I64], vec![Ref(node_ref)]);
			self.functions.function(func_type);

			let mut func = Function::new(vec![]);

			// Get node.kind
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.node_type,
				field_index: 0, // kind field
			});

			// Check if kind is Text (3) or Symbol (5)
			// kind == 3 || kind == 5 means it's a string
			func.instruction(&Instruction::I64Const(3)); // Kind::Text
			func.instruction(&Instruction::I64Eq);
			func.instruction(&Instruction::If(BlockType::Result(Ref(node_ref))));
			// It's a Text - call string_char_at
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalGet(1));
			self.emit_call(&mut func, "string_char_at");
			func.instruction(&Instruction::Else);
			// Check for Symbol
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.node_type,
				field_index: 0,
			});
			func.instruction(&Instruction::I64Const(5)); // Kind::Symbol
			func.instruction(&Instruction::I64Eq);
			func.instruction(&Instruction::If(BlockType::Result(Ref(node_ref))));
			// It's a Symbol - call string_char_at
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalGet(1));
			self.emit_call(&mut func, "string_char_at");
			func.instruction(&Instruction::Else);
			// Otherwise it's a list - call list_node_at
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalGet(1));
			self.emit_call(&mut func, "list_node_at");
			func.instruction(&Instruction::End); // end inner if
			func.instruction(&Instruction::End); // end outer if

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("node_index_at");
			self.exports.export("node_index_at", ExportKind::Func, idx);
		}

		// list_set_at(list: ref $Node, index: i64, value: i64) -> i64
		// Set the numeric value at index (1-based) and return the value
		if self.should_emit_function("list_set_at") {
			let func_type = self.types.len();
			self.types
				.ty()
				.function(vec![Ref(node_ref), ValType::I64, ValType::I64], vec![ValType::I64]);
			self.functions.function(func_type);

			// Locals: 0=list, 1=index, 2=value, 3=current (loop variable)
			let mut func = Function::new(vec![(1, Ref(node_ref_nullable))]);

			// current = list
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalSet(3));

			// Loop while index > 1: current = current.value, index--
			func.instruction(&Instruction::Block(BlockType::Empty));
			func.instruction(&Instruction::Loop(BlockType::Empty));

			// if index <= 1, break
			func.instruction(&Instruction::LocalGet(1));
			func.instruction(&Instruction::I64Const(1));
			func.instruction(&Instruction::I64LeS);
			func.instruction(&Instruction::BrIf(1)); // break to outer block

			// current = current.value (field 2)
			func.instruction(&Instruction::LocalGet(3));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.node_type,
				field_index: 2,
			});
			func.instruction(&Instruction::LocalSet(3));

			// index = index - 1
			func.instruction(&Instruction::LocalGet(1));
			func.instruction(&Instruction::I64Const(1));
			func.instruction(&Instruction::I64Sub);
			func.instruction(&Instruction::LocalSet(1));

			// continue loop
			func.instruction(&Instruction::Br(0));
			func.instruction(&Instruction::End); // end loop
			func.instruction(&Instruction::End); // end block

			// Get current.data (which is the wrapper Node for the element)
			func.instruction(&Instruction::LocalGet(3));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.node_type,
				field_index: 1, // data field
			});
			// Cast anyref to ref $Node
			func.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(self.node_type)));

			// Create new i64box with the value
			func.instruction(&Instruction::LocalGet(2)); // value
			func.instruction(&Instruction::StructNew(self.i64_box_type));

			// Set the inner node's data field to the new i64box
			func.instruction(&Instruction::StructSet {
				struct_type_index: self.node_type,
				field_index: 1, // data field
			});

			// Return the value
			func.instruction(&Instruction::LocalGet(2));

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("list_set_at");
			self.exports.export("list_set_at", ExportKind::Func, idx);
		}

		// string_set_char_at(node: ref $Node, index: i64, value: i64) -> i64
		// Set the character at index (1-based) in a Text/Symbol node, returns the value
		if self.should_emit_function("string_set_char_at") {
			let func_type = self.types.len();
			self.types
				.ty()
				.function(vec![Ref(node_ref), ValType::I64, ValType::I64], vec![ValType::I64]);
			self.functions.function(func_type);

			let mut func = Function::new(vec![]);

			// Get node.data (which is a ref $String)
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.node_type,
				field_index: 1, // data field
			});
			// Cast anyref to ref $String
			func.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(self.string_type)));

			// Get ptr from $String
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.string_type,
				field_index: 0, // ptr
			});

			// Calculate address: ptr + (index - 1) for 1-based indexing
			func.instruction(&Instruction::LocalGet(1)); // index
			func.instruction(&Instruction::I32WrapI64);
			func.instruction(&I32Const(1));
			func.instruction(&Instruction::I32Sub);
			func.instruction(&Instruction::I32Add); // address = ptr + (index - 1)

			// Store the value as a byte
			func.instruction(&Instruction::LocalGet(2)); // value
			func.instruction(&Instruction::I32WrapI64);
			func.instruction(&Instruction::I32Store8(MemArg {
				offset: 0,
				align: 0,
				memory_index: 0,
			}));

			// Return the value
			func.instruction(&Instruction::LocalGet(2));

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("string_set_char_at");
			self.exports.export("string_set_char_at", ExportKind::Func, idx);
		}

		// node_set_at(node: ref $Node, index: i64, value: i64) -> i64
		// Runtime dispatch for index assignment: string_set_char_at or list_set_at
		if self.should_emit_function("node_set_at") {
			let func_type = self.types.len();
			self.types
				.ty()
				.function(vec![Ref(node_ref), ValType::I64, ValType::I64], vec![ValType::I64]);
			self.functions.function(func_type);

			let mut func = Function::new(vec![]);

			// Get node.kind
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.node_type,
				field_index: 0, // kind field
			});

			// Check if kind is Text (3) or Symbol (5)
			func.instruction(&Instruction::I64Const(3)); // Kind::Text
			func.instruction(&Instruction::I64Eq);
			func.instruction(&Instruction::If(BlockType::Result(ValType::I64)));
			// It's a Text - call string_set_char_at
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalGet(1));
			func.instruction(&Instruction::LocalGet(2));
			self.emit_call(&mut func, "string_set_char_at");
			func.instruction(&Instruction::Else);
			// Check for Symbol
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.node_type,
				field_index: 0,
			});
			func.instruction(&Instruction::I64Const(5)); // Kind::Symbol
			func.instruction(&Instruction::I64Eq);
			func.instruction(&Instruction::If(BlockType::Result(ValType::I64)));
			// It's a Symbol - call string_set_char_at
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalGet(1));
			func.instruction(&Instruction::LocalGet(2));
			self.emit_call(&mut func, "string_set_char_at");
			func.instruction(&Instruction::Else);
			// Otherwise it's a list - call list_set_at
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalGet(1));
			func.instruction(&Instruction::LocalGet(2));
			self.emit_call(&mut func, "list_set_at");
			func.instruction(&Instruction::End); // end inner if
			func.instruction(&Instruction::End); // end outer if

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("node_set_at");
			self.exports.export("node_set_at", ExportKind::Func, idx);
		}

		// Emit helper functions
		self.emit_getters();
		self.emit_math_helpers();
	}

	fn emit_getters(&mut self) {
		let node_ref = self.node_ref(true);

		// get_kind(node: ref $Node) -> i64
		let func_type = self.types.len();
		self.types.ty().function(vec![Ref(node_ref)], vec![ValType::I64]);
		self.functions.function(func_type);
		let mut func = Function::new(vec![]);
		func.instruction(&Instruction::LocalGet(0));
		func.instruction(&Instruction::StructGet {
			struct_type_index: self.node_type,
			field_index: 0,
		});
		func.instruction(&Instruction::End);
		self.code.function(&func);
		let idx = self.register_func("get_kind");
		self.exports.export("get_kind", ExportKind::Func, idx);
	}

	/// Emit math helper functions (i64_pow, etc.)
	fn emit_math_helpers(&mut self) {
		// i64_pow(base: i64, exp: i64) -> i64
		// Computes base^exp using a loop
		if self.should_emit_function("i64_pow") {
			let func_type = self.types.len();
			self.types
				.ty()
				.function(vec![ValType::I64, ValType::I64], vec![ValType::I64]);
			self.functions.function(func_type);

			// Locals: 0=base, 1=exp, 2=result
			let mut func = Function::new(vec![(1, ValType::I64)]);

			// result = 1
			func.instruction(&Instruction::I64Const(1));
			func.instruction(&Instruction::LocalSet(2));

			// block $done
			func.instruction(&Instruction::Block(BlockType::Empty));
			// loop $loop
			func.instruction(&Instruction::Loop(BlockType::Empty));

			// br_if $done (i64.eqz (local.get $exp))
			func.instruction(&Instruction::LocalGet(1)); // exp
			func.instruction(&Instruction::I64Eqz);
			func.instruction(&Instruction::BrIf(1)); // break to $done

			// result = result * base
			func.instruction(&Instruction::LocalGet(2)); // result
			func.instruction(&Instruction::LocalGet(0)); // base
			func.instruction(&Instruction::I64Mul);
			func.instruction(&Instruction::LocalSet(2));

			// exp = exp - 1
			func.instruction(&Instruction::LocalGet(1)); // exp
			func.instruction(&Instruction::I64Const(1));
			func.instruction(&Instruction::I64Sub);
			func.instruction(&Instruction::LocalSet(1));

			// br $loop
			func.instruction(&Instruction::Br(0));

			// end loop
			func.instruction(&Instruction::End);
			// end block
			func.instruction(&Instruction::End);

			// return result
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::End);

			self.code.function(&func);
			let idx = self.register_func("i64_pow");
			self.exports.export("i64_pow", ExportKind::Func, idx);
		}
	}

	/// Allocate a string in linear memory
	fn allocate_string(&mut self, s: &str) -> (u32, u32) {
		if let Some(&offset) = self.ctx.string_table.get(s) {
			return (offset, s.len() as u32);
		}
		let offset = self.next_data_offset;
		let bytes = s.as_bytes();
		self.data
			.active(0, &ConstExpr::i32_const(offset as i32), bytes.iter().copied());
		self.ctx.string_table.insert(s.to_string(), offset);
		self.next_data_offset += bytes.len() as u32;
		(offset, bytes.len() as u32)
	}

	/// Emit main function that constructs the node
	pub fn emit_node_main(&mut self, node: &Node) {
		// Pre-pass: collect variables first so scope is populated
		let temp_locals = collect_variables(node, &mut self.scope);

		// Allocate strings and update Local data pointers
		self.collect_and_allocate_strings(node);
		let var_count = self.scope.local_count();
		self.next_temp_local = var_count; // Temp locals start after variables

		let node_ref = self.node_ref(false);
		let func_type = self.types.len();
		// WASI mode: return i64 directly instead of Node ref
		if self.emit_wasi_imports {
			self.types.ty().function(vec![], vec![ValType::I64]);
		} else {
			self.types.ty().function(vec![], vec![Ref(node_ref)]);
		}
		self.functions.function(func_type);

		// Build locals list based on variable types
		// Each variable gets its own entry, then temp locals (all i64)
		let mut locals: Vec<(u32, ValType)> = Vec::new();

		// Sort locals by position to ensure correct order
		let mut sorted_locals: Vec<_> = self.scope.locals.values().collect();
		sorted_locals.sort_by_key(|l| l.position);

		for local in sorted_locals {
			if local.kind.is_ref() {
				locals.push((1, Ref(node_ref)));
			} else if local.kind.is_float() {
				locals.push((1, ValType::F64));
			} else {
				locals.push((1, ValType::I64));
			}
		}

		// Add temp locals (i64 for now)
		if temp_locals > 0 {
			locals.push((temp_locals, ValType::I64));
		}

		let mut func = Function::new(locals);
		self.emit_node_instructions(&mut func, node);
		func.instruction(&Instruction::End);

		self.code.function(&func);
		self.exports.export("main", ExportKind::Func, self.next_func_idx);
		self.next_func_idx += 1;
	}

	fn collect_and_allocate_strings(&mut self, node: &Node) {
		let node = node.drop_meta();
		match node {
			Node::Text(s) | Node::Symbol(s) => {
				self.allocate_string(s);
			}
			Node::Key(key, op, value) => {
				// Track string variable assignments: x='hello' or x:='hello'
				// Store data pointer/length in the Local for later use by WASI puts
				if matches!(op, Op::Assign | Op::Define) {
					if let Node::Symbol(var_name) = key.drop_meta() {
						if let Node::Text(s) = value.drop_meta() {
							let (ptr, len) = self.allocate_string(s);
							self.scope.set_local_data(var_name, ptr, len);
						}
					}
				}
				self.collect_and_allocate_strings(key);
				self.collect_and_allocate_strings(value);
			}
			Node::List(items, _, _) => {
				for item in items {
					self.collect_and_allocate_strings(item);
				}
			}
			Node::Data(dada) => {
				self.allocate_string(&dada.type_name);
			}
			Node::Type { name, body } => {
				self.collect_and_allocate_strings(name);
				self.collect_and_allocate_strings(body);
			}
			_ => {}
		}
	}

	fn emit_call(&mut self, func: &mut Function, name: &'static str) {
		self.ctx.used_functions.insert(name);
		func.instruction(&Instruction::Call(self.func_index(name)));
	}

	fn emit_node_null(&self, func: &mut Function) {
		func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_type)));
	}

	/// Emit instructions to construct a Node
	fn emit_node_instructions(&mut self, func: &mut Function, node: &Node) {
		let node = node.drop_meta();

		match node {
			Node::Empty => {
				self.emit_call(func, "new_empty");
			}
			Node::Number(num) => match num {
				Number::Int(i) => {
					func.instruction(&Instruction::I64Const(*i));
					self.emit_call(func, "new_int");
				}
				Number::Float(f) => {
					func.instruction(&Instruction::F64Const(Ieee64::new(f.to_bits())));
					self.emit_call(func, "new_float");
				}
				_ => {
					self.emit_call(func, "new_empty");
				}
			},
			Node::Text(s) => {
				self.emit_string_call(func, s, "new_text");
			}
			Node::Char(c) => {
				func.instruction(&I32Const(*c as i32));
				self.emit_call(func, "new_codepoint");
			}
			Node::Symbol(s) => {
				// Check if this is a local variable lookup
				if let Some(local) = self.scope.lookup(s) {
					func.instruction(&Instruction::LocalGet(local.position));
					if local.kind.is_ref() {
						return; // Already a Node reference
					} else if local.kind.is_float() {
						self.emit_call(func, "new_float");
					} else {
						self.emit_call(func, "new_int");
					}
					return;
				}
				// Check if this is a global variable lookup
				if let Some(&(idx, kind)) = self.ctx.user_globals.get(s) {
					func.instruction(&Instruction::GlobalGet(idx));
					if kind.is_float() {
						self.emit_call(func, "new_float");
					} else {
						self.emit_call(func, "new_int");
					}
					return;
				}
				self.emit_string_call(func, s, "new_symbol");
			}
			Node::Key(left, op, right) => {
				// Handle global keyword: global:Key(name, =, value)
				if let Node::Symbol(kw) = left.drop_meta() {
					if kw == "global" {
						self.emit_global_declaration(func, right);
						return;
					}
					// Handle fetch URL - call host.fetch and return Text node
					if kw == "fetch" && self.emit_host_imports {
						self.emit_fetch_call(func, right);
						return;
					}
				}
				// Handle x = fetch URL pattern: Key(Assign, x, List[fetch, URL])
				if (*op == Op::Assign || *op == Op::Define) && self.emit_host_imports {
					if let Node::Symbol(var_name) = left.drop_meta() {
						if let Node::List(items, _, _) = right.drop_meta() {
							if items.len() == 2 {
								if let Node::Symbol(s) = items[0].drop_meta() {
									if s == "fetch" {
										// Emit fetch call - result is a Text node (ref $Node)
										self.emit_fetch_call(func, &items[1]);
										// Store in ref-type local variable
										if let Some(local) = self.scope.lookup(var_name) {
											func.instruction(&Instruction::LocalTee(local.position));
										}
										return;
									}
								}
							}
						}
					}
				}
				// Skip user function definitions - they're already compiled
				if *op == Op::Define {
					if let Node::Symbol(name) = left.drop_meta() {
						if self.ctx.user_functions.contains_key(name) {
							// Function definitions don't produce a value
							// The caller (statement sequence handler) should skip this
							return;
						}
					}
				}
				// Route to emit_arithmetic for:
				// - Arithmetic/comparison/logical ops (when both operands are numeric)
				// - Define (:=) always creates a numeric local (unless it's a list)
				// - Assign (=) only if LHS is a known numeric variable
				// - Compound assignments (+=, -=, etc.)
				let is_numeric_assign = *op == Op::Assign
					&& matches!(left.drop_meta(), Node::Symbol(s) if {
						self.scope.lookup(s).is_some_and(|l| !l.kind.is_ref())
					});
				let is_numeric_define = *op == Op::Define
					&& matches!(left.drop_meta(), Node::Symbol(s) if {
						self.scope.lookup(s).is_some_and(|l| !l.kind.is_ref())
					});
				// Handle ref-type variable assignment (lists, etc.)
				let is_ref_assign = (*op == Op::Assign || *op == Op::Define)
					&& matches!(left.drop_meta(), Node::Symbol(s) if {
						self.scope.lookup(s).is_some_and(|l| l.kind.is_ref())
					});
				if is_ref_assign {
					if let Node::Symbol(name) = left.drop_meta() {
						// Emit the right side as a Node reference
						self.emit_node_instructions(func, right);
						// Store in ref-type local
						if let Some(local) = self.scope.lookup(name) {
							func.instruction(&Instruction::LocalTee(local.position));
						}
					}
					return;
				}
				// Handle index assignment: node#index = value → node_set_at(node, index, value)
				// Works for both lists and strings via runtime dispatch
				if *op == Op::Assign {
					if let Node::Key(node_expr, Op::Hash, index_expr) = left.drop_meta() {
						// Emit node (string or list ref)
						self.emit_node_instructions(func, node_expr);
						// Emit index (as i64)
						self.emit_numeric_value(func, index_expr);
						// Emit value (as i64)
						self.emit_numeric_value(func, right);
						// Call node_set_at which dispatches to string_set_char_at or list_set_at
						self.emit_call(func, "node_set_at");
						// Wrap result as Node
						self.emit_call(func, "new_int");
						return;
					}
				}
				// For logical ops with non-numeric operands, use Node-returning truthy path
				let both_numeric = self.is_numeric(left) && self.is_numeric(right);
				if op.is_logical() && !both_numeric {
					self.emit_truthy_logical(func, left, op, right);
				} else if op.is_arithmetic()
					|| op.is_comparison()
					|| op.is_logical()
					|| is_numeric_define
					|| is_numeric_assign
					|| op.is_compound_assign()
				{
					self.emit_arithmetic(func, left, op, right);
				} else if *op == Op::Square || *op == Op::Cube {
					// Suffix operators: x² = x*x, x³ = x*x*x
					// Emit the operand multiple times (safe for pure expressions)
					let use_float = self.get_type(left).is_float();
					if use_float {
						self.emit_float_value(func, left);
						self.emit_float_value(func, left);
						func.instruction(&Instruction::F64Mul);
						if *op == Op::Cube {
							self.emit_float_value(func, left);
							func.instruction(&Instruction::F64Mul);
						}
						self.emit_call(func, "new_float");
					} else {
						self.emit_numeric_value(func, left);
						self.emit_numeric_value(func, left);
						func.instruction(&Instruction::I64Mul);
						if *op == Op::Cube {
							self.emit_numeric_value(func, left);
							func.instruction(&Instruction::I64Mul);
						}
						self.emit_call(func, "new_int");
					}
				} else if op.is_prefix() && matches!(left.drop_meta(), Node::Empty) {
					// Prefix operators: √x, -x, !x, ‖x‖
					match op {
						Op::Sqrt => {
							// √x = sqrt(x), returns float
							self.emit_float_value(func, right);
							func.instruction(&Instruction::F64Sqrt);
							self.emit_call(func, "new_float");
						}
						Op::Neg => {
							// -x = 0 - x
							let use_float = self.get_type(right).is_float();
							if use_float {
								func.instruction(&Instruction::F64Const(0.0.into()));
								self.emit_float_value(func, right);
								func.instruction(&Instruction::F64Sub);
								self.emit_call(func, "new_float");
							} else {
								func.instruction(&Instruction::I64Const(0));
								self.emit_numeric_value(func, right);
								func.instruction(&Instruction::I64Sub);
								self.emit_call(func, "new_int");
							}
						}
						Op::Not => {
							// !x = x == 0
							self.emit_numeric_value(func, right);
							func.instruction(&Instruction::I64Eqz);
							func.instruction(&Instruction::I64ExtendI32U);
							self.emit_call(func, "new_int");
						}
						Op::Abs => {
							// ‖x‖ = abs(x)
							let use_float = self.get_type(right).is_float();
							if use_float {
								self.emit_float_value(func, right);
								func.instruction(&Instruction::F64Abs);
								self.emit_call(func, "new_float");
							} else {
								// i64 abs: if x < 0 then -x else x
								self.emit_numeric_value(func, right);
								func.instruction(&Instruction::LocalTee(self.next_temp_local));
								func.instruction(&Instruction::I64Const(0));
								func.instruction(&Instruction::I64LtS);
								func.instruction(&Instruction::If(BlockType::Result(ValType::I64)));
								func.instruction(&Instruction::I64Const(0));
								func.instruction(&Instruction::LocalGet(self.next_temp_local));
								func.instruction(&Instruction::I64Sub);
								func.instruction(&Instruction::Else);
								func.instruction(&Instruction::LocalGet(self.next_temp_local));
								func.instruction(&Instruction::End);
								self.emit_call(func, "new_int");
							}
						}
						_ => {
							// Fallback: emit as Key node
							self.emit_node_instructions(func, left);
							self.emit_node_instructions(func, right);
							func.instruction(&Instruction::I64Const(op_to_code(op)));
							self.emit_call(func, "new_key");
						}
					}
				} else if *op == Op::Question {
					// Ternary: condition ? then : else
					self.emit_ternary(func, left, right);
				} else if *op == Op::Else {
					// Check if this is a full if-then-else or just a fallback operator
					if let Node::Key(_, Op::Then, _) = left.drop_meta() {
						// If-then-else: ((if condition) then then_expr) else else_expr
						self.emit_if_then_else(func, left, Some(right));
					} else {
						// Standalone else acts like truthy or: `false else 3` → 3
						self.emit_truthy_logical(func, left, &Op::Or, right);
					}
				} else if *op == Op::Then {
					// If-then (no else): (if condition) then then_expr
					// Construct the full structure for emit_if_then_else
					let full_node = Node::Key(left.clone(), Op::Then, right.clone());
					self.emit_if_then_else(func, &full_node, None);
				} else if *op == Op::Do {
					// While loop: (while condition) do body
					self.emit_while_loop(func, left, right);
				} else if *op == Op::Hash {
					// Check if prefix (count) or infix (index)
					if matches!(left.drop_meta(), Node::Empty) {
						// Prefix #x = count - emit the node and call node_count
						self.emit_node_instructions(func, right);
						self.emit_call(func, "node_count");
						// node_count returns i64, wrap in new_int
						self.emit_call(func, "new_int");
					} else {
						// Infix x#y = indexing - dispatches to string_char_at or list_node_at at runtime
						self.emit_node_instructions(func, left); // emit node (string or list)
						self.emit_numeric_value(func, right); // emit index
						self.emit_call(func, "node_index_at"); // runtime dispatch
					}
				} else if *op == Op::As {
					// Type cast: value as type
					self.emit_cast(func, left, right);
				} else if *op == Op::Dot {
					// Method call syntax: obj.method() or obj.property
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
						match method.as_str() {
							"count" | "number" => {
								// obj.count() or obj.count returns element count
								self.emit_node_instructions(func, left);
								self.emit_call(func, "node_count");
								self.emit_call(func, "new_int");
								return;
							}
							"size" => {
								// obj.size() returns byte count (elements * 8)
								self.emit_node_instructions(func, left);
								self.emit_call(func, "node_count");
								func.instruction(&Instruction::I64Const(8));
								func.instruction(&Instruction::I64Mul);
								self.emit_call(func, "new_int");
								return;
							}
							_ => {}
						}
					}
					// Default: emit as Key node
					self.emit_node_instructions(func, left);
					self.emit_node_instructions(func, right);
					func.instruction(&Instruction::I64Const(op_to_code(op)));
					self.emit_call(func, "new_key");
				} else if *op == Op::Range || *op == Op::To {
					// Range operators: 0..3 (exclusive) or 0...3 / 0…3 (inclusive)
					self.emit_range(func, left, right, *op == Op::To);
				} else {
					self.emit_node_instructions(func, left);
					// For struct instances like Person{...}, emit block as list
					let right_node = right.drop_meta();
					if let Node::List(items, Bracket::Curly, sep) = right_node {
						// Convert curly block to square list, preserving inner ops
						let list_node = Node::List(items.clone(), Bracket::Square, sep.clone());
						self.emit_node_instructions(func, &list_node);
					} else {
						self.emit_node_instructions(func, right_node);
					}
					// Preserve the op for roundtrip
					func.instruction(&Instruction::I64Const(op_to_code(op)));
					self.emit_call(func, "new_key");
				}
			}
			Node::List(items, bracket, _separator) => {
				if items.is_empty() {
					self.emit_call(func, "new_empty");
					return;
				}
				if items.len() == 1 {
					// Check for zero-argument function call: (funcname)
					if *bracket == Bracket::Round {
						if let Node::Symbol(fn_name) = items[0].drop_meta() {
							if self.ctx.user_functions.contains_key(fn_name) {
								self.emit_user_function_call(func, fn_name, &[]);
								return;
							}
						}
					}
					self.emit_node_instructions(func, &items[0]);
					return;
				}
				// Check for fetch call: [Symbol("fetch"), url_node]
				if items.len() == 2 && self.emit_host_imports {
					if let Node::Symbol(s) = items[0].drop_meta() {
						if s == "fetch" {
							self.emit_fetch_call(func, &items[1]);
							return;
						}
					}
				}
				// Check for WASI calls: puts, puti, putl, putf, fd_write
				// These return raw i32 (WASI mode uses read_bytes_with_wasi which returns i32)
				if items.len() >= 2 && self.emit_wasi_imports {
					if let Node::Symbol(s) = items[0].drop_meta() {
						match s.as_str() {
							"puts" => {
								self.emit_wasi_puts(func, &items[1]);
								// fd_write returns i32, convert to i64 for main return
								func.instruction(&Instruction::I64ExtendI32S);
								return;
							}
							"puti" | "putl" => {
								self.emit_wasi_puti(func, &items[1]);
								// Return the value that was printed
								self.emit_numeric_value(func, &items[1]);
								return;
							}
							"putf" => {
								self.emit_wasi_putf(func, &items[1]);
								// fd_write returns i32, convert to i64
								func.instruction(&Instruction::I64ExtendI32S);
								return;
							}
							"fd_write" if items.len() >= 5 => {
								self.emit_wasi_fd_write_call(func, &items[1..]);
								return;
							}
							_ => {}
						}
					}
				}
				// Check for type() introspection: type(x) returns the type of x as a Symbol
				if items.len() == 2 {
					if let Node::Symbol(fn_name) = items[0].drop_meta() {
						if fn_name == "type" {
							// Get the type of the argument and emit it as a Symbol
							let kind = self.get_type(&items[1]);
							let type_name = kind.to_string();
							// Allocate the type name string and emit
							let (ptr, len) = self.allocate_string(&type_name);
							func.instruction(&I32Const(ptr as i32));
							func.instruction(&I32Const(len as i32));
							self.emit_call(func, "new_symbol");
							return;
						}
						if fn_name == "count" {
							// count(x) returns element count of list/array
							self.emit_node_instructions(func, &items[1]);
							self.emit_call(func, "node_count");
							self.emit_call(func, "new_int");
							return;
						}
						if fn_name == "size" {
							// size(x) returns byte count (element count * 8 for i64)
							self.emit_node_instructions(func, &items[1]);
							self.emit_call(func, "node_count");
							func.instruction(&Instruction::I64Const(8));
							func.instruction(&Instruction::I64Mul);
							self.emit_call(func, "new_int");
							return;
						}
						// ceil/floor/round: map to WASM built-in f64 instructions
						// Skip builtin handling if explicitly imported via FFI
						if !self.ctx.ffi_imports.contains_key(fn_name) {
							if fn_name == "ceil" {
								self.emit_float_value(func, &items[1]);
								func.instruction(&Instruction::F64Ceil);
								func.instruction(&Instruction::I64TruncF64S);
								self.emit_call(func, "new_int");
								return;
							}
							if fn_name == "floor" {
								self.emit_float_value(func, &items[1]);
								func.instruction(&Instruction::F64Floor);
								func.instruction(&Instruction::I64TruncF64S);
								self.emit_call(func, "new_int");
								return;
							}
							if fn_name == "round" {
								self.emit_float_value(func, &items[1]);
								func.instruction(&Instruction::F64Nearest);
								func.instruction(&Instruction::I64TruncF64S);
								self.emit_call(func, "new_int");
								return;
							}
						}
					}
				}
				// Check for type constructor calls: int('123'), str(123), char(0x41), etc.
				// But NOT typed variable declarations like "string x=fetch..."
				if items.len() == 2 {
					if let Node::Symbol(type_name) = items[0].drop_meta() {
						// Skip if second arg is assignment/define (typed variable declaration)
						let is_typed_decl = matches!(items[1].drop_meta(), Node::Key(_, Op::Assign | Op::Define, _));
						if !is_typed_decl {
							match type_name.as_str() {
								"int" | "float" | "str" | "string" | "String" | "char" | "bool" | "number" => {
									// Emit normalization hint: prefer 'x as int' over 'int(x)'
									norm::type_constructor(type_name, &items[1].to_string());
									if type_name == "str" || type_name == "String" {
										norm::string_type(type_name);
									}
									self.emit_cast(func, &items[1], &items[0]);
									return;
								}
								_ => {}
							}
						}
					}
				}
				// Check for range function: range start end
				if items.len() == 3 {
					if let Node::Symbol(fn_name) = items[0].drop_meta() {
						if fn_name == "range" {
							// range start end produces [start, start+1, ..., end] (inclusive)
							self.emit_range(func, &items[1], &items[2], true);
							return;
						}
					}
				}
				// Check for user function call: [Symbol("funcname"), arg1, arg2, ...]
				if items.len() >= 2 {
					if let Node::Symbol(fn_name) = items[0].drop_meta() {
						if self.ctx.user_functions.contains_key(fn_name) {
							// Check if it's a zero-arg call: (funcname Empty)
							if items.len() == 2 && matches!(items[1].drop_meta(), Node::Empty) {
								self.emit_user_function_call(func, fn_name, &[]);
							} else {
								self.emit_user_function_call(func, fn_name, &items[1..]);
							}
							return;
						}
						// Check for FFI function call
						if self.ctx.ffi_imports.contains_key(fn_name) {
							self.emit_ffi_call(func, fn_name, &items[1..], None);
							return;
						}
					}
				}
				// Check if this list contains type definitions (class/struct)
				// If so, treat as statement sequence and return last non-Type item
				let has_type_def = items.iter().any(|item| matches!(item.drop_meta(), Node::Type { .. }));
				if has_type_def {
					// Find last non-Type item to return
					let last_expr = items
						.iter()
						.rev()
						.find(|item| !matches!(item.drop_meta(), Node::Type { .. }));
					if let Some(expr) = last_expr {
						self.emit_node_instructions(func, expr);
					} else {
						// All items are Type definitions, return empty
						self.emit_call(func, "new_empty");
					}
					return;
				}
				// Check if this is a statement sequence with assignments/definitions/globals/indexing
				let is_statement_sequence = items.iter().any(|item| {
					let item = item.drop_meta();
					match item {
						Node::Key(_, Op::Assign | Op::Define, _) => true,
						Node::Key(_, Op::Hash, _) => true, // Index operation
						Node::Key(_, op, _) if op.is_compound_assign() => true,
						// global:... is a statement
						Node::Key(left, Op::Colon, _) => {
							matches!(left.drop_meta(), Node::Symbol(s) if s == "global")
						}
						// def/fun/fn syntax starts a statement sequence
						Node::List(list_items, _, _) if list_items.len() >= 2 => {
							if let Node::Symbol(s) = list_items[0].drop_meta() {
								// use/import for FFI also indicates statement sequence
								is_function_keyword(s) || s == "use" || s == "import"
							} else {
								false
							}
						}
						_ => false,
					}
				});

				if is_statement_sequence {
					// Execute statements in order, return last result
					// Filter out user function definitions (they don't produce values)
					let non_func_items: Vec<_> = items
						.iter()
						.filter(|item| {
							match item.drop_meta() {
								// Pattern: name := body (uses implicit `it` parameter)
								// or name(params...) := body
								Node::Key(left, Op::Define, _) => {
									if let Node::Symbol(name) = left.drop_meta() {
										if self.ctx.user_functions.contains_key(name) {
											return false;
										}
									}
									// Pattern: name(params...) := body
									if let Node::List(items, _, _) = left.drop_meta() {
										if !items.is_empty() {
											if let Node::Symbol(name) = items[0].drop_meta() {
												if self.ctx.user_functions.contains_key(name) {
													return false;
												}
											}
										}
									}
								}
								// Pattern: name(params...) = body
								Node::Key(left, Op::Assign, _) => {
									if let Node::List(items, _, _) = left.drop_meta() {
										if !items.is_empty() {
											if let Node::Symbol(name) = items[0].drop_meta() {
												if self.ctx.user_functions.contains_key(name) {
													return false;
												}
											}
										}
									}
								}
								// Pattern: def/fun/fn name(params...): body or {body}
								// Also filter out use/import statements (FFI side effects only)
								Node::List(list_items, _, _) => {
									if list_items.len() >= 2 {
										if let Node::Symbol(s) = list_items[0].drop_meta() {
											if is_function_keyword(s) || s == "use" || s == "import" {
												return false;
											}
										}
									}
								}
								_ => {}
							}
							true
						})
						.collect();

					for (i, item) in non_func_items.iter().enumerate() {
						self.emit_node_instructions(func, item);
						// Drop intermediate results, keep last
						if i < non_func_items.len() - 1 {
							func.instruction(&Instruction::Drop);
						}
					}
				} else {
					// Check for pure numeric expressions
					let has_arithmetic = items.iter().any(|item| {
						let item = item.drop_meta();
						matches!(item, Node::Key(_, op, _) if op.is_arithmetic())
					});
					if has_arithmetic {
						if self.get_type(node).is_float() {
							self.emit_float_value(func, node);
							self.emit_call(func, "new_float");
						} else {
							self.emit_numeric_value(func, node);
							self.emit_call(func, "new_int");
						}
					} else {
						// Build linked list: (first, rest, bracket_info)
						self.emit_list_structure(func, items, bracket);
					}
				}
			}
			Node::Data(dada) => {
				self.emit_string_call(func, &dada.type_name, "new_symbol");
			}
			Node::Meta { .. } => {
				self.emit_call(func, "new_empty");
			}
			Node::Error(inner) => {
				// Emit the inner node, but mark as error in kind
				// For now, just emit the inner
				self.emit_node_instructions(func, inner);
			}
			Node::Type { name, body } => {
				// Emit type as a tagged block with name and fields
				self.emit_node_instructions(func, name);
				self.emit_node_instructions(func, body);
				self.emit_call(func, "new_type");
			}
			&Node::False => {
				func.instruction(&Instruction::I64Const(0));
				self.emit_call(func, "new_int");
			}
			&Node::True => {
				func.instruction(&Instruction::I64Const(1));
				self.emit_call(func, "new_int");
			}
		}
	}

	/// Emit arithmetic operation: evaluate operands and apply operator
	fn emit_arithmetic(&mut self, func: &mut Function, left: &Node, op: &Op, right: &Node) {
		// Determine if we need float operations (type upgrading)
		// Division always uses float to preserve precision: 1/2 = 0.5, not 0
		let use_float = self.get_type(left).is_float() || self.get_type(right).is_float() || *op == Op::Div;

		// Handle variable definition/assignment specially
		if *op == Op::Define || *op == Op::Assign {
			// Check for string assignment in WASI mode - skip emit (tracked in Local)
			if self.emit_wasi_imports && matches!(right.drop_meta(), Node::Text(_)) {
				// String data stored in Local's data_pointer/data_length, just emit 0
				func.instruction(&Instruction::I64Const(0));
				return;
			}
			// x:=42 or x=42 → emit value, store to local, return value
			if use_float {
				self.emit_float_value(func, right);
			} else {
				self.emit_numeric_value(func, right);
			}
			if let Node::Symbol(name) = left.drop_meta() {
				if let Some(local) = self.scope.lookup(name) {
					func.instruction(&Instruction::LocalTee(local.position));
				} else {
					panic!("Undefined variable: {}", name);
				}
			} else {
				panic!("Expected symbol in definition, got {:?}", left);
			}
		} else if *op == Op::Inc || *op == Op::Dec {
			// i++ → i = i + 1 (returns new value)
			// i-- → i = i - 1 (returns new value)
			if let Node::Symbol(name) = left.drop_meta() {
				let local_pos = self.scope
					.lookup(name)
					.map(|l| l.position)
					.unwrap_or_else(|| panic!("Undefined variable: {}", name));
				// Get current value
				func.instruction(&Instruction::LocalGet(local_pos));
				// Add/subtract 1
				func.instruction(&Instruction::I64Const(1));
				if *op == Op::Inc {
					func.instruction(&Instruction::I64Add);
				} else {
					func.instruction(&Instruction::I64Sub);
				}
				// Store and return new value
				func.instruction(&Instruction::LocalTee(local_pos));
			} else {
				panic!("Expected symbol for increment/decrement, got {:?}", left);
			}
		} else if op.is_compound_assign() {
			// x += y → x = x + y
			if let Node::Symbol(name) = left.drop_meta() {
				let local_pos = self.scope
					.lookup(name)
					.map(|l| l.position)
					.unwrap_or_else(|| panic!("Undefined variable: {}", name));
				let base_op = op.base_op();
				// Get current value of x
				func.instruction(&Instruction::LocalGet(local_pos));
				// Emit y
				if use_float {
					self.emit_float_value(func, right);
				} else {
					self.emit_numeric_value(func, right);
				}
				// Apply base operation
				if use_float {
					match base_op {
						Op::Add => func.instruction(&Instruction::F64Add),
						Op::Sub => func.instruction(&Instruction::F64Sub),
						Op::Mul => func.instruction(&Instruction::F64Mul),
						Op::Div => func.instruction(&Instruction::F64Div),
						_ => func.instruction(&Instruction::F64Mul), // fallback
					};
				} else {
					match base_op {
						Op::Add => func.instruction(&Instruction::I64Add),
						Op::Sub => func.instruction(&Instruction::I64Sub),
						Op::Mul => func.instruction(&Instruction::I64Mul),
						Op::Div => func.instruction(&Instruction::I64DivS),
						Op::Mod => func.instruction(&Instruction::I64RemS),
						Op::And => func.instruction(&Instruction::I64And),
						Op::Or => func.instruction(&Instruction::I64Or),
						Op::Xor => func.instruction(&Instruction::I64Xor),
						_ => func.instruction(&Instruction::I64Mul), // fallback
					};
				}
				// Store result and leave on stack
				func.instruction(&Instruction::LocalTee(local_pos));
			} else {
				panic!("Expected symbol in compound assignment, got {:?}", left);
			}
		} else if use_float {
			// Float path: emit operands as f64, use F64 instructions
			// Truthy and/or need special short-circuit handling
			if *op == Op::And {
				// Truthy and: if left is 0, return left; else return right
				self.emit_float_value(func, left);
				func.instruction(&Instruction::F64Const(Ieee64::new(0.0f64.to_bits())));
				func.instruction(&Instruction::F64Eq);
				func.instruction(&Instruction::If(BlockType::Result(ValType::F64)));
				self.emit_float_value(func, left); // return left (0.0)
				func.instruction(&Instruction::Else);
				self.emit_float_value(func, right); // return right
				func.instruction(&Instruction::End);
				self.emit_call(func, "new_float");
				return;
			}
			if *op == Op::Or {
				// Truthy or: if left is non-0, return left; else return right
				self.emit_float_value(func, left);
				func.instruction(&Instruction::F64Const(Ieee64::new(0.0f64.to_bits())));
				func.instruction(&Instruction::F64Ne);
				func.instruction(&Instruction::If(BlockType::Result(ValType::F64)));
				self.emit_float_value(func, left); // return left (truthy)
				func.instruction(&Instruction::Else);
				self.emit_float_value(func, right); // return right
				func.instruction(&Instruction::End);
				self.emit_call(func, "new_float");
				return;
			}

			self.emit_float_value(func, left);
			self.emit_float_value(func, right);

			match op {
				Op::Add => {
					func.instruction(&Instruction::F64Add);
				}
				Op::Sub => {
					func.instruction(&Instruction::F64Sub);
				}
				Op::Mul => {
					func.instruction(&Instruction::F64Mul);
				}
				Op::Div => {
					func.instruction(&Instruction::F64Div);
				}
				Op::Mod => {
					// WASM doesn't have F64Rem. Use integer modulo path instead.
					// Drop the f64 values and re-emit as i64
					func.instruction(&Instruction::Drop);
					func.instruction(&Instruction::Drop);
					self.emit_numeric_value(func, left);
					self.emit_numeric_value(func, right);
					func.instruction(&Instruction::I64RemS);
					self.emit_call(func, "new_int");
					return;
				}
				Op::Pow => {
					// Drop the f64 values and re-emit as i64 for power
					func.instruction(&Instruction::Drop);
					func.instruction(&Instruction::Drop);
					self.emit_numeric_value(func, left);
					self.emit_numeric_value(func, right);
					self.emit_call(func, "i64_pow");
					self.emit_call(func, "new_int");
					return;
				}
				op if op.is_comparison() => {
					self.emit_float_comparison(func, op);
					self.emit_call(func, "new_int");
					return;
				}
				_ => unreachable!("Unsupported operator in emit_arithmetic: {:?}", op),
			}
		} else {
			// Integer path: emit operands as i64, use I64 instructions
			// Truthy and/or use short-circuit evaluation
			if *op == Op::And {
				// Truthy and: if left is 0, return 0; else return right
				self.emit_numeric_value(func, left);
				func.instruction(&Instruction::I64Eqz);
				func.instruction(&Instruction::If(BlockType::Result(ValType::I64)));
				func.instruction(&Instruction::I64Const(0)); // return 0 (falsy)
				func.instruction(&Instruction::Else);
				self.emit_numeric_value(func, right); // return right
				func.instruction(&Instruction::End);
				self.emit_call(func, "new_int");
				return;
			}
			if *op == Op::Or {
				// Truthy or: if left is non-0, return left; else return right
				self.emit_numeric_value(func, left);
				func.instruction(&Instruction::I64Eqz);
				func.instruction(&Instruction::If(BlockType::Result(ValType::I64)));
				self.emit_numeric_value(func, right); // return right (left was falsy)
				func.instruction(&Instruction::Else);
				self.emit_numeric_value(func, left); // return left (truthy)
				func.instruction(&Instruction::End);
				self.emit_call(func, "new_int");
				return;
			}

			self.emit_numeric_value(func, left);
			self.emit_numeric_value(func, right);

			match op {
				Op::Add => {
					func.instruction(&Instruction::I64Add);
				}
				Op::Sub => {
					func.instruction(&Instruction::I64Sub);
				}
				Op::Mul => {
					func.instruction(&Instruction::I64Mul);
				}
				Op::Div => {
					func.instruction(&Instruction::I64DivS);
				}
				Op::Mod => {
					func.instruction(&Instruction::I64RemS);
				}
				Op::Pow => {
					self.emit_call(func, "i64_pow");
				}
				Op::Xor => {
					func.instruction(&Instruction::I64Xor);
				}
				op if op.is_comparison() => self.emit_comparison(func, op),
				_ => unreachable!("Unsupported operator in emit_arithmetic: {:?}", op),
			}
		}

		// Wrap result appropriately
		if use_float {
			self.emit_call(func, "new_float");
		} else {
			self.emit_call(func, "new_int");
		}
	}

	/// Emit truthy logical operations (and/or) when operands may be non-numeric
	/// Returns a Node reference based on short-circuit evaluation
	fn emit_truthy_logical(&mut self, func: &mut Function, left: &Node, op: &Op, right: &Node) {
		let node_ref = RefType {
			nullable: false,
			heap_type: HeapType::Concrete(self.node_type),
		};

		// For numeric left operand, extract its value for truthiness check
		let left_is_numeric = self.is_numeric(left);

		if left_is_numeric {
			// Emit left as numeric and check truthiness
			if self.get_type(left).is_float() {
				self.emit_float_value(func, left);
				func.instruction(&Instruction::F64Const(Ieee64::new(0.0f64.to_bits())));
				if *op == Op::And {
					// and: if left == 0, return left (falsy); else return right
					func.instruction(&Instruction::F64Eq);
				} else {
					// or: if left != 0, return left (truthy); else return right
					func.instruction(&Instruction::F64Ne);
				}
			} else {
				self.emit_numeric_value(func, left);
				func.instruction(&Instruction::I64Eqz);
				if *op == Op::And {
					// and: if left == 0 (eqz is true), return left
					// eqz returns 1 if zero, 0 if non-zero
				} else {
					// or: if left != 0 (eqz is false), return left
					func.instruction(&Instruction::I32Eqz); // flip the condition
				}
			}

			// If condition is true, return left; else return right
			func.instruction(&Instruction::If(BlockType::Result(Ref(node_ref))));
			self.emit_node_instructions(func, left);
			func.instruction(&Instruction::Else);
			self.emit_node_instructions(func, right);
			func.instruction(&Instruction::End);
		} else {
			// Left is non-numeric - check if it's falsy at compile time
			let left_is_falsy = left.is_falsy();

			if *op == Op::And {
				if left_is_falsy {
					// and with falsy left: return left
					self.emit_node_instructions(func, left);
				} else {
					// and with truthy left: return right
					self.emit_node_instructions(func, right);
				}
			} else {
				// Op::Or
				if left_is_falsy {
					// or with falsy left: return right
					self.emit_node_instructions(func, right);
				} else {
					// or with truthy left: return left
					self.emit_node_instructions(func, left);
				}
			}
		}
	}



	/// Emit a fetch call using host.fetch import
	/// Takes a URL node, calls host.fetch, returns a Text node with the content
	fn emit_fetch_call(&mut self, func: &mut Function, url_node: &Node) {
		// Extract URL string from node tree
		let url = self.extract_url_string(url_node);

		// Store URL in data section
		let (url_ptr, url_len) = self.allocate_string(&url);

		// Call host.fetch(url_ptr, url_len) -> (result_ptr, result_len)
		func.instruction(&I32Const(url_ptr as i32));
		func.instruction(&I32Const(url_len as i32));

		// Get the host_fetch function index
		if let Some(f) = self.ctx.func_registry.get("host_fetch") {
			func.instruction(&Instruction::Call(f.call_index as u32));
		} else {
			// Fallback: emit empty text if host imports not available
			func.instruction(&I32Const(0));
			func.instruction(&I32Const(0));
		}

		// Stack now has (result_ptr: i32, result_len: i32)
		// Call new_text to create a Text node from the result
		self.emit_call(func, "new_text");
	}

	/// Number of string arguments that need (ptr, len) pairs for FFI functions
	/// Detected from function name - functions like strcmp need paired marshalling
	fn string_pair_arg_count(&self, fn_name: &str) -> usize {
		match fn_name {
			"strcmp" | "strncmp" => 2, // (ptr1, len1, ptr2, len2, ...)
			_ => 0,
		}
	}

	/// Emit FFI function arguments according to signature
	/// Handles both standard args and (ptr, len) string pair marshalling
	fn emit_ffi_args(&mut self, func: &mut Function, fn_name: &str, args: &[Node], sig: &crate::ffi::FfiSignature) {
		let string_pair_count = self.string_pair_arg_count(fn_name);
		let mut arg_idx = 0;
		let mut param_idx = 0;

		// Process string arguments that need (ptr, len) pairs
		for _ in 0..string_pair_count {
			if arg_idx < args.len() && self.is_string_arg(&args[arg_idx]) {
				self.emit_string_ptr_len(func, &args[arg_idx]);
				arg_idx += 1;
				param_idx += 2;
			} else if arg_idx < args.len() {
				self.emit_numeric_value(func, &args[arg_idx]);
				func.instruction(&Instruction::I32WrapI64);
				func.instruction(&I32Const(0));
				arg_idx += 1;
				param_idx += 2;
			} else {
				func.instruction(&I32Const(0));
				func.instruction(&I32Const(0));
				param_idx += 2;
			}
		}

		// Process remaining arguments according to param types
		while param_idx < sig.params.len() {
			if arg_idx < args.len() {
				self.emit_ffi_arg(func, &args[arg_idx], &sig.params[param_idx]);
				arg_idx += 1;
			} else {
				self.emit_ffi_default(func, &sig.params[param_idx]);
			}
			param_idx += 1;
		}
	}

	/// Emit a single FFI argument with appropriate type conversion
	fn emit_ffi_arg(&mut self, func: &mut Function, arg: &Node, param_type: &ValType) {
		match param_type {
			ValType::F64 => self.emit_float_value(func, arg),
			ValType::F32 => {
				self.emit_float_value(func, arg);
				func.instruction(&Instruction::F32DemoteF64);
			}
			ValType::I64 => self.emit_numeric_value(func, arg),
			ValType::I32 => {
				if self.is_string_arg(arg) {
					self.emit_string_ptr_only(func, arg);
				} else {
					self.emit_numeric_value(func, arg);
					func.instruction(&Instruction::I32WrapI64);
				}
			}
			_ => self.emit_numeric_value(func, arg),
		}
	}

	/// Emit default value for missing FFI argument
	fn emit_ffi_default(&mut self, func: &mut Function, param_type: &ValType) {
		match param_type {
			ValType::F64 => func.instruction(&Instruction::F64Const(Ieee64::new(0.0f64.to_bits()))),
			ValType::F32 => func.instruction(&Instruction::F32Const(Ieee32::new(0.0f32.to_bits()))),
			ValType::I64 => func.instruction(&Instruction::I64Const(0)),
			_ => func.instruction(&I32Const(0)),
		};
	}

	/// Emit FFI result conversion based on context
	/// None = wrap in Node, Some(Kind::Int) = raw i64, Some(Kind::Float) = raw f64
	fn emit_ffi_result(&mut self, func: &mut Function, sig: &crate::ffi::FfiSignature, ctx: Option<Kind>) {
		let result_type = sig.results.first().copied();
		match ctx {
			None => match result_type {
				None => self.emit_call(func, "new_empty"),
				Some(ValType::F64) => self.emit_call(func, "new_float"),
				Some(ValType::F32) => {
					func.instruction(&Instruction::F64PromoteF32);
					self.emit_call(func, "new_float");
				}
				Some(ValType::I64) => self.emit_call(func, "new_int"),
				Some(ValType::I32) => {
					func.instruction(&Instruction::I64ExtendI32S);
					self.emit_call(func, "new_int");
				}
				_ => self.emit_call(func, "new_int"),
			},
			Some(Kind::Float) => match result_type {
				None => { func.instruction(&Instruction::F64Const(Ieee64::new(0.0f64.to_bits()))); }
				Some(ValType::F32) => { func.instruction(&Instruction::F64PromoteF32); }
				Some(ValType::I64) => { func.instruction(&Instruction::F64ConvertI64S); }
				Some(ValType::I32) => {
					func.instruction(&Instruction::I64ExtendI32S);
					func.instruction(&Instruction::F64ConvertI64S);
				}
				_ => {} // F64 already correct
			},
			Some(_) => match result_type { // Int or other → i64
				None => { func.instruction(&Instruction::I64Const(0)); }
				Some(ValType::F64) => { func.instruction(&Instruction::I64TruncF64S); }
				Some(ValType::F32) => {
					func.instruction(&Instruction::F64PromoteF32);
					func.instruction(&Instruction::I64TruncF64S);
				}
				Some(ValType::I32) => { func.instruction(&Instruction::I64ExtendI32S); }
				_ => {} // I64 already correct
			},
		}
	}

	/// Emit FFI function call with automatic result handling based on context
	fn emit_ffi_call(&mut self, func: &mut Function, fn_name: &str, args: &[Node], ctx: Option<Kind>) {
		let sig = match self.ctx.ffi_imports.get(fn_name) {
			Some(s) => s.clone(),
			None => return,
		};
		self.emit_ffi_args(func, fn_name, args, &sig);
		if let Some(idx) = self.ffi_func_index(fn_name) {
			func.instruction(&Instruction::Call(idx));
		}
		self.emit_ffi_result(func, &sig, ctx);
	}

	/// Check if a node argument should be treated as a string
	fn is_string_arg(&self, node: &Node) -> bool {
		match node.drop_meta() {
			Node::Text(_) => true,
			Node::Symbol(name) => {
				if let Some(local) = self.scope.lookup(name) {
					local.data_pointer > 0 // Has string data stored
				} else {
					false
				}
			}
			_ => false,
		}
	}

	/// Emit string pointer and length for FFI calls
	fn emit_string_ptr_len(&mut self, func: &mut Function, node: &Node) {
		match node.drop_meta() {
			Node::Text(s) => {
				let (ptr, len) = self.allocate_string(s);
				func.instruction(&I32Const(ptr as i32));
				func.instruction(&I32Const(len as i32));
			}
			Node::Symbol(name) => {
				if let Some(local) = self.scope.lookup(name) {
					if local.data_pointer > 0 {
						func.instruction(&I32Const(local.data_pointer as i32));
						func.instruction(&I32Const(local.data_length as i32));
					} else {
						// Fallback: use symbol name
						let (ptr, len) = self.allocate_string(name);
						func.instruction(&I32Const(ptr as i32));
						func.instruction(&I32Const(len as i32));
					}
				} else {
					// Unknown symbol - use name as string
					let (ptr, len) = self.allocate_string(name);
					func.instruction(&I32Const(ptr as i32));
					func.instruction(&I32Const(len as i32));
				}
			}
			_ => {
				// For other nodes, try to get a string representation
				let s = node.to_string();
				let (ptr, len) = self.allocate_string(&s);
				func.instruction(&I32Const(ptr as i32));
				func.instruction(&I32Const(len as i32));
			}
		}
	}

	/// Emit only string pointer for C-style FFI calls (null-terminated strings)
	fn emit_string_ptr_only(&mut self, func: &mut Function, node: &Node) {
		match node.drop_meta() {
			Node::Text(s) => {
				// Add null terminator for C string
				let c_str = format!("{}\0", s);
				let (ptr, _) = self.allocate_string(&c_str);
				func.instruction(&I32Const(ptr as i32));
			}
			Node::Symbol(name) => {
				if let Some(local) = self.scope.lookup(name) {
					if local.data_pointer > 0 {
						func.instruction(&I32Const(local.data_pointer as i32));
					} else {
						// Fallback: use symbol name with null terminator
						let c_str = format!("{}\0", name);
						let (ptr, _) = self.allocate_string(&c_str);
						func.instruction(&I32Const(ptr as i32));
					}
				} else {
					// Unknown symbol - use name as string
					let c_str = format!("{}\0", name);
					let (ptr, _) = self.allocate_string(&c_str);
					func.instruction(&I32Const(ptr as i32));
				}
			}
			_ => {
				// For other nodes, try to get a string representation
				let s = node.to_string();
				let c_str = format!("{}\0", s);
				let (ptr, _) = self.allocate_string(&c_str);
				func.instruction(&I32Const(ptr as i32));
			}
		}
	}

	/// Emit WASI puts: write string to stdout
	/// Memory layout: [0-3]: buf_ptr, [4-7]: buf_len, [8-11]: nwritten
	fn emit_wasi_puts(&mut self, func: &mut Function, arg: &Node) {
		// Get string data pointer and length
		let (str_ptr, str_len) = match arg.drop_meta() {
			Node::Text(s) => self.allocate_string(s),
			Node::Symbol(var_name) => {
				// Check if this is a string variable with stored data
				if let Some(local) = self.scope.lookup(var_name) {
					if local.data_pointer > 0 {
						(local.data_pointer, local.data_length)
					} else {
						// Fallback: use the symbol name itself
						self.allocate_string(var_name)
					}
				} else {
					self.allocate_string(var_name)
				}
			}
			_ => self.allocate_string(""),
		};

		// Set up iovec at address 0: {buf_ptr: i32, buf_len: i32}
		// i32.store at address 0 = str_ptr
		func.instruction(&I32Const(0)); // address
		func.instruction(&I32Const(str_ptr as i32)); // value
		func.instruction(&Instruction::I32Store(MemArg {
			offset: 0,
			align: 2,
			memory_index: 0,
		}));

		// i32.store at address 4 = str_len
		func.instruction(&I32Const(4)); // address
		func.instruction(&I32Const(str_len as i32)); // value
		func.instruction(&Instruction::I32Store(MemArg {
			offset: 0,
			align: 2,
			memory_index: 0,
		}));

		// Call fd_write(fd=1, iovs=0, iovs_len=1, nwritten=8)
		func.instruction(&I32Const(1)); // fd = stdout
		func.instruction(&I32Const(0)); // iovs ptr
		func.instruction(&I32Const(1)); // iovs len
		func.instruction(&I32Const(8)); // nwritten ptr

		if let Some(f) = self.ctx.func_registry.get("wasi_fd_write") {
			func.instruction(&Instruction::Call(f.call_index as u32));
		}
		// Stack now has i32 (error code), leave it for conversion to Node
	}

	/// Emit WASI puti: write integer to stdout
	/// Converts integer to string and writes via fd_write
	fn emit_wasi_puti(&mut self, func: &mut Function, arg: &Node) {
		// For compile-time constants, we can pre-compute the string
		if let Node::Number(n) = arg.drop_meta() {
			let s = format!("{}", n); // Number implements Display
			let (str_ptr, str_len) = self.allocate_string(&s);

			// Set up iovec
			func.instruction(&I32Const(0));
			func.instruction(&I32Const(str_ptr as i32));
			func.instruction(&Instruction::I32Store(MemArg {
				offset: 0,
				align: 2,
				memory_index: 0,
			}));
			func.instruction(&I32Const(4));
			func.instruction(&I32Const(str_len as i32));
			func.instruction(&Instruction::I32Store(MemArg {
				offset: 0,
				align: 2,
				memory_index: 0,
			}));

			// Call fd_write
			func.instruction(&I32Const(1));
			func.instruction(&I32Const(0));
			func.instruction(&I32Const(1));
			func.instruction(&I32Const(8));

			if let Some(f) = self.ctx.func_registry.get("wasi_fd_write") {
				func.instruction(&Instruction::Call(f.call_index as u32));
				func.instruction(&Instruction::Drop); // Drop return value
			}
		}
		// For runtime values, we'd need itoa - just drop for now
	}

	/// Emit WASI putf: write float to stdout
	fn emit_wasi_putf(&mut self, func: &mut Function, arg: &Node) {
		// For compile-time constants, pre-compute the string
		if let Node::Number(n) = arg.drop_meta() {
			let s = format!("{}", n); // Number implements Display
			let (str_ptr, str_len) = self.allocate_string(&s);

			func.instruction(&I32Const(0));
			func.instruction(&I32Const(str_ptr as i32));
			func.instruction(&Instruction::I32Store(MemArg {
				offset: 0,
				align: 2,
				memory_index: 0,
			}));
			func.instruction(&I32Const(4));
			func.instruction(&I32Const(str_len as i32));
			func.instruction(&Instruction::I32Store(MemArg {
				offset: 0,
				align: 2,
				memory_index: 0,
			}));

			func.instruction(&I32Const(1));
			func.instruction(&I32Const(0));
			func.instruction(&I32Const(1));
			func.instruction(&I32Const(8));

			if let Some(f) = self.ctx.func_registry.get("wasi_fd_write") {
				func.instruction(&Instruction::Call(f.call_index as u32));
			}
		} else {
			// Return 0 for non-constant
			func.instruction(&I32Const(0));
		}
	}

	/// Emit fd_write call - auto-detects string arguments and sets up iovec
	fn emit_wasi_fd_write_call(&mut self, func: &mut Function, args: &[Node]) {
		// fd_write(fd, iovs_ptr, iovs_len, nwritten_ptr) -> i32
		// Check if second argument is a string (variable or literal)
		// If so, use the puts mechanism to set up iovec automatically
		let second_arg = args.get(1).map(|n| n.drop_meta());
		let is_string_arg = match &second_arg {
			Some(Node::Text(_)) => true,
			Some(Node::Symbol(s)) => {
				// Check if symbol refers to a string variable
				if let Some(local) = self.scope.lookup(s) {
					local.kind == Kind::Text
				} else {
					false
				}
			}
			_ => false,
		};

		if is_string_arg && args.len() >= 4 {
			// Use puts mechanism: set up iovec from string
			self.emit_wasi_puts(func, &args[1]);
			// fd_write already called by emit_wasi_puts, just extend to i64
			func.instruction(&Instruction::I64ExtendI32S);
		} else {
			// Raw numeric mode
			for arg in args.iter().take(4) {
				self.emit_numeric_value(func, arg);
				func.instruction(&Instruction::I32WrapI64);
			}
			if let Some(f) = self.ctx.func_registry.get("wasi_fd_write") {
				func.instruction(&Instruction::Call(f.call_index as u32));
				func.instruction(&Instruction::I64ExtendI32S);
			}
		}
	}

	/// Extract URL string from parsed node tree
	/// Handles patterns like: https://... which parses as Key(Symbol("https"), Colon, ...)
	fn extract_url_string(&self, node: &Node) -> String {
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
	fn emit_global_declaration(&mut self, func: &mut Function, decl: &Node) {
		// decl should be Key(name, Define/Assign, value)
		let (name, value) = match decl.drop_meta() {
			Node::Key(left, Op::Define | Op::Assign, right) => {
				if let Node::Symbol(n) = left.drop_meta() {
					(n.clone(), right.clone())
				} else {
					panic!("Expected symbol in global declaration, got {:?}", left);
				}
			}
			_ => panic!("Expected assignment in global declaration, got {:?}", decl),
		};

		let kind = self.get_type(&value);

		// Check if global already exists (reassignment)
		if let Some(&(global_idx, existing_kind)) = self.ctx.user_globals.get(&name) {
			if existing_kind.is_float() {
				self.emit_float_value(func, &value);
				func.instruction(&Instruction::GlobalSet(global_idx));
				func.instruction(&Instruction::GlobalGet(global_idx));
				self.emit_call(func, "new_float");
			} else {
				self.emit_numeric_value(func, &value);
				func.instruction(&Instruction::GlobalSet(global_idx));
				func.instruction(&Instruction::GlobalGet(global_idx));
				self.emit_call(func, "new_int");
			}
			return;
		}

		let wasm_val_type = if kind.is_float() { ValType::F64 } else { ValType::I64 };

		// Create mutable global with default initial value
		let init_expr = if kind.is_float() {
			ConstExpr::f64_const(Ieee64::new(0.0f64.to_bits()))
		} else {
			ConstExpr::i64_const(0)
		};

		self.globals.global(
			GlobalType {
				val_type: wasm_val_type,
				mutable: true,
				shared: false,
			},
			&init_expr,
		);

		let global_idx = self.next_global_idx;
		self.next_global_idx += 1;
		self.ctx.user_globals.insert(name.clone(), (global_idx, kind));

		// Emit value computation and store to global
		if kind.is_float() {
			self.emit_float_value(func, &value);
			func.instruction(&Instruction::GlobalSet(global_idx));
			func.instruction(&Instruction::GlobalGet(global_idx));
			self.emit_call(func, "new_float");
		} else {
			self.emit_numeric_value(func, &value);
			func.instruction(&Instruction::GlobalSet(global_idx));
			func.instruction(&Instruction::GlobalGet(global_idx));
			self.emit_call(func, "new_int");
		}
	}

	/// Emit global declaration and return numeric value (for use in emit_numeric_value)
	fn emit_global_numeric(&mut self, func: &mut Function, decl: &Node) {
		let (name, value) = match decl.drop_meta() {
			Node::Key(left, Op::Define | Op::Assign, right) => {
				if let Node::Symbol(n) = left.drop_meta() {
					(n.clone(), right.clone())
				} else {
					panic!("Expected symbol in global declaration, got {:?}", left);
				}
			}
			_ => panic!("Expected assignment in global declaration, got {:?}", decl),
		};

		let kind = self.get_type(&value);

		// Check if global already exists (reassignment)
		if let Some(&(global_idx, existing_kind)) = self.ctx.user_globals.get(&name) {
			if existing_kind.is_float() {
				self.emit_float_value(func, &value);
				func.instruction(&Instruction::GlobalSet(global_idx));
				func.instruction(&Instruction::GlobalGet(global_idx));
				func.instruction(&Instruction::I64TruncF64S);
			} else {
				self.emit_numeric_value(func, &value);
				func.instruction(&Instruction::GlobalSet(global_idx));
				func.instruction(&Instruction::GlobalGet(global_idx));
			}
			return;
		}

		let wasm_val_type = if kind.is_float() { ValType::F64 } else { ValType::I64 };

		let init_expr = if kind.is_float() {
			ConstExpr::f64_const(Ieee64::new(0.0f64.to_bits()))
		} else {
			ConstExpr::i64_const(0)
		};

		self.globals.global(
			GlobalType {
				val_type: wasm_val_type,
				mutable: true,
				shared: false,
			},
			&init_expr,
		);

		let global_idx = self.next_global_idx;
		self.next_global_idx += 1;
		self.ctx.user_globals.insert(name.clone(), (global_idx, kind));

		// Emit value, store to global, and return value on stack
		if kind.is_float() {
			self.emit_float_value(func, &value);
			func.instruction(&Instruction::GlobalSet(global_idx));
			func.instruction(&Instruction::GlobalGet(global_idx));
			// Convert to i64 for emit_numeric_value context
			func.instruction(&Instruction::I64TruncF64S);
		} else {
			self.emit_numeric_value(func, &value);
			func.instruction(&Instruction::GlobalSet(global_idx));
			func.instruction(&Instruction::GlobalGet(global_idx));
		}
	}

	/// Emit ternary expression: condition ? then_expr : else_expr
	/// Returns a Node reference, handling mixed-type branches (numbers, strings, etc.)
	fn emit_ternary(&mut self, func: &mut Function, condition: &Node, then_else: &Node) {
		// Structure: condition ? Key(then, Colon, else)
		let (then_expr, else_expr) = match then_else.drop_meta() {
			Node::Key(then_node, Op::Colon, else_node) => (then_node, else_node),
			_ => panic!("Ternary operator expects then:else structure, got {:?}", then_else),
		};

		// Evaluate condition and convert to i32 for if instruction
		self.emit_numeric_value(func, condition);
		func.instruction(&Instruction::I32WrapI64);

		// if (condition) { then_expr } else { else_expr }
		func.instruction(&Instruction::If(BlockType::Result(Ref(self.node_ref(false)))));

		// Then branch - use emit_node_instructions to handle any type (Text, Number, etc.)
		self.emit_node_instructions(func, then_expr);

		func.instruction(&Instruction::Else);

		// Else branch - use emit_node_instructions to handle any type
		self.emit_node_instructions(func, else_expr);

		func.instruction(&Instruction::End);
	}

	/// Emit ternary expression returning i64: condition ? then_expr : else_expr
	fn emit_ternary_numeric(&mut self, func: &mut Function, condition: &Node, then_else: &Node) {
		// Structure: condition ? Key(then, Colon, else)
		let (then_expr, else_expr) = match then_else.drop_meta() {
			Node::Key(then_node, Op::Colon, else_node) => (then_node, else_node),
			_ => panic!("Ternary operator expects then:else structure, got {:?}", then_else),
		};

		// Evaluate condition and convert to i32 for if instruction
		self.emit_numeric_value(func, condition);
		func.instruction(&Instruction::I32WrapI64);

		// if (condition) { then_expr } else { else_expr }
		func.instruction(&Instruction::If(BlockType::Result(ValType::I64)));

		// Then branch
		self.emit_numeric_value(func, then_expr);

		func.instruction(&Instruction::Else);

		// Else branch
		self.emit_numeric_value(func, else_expr);

		func.instruction(&Instruction::End);
	}

	/// Emit if-then-else returning i64: if condition then then_expr else else_expr
	fn emit_if_then_else_numeric(&mut self, func: &mut Function, left: &Node, else_expr: Option<&Node>) {
		// Extract condition and then_expr from structure
		// Structure: Key(Key(Empty, If, condition), Then, then_expr)
		let (condition, then_expr) = match left.drop_meta() {
			Node::Key(if_condition, Op::Then, then_node) => {
				let cond = match if_condition.drop_meta() {
					Node::Key(_, Op::If, c) => c,
					other => panic!("Expected if condition, got {:?}", other),
				};
				(cond, then_node)
			}
			other => panic!("Expected if-then structure, got {:?}", other),
		};

		// Evaluate condition
		self.emit_numeric_value(func, condition);
		func.instruction(&Instruction::I32WrapI64);

		// if (condition) { then_expr } else { else_expr }
		func.instruction(&Instruction::If(BlockType::Result(ValType::I64)));

		// Then branch
		self.emit_numeric_value(func, then_expr);

		func.instruction(&Instruction::Else);

		// Else branch
		if let Some(else_node) = else_expr {
			self.emit_numeric_value(func, else_node);
		} else {
			// No else branch - return 0
			func.instruction(&Instruction::I64Const(0));
		}

		func.instruction(&Instruction::End);
	}

	/// Emit if-then-else expression: if condition then then_expr [else else_expr]
	/// Structure: Key(Key(Key(Empty, If, condition), Then, then_expr), Else, else_expr)
	/// Or for if-then without else: Key(Key(Empty, If, condition), Then, then_expr)
	fn emit_if_then_else(&mut self, func: &mut Function, left: &Node, else_expr: Option<&Node>) {
		// Extract condition and then_expr from structure
		// Structure: Key(Key(Empty, If, condition), Then, then_expr)
		let (condition, then_expr) = match left.drop_meta() {
			Node::Key(if_condition, Op::Then, then_node) => {
				let cond = match if_condition.drop_meta() {
					Node::Key(_, Op::If, c) => c,
					other => panic!("Expected if condition, got {:?}", other),
				};
				(cond, then_node)
			}
			other => panic!("Expected if-then structure, got {:?}", other),
		};

		// Evaluate condition and convert to i32 for if instruction
		self.emit_block_value(func, condition);
		func.instruction(&Instruction::I32WrapI64);

		// if (condition) { then_expr } else { else_expr }
		func.instruction(&Instruction::If(BlockType::Result(Ref(self.node_ref(false)))));

		// Then branch - extract value from block if needed
		self.emit_block_value(func, then_expr);
		self.emit_call(func, "new_int");

		func.instruction(&Instruction::Else);

		// Else branch - use provided else_expr or default to 0
		if let Some(else_node) = else_expr {
			self.emit_block_value(func, else_node);
		} else {
			func.instruction(&Instruction::I64Const(0));
		}
		self.emit_call(func, "new_int");

		func.instruction(&Instruction::End);
	}

	/// Emit while loop: (while condition) do body
	/// If wrap_result is true, wraps result in Node; otherwise returns raw i64
	fn emit_while_loop_impl(&mut self, func: &mut Function, left: &Node, body: &Node, wrap_result: bool) {
		let condition = match left.drop_meta() {
			Node::Key(_, Op::While, cond) => cond,
			other => panic!("Expected while condition, got {:?}", other),
		};

		let result_local = self.next_temp_local;
		self.next_temp_local += 1;

		func.instruction(&Instruction::I64Const(0));
		func.instruction(&Instruction::LocalSet(result_local));

		func.instruction(&Instruction::Block(BlockType::Empty));
		func.instruction(&Instruction::Loop(BlockType::Empty));

		self.emit_block_value(func, condition);
		func.instruction(&Instruction::I32WrapI64);
		func.instruction(&Instruction::I32Eqz);
		func.instruction(&Instruction::BrIf(1));

		self.emit_block_value(func, body);
		func.instruction(&Instruction::LocalSet(result_local));
		func.instruction(&Instruction::Br(0));

		func.instruction(&Instruction::End);
		func.instruction(&Instruction::End);

		func.instruction(&Instruction::LocalGet(result_local));
		if wrap_result {
			self.emit_call(func, "new_int");
		}
	}

	fn emit_while_loop(&mut self, func: &mut Function, left: &Node, body: &Node) {
		self.emit_while_loop_impl(func, left, body, true);
	}

	fn emit_while_loop_value(&mut self, func: &mut Function, left: &Node, body: &Node) {
		self.emit_while_loop_impl(func, left, body, false);
	}

	/// Emit type cast: value as type
	/// Handles conversions between int, float, string
	/// Optimizes literal conversions at compile time
	fn emit_cast(&mut self, func: &mut Function, value: &Node, target_type: &Node) {
		let type_name = match target_type.drop_meta() {
			Node::Symbol(s) => s.to_lowercase(),
			Node::Text(s) => s.to_lowercase(),
			_ => {
				// Unknown type, emit as-is
				self.emit_node_instructions(func, value);
				return;
			}
		};

		let value = value.drop_meta();

		match type_name.as_str() {
			"int" | "integer" | "i32" | "i64" | "long" => {
				// Cast to integer
				match value {
					// Compile-time: float literal to int
					Node::Number(Number::Float(f)) => {
						func.instruction(&Instruction::I64Const(*f as i64));
						self.emit_call(func, "new_int");
					}
					// Compile-time: string literal to int (parse, truncate if float)
					Node::Text(s) => {
						let n: i64 = s
							.parse::<i64>()
							.unwrap_or_else(|_| s.parse::<f64>().map(|f| f as i64).unwrap_or(0));
						func.instruction(&Instruction::I64Const(n));
						self.emit_call(func, "new_int");
					}
					// Compile-time: char literal to int (parse digit)
					Node::Char(c) => {
						let n: i64 = c.to_string().parse().unwrap_or(*c as i64);
						func.instruction(&Instruction::I64Const(n));
						self.emit_call(func, "new_int");
					}
					// Runtime: float expression to int
					_ if self.get_type(value).is_float() => {
						self.emit_float_value(func, value);
						func.instruction(&Instruction::I64TruncF64S);
						self.emit_call(func, "new_int");
					}
					// Already int or coercible
					_ => {
						self.emit_numeric_value(func, value);
						self.emit_call(func, "new_int");
					}
				}
			}
			"float" | "real" | "double" | "f32" | "f64" => {
				// Cast to float
				match value {
					// Compile-time: string literal to float
					Node::Text(s) => {
						let f: f64 = s.parse().unwrap_or(0.0);
						func.instruction(&Instruction::F64Const(f.into()));
						self.emit_call(func, "new_float");
					}
					// Compile-time: char literal to float (parse digit)
					Node::Char(c) => {
						let f: f64 = c.to_string().parse().unwrap_or(*c as i64 as f64);
						func.instruction(&Instruction::F64Const(f.into()));
						self.emit_call(func, "new_float");
					}
					// Compile-time: int literal to float
					Node::Number(Number::Int(n)) => {
						func.instruction(&Instruction::F64Const((*n as f64).into()));
						self.emit_call(func, "new_float");
					}
					// Runtime: emit as float
					_ => {
						self.emit_float_value(func, value);
						self.emit_call(func, "new_float");
					}
				}
			}
			"string" | "str" | "text" => {
				// Cast to string
				match value {
					// Compile-time: number literal to string
					Node::Number(n) => {
						let s = n.to_string();
						let (ptr, len) = self.allocate_string(&s);
						func.instruction(&I32Const(ptr as i32));
						func.instruction(&I32Const(len as i32));
						self.emit_call(func, "new_text");
					}
					// Compile-time: char to string
					Node::Char(c) => {
						let s = c.to_string();
						let (ptr, len) = self.allocate_string(&s);
						func.instruction(&I32Const(ptr as i32));
						func.instruction(&I32Const(len as i32));
						self.emit_call(func, "new_text");
					}
					// Already a string - use the string table
					Node::Text(s) => {
						self.emit_string_call(func, s, "new_text");
					}
					// Runtime: use cast function
					_ => {
						self.emit_node_instructions(func, value);
						self.emit_call(func, "cast_to_string");
					}
				}
			}
			"char" | "character" => {
				// Cast to char: int to char (digit representation)
				// new_codepoint expects i32
				match value {
					Node::Number(Number::Int(n)) => {
						// Convert digit to char: 2 → '2'
						let c = if *n >= 0 && *n <= 9 {
							char::from_digit(*n as u32, 10).unwrap_or('?')
						} else {
							char::from_u32(*n as u32).unwrap_or('?')
						};
						func.instruction(&I32Const(c as i32));
						self.emit_call(func, "new_codepoint");
					}
					Node::Char(c) => {
						func.instruction(&I32Const(*c as i32));
						self.emit_call(func, "new_codepoint");
					}
					_ => {
						self.emit_numeric_value(func, value);
						func.instruction(&Instruction::I32WrapI64);
						self.emit_call(func, "new_codepoint");
					}
				}
			}
			"bool" | "boolean" => {
				// Cast to bool
				match value {
					// Compile-time: string to bool
					Node::Text(s) => {
						let b = !matches!(
							s.to_lowercase().as_str(),
							"" | "0" | "false" | "no" | "ø" | "nil" | "null" | "none"
						);
						func.instruction(&Instruction::I64Const(if b { 1 } else { 0 }));
						self.emit_call(func, "new_int");
					}
					Node::Char(c) => {
						let b = !matches!(*c, '0' | 'ø');
						func.instruction(&Instruction::I64Const(if b { 1 } else { 0 }));
						self.emit_call(func, "new_int");
					}
					Node::Number(Number::Int(n)) => {
						let b = *n != 0;
						func.instruction(&Instruction::I64Const(if b { 1 } else { 0 }));
						self.emit_call(func, "new_int");
					}
					Node::Number(Number::Float(f)) => {
						let b = *f != 0.0;
						func.instruction(&Instruction::I64Const(if b { 1 } else { 0 }));
						self.emit_call(func, "new_int");
					}
					Node::True => {
						func.instruction(&Instruction::I64Const(1));
						self.emit_call(func, "new_int");
					}
					Node::False => {
						func.instruction(&Instruction::I64Const(0));
						self.emit_call(func, "new_int");
					}
					_ => {
						// Runtime: non-zero/non-empty is truthy
						self.emit_numeric_value(func, value);
						func.instruction(&Instruction::I64Eqz);
						func.instruction(&Instruction::I64ExtendI32U);
						func.instruction(&Instruction::I64Const(1));
						func.instruction(&Instruction::I64Xor);
						self.emit_call(func, "new_int");
					}
				}
			}
			"number" | "num" => {
				// Cast to number: auto-detect int or float
				match value {
					Node::Text(s) => {
						// Try parsing as int first, then float
						if let Ok(n) = s.parse::<i64>() {
							func.instruction(&Instruction::I64Const(n));
							self.emit_call(func, "new_int");
						} else if let Ok(f) = s.parse::<f64>() {
							func.instruction(&Instruction::F64Const(f.into()));
							self.emit_call(func, "new_float");
						} else {
							func.instruction(&Instruction::I64Const(0));
							self.emit_call(func, "new_int");
						}
					}
					Node::Char(c) => {
						if let Some(n) = c.to_digit(10) {
							func.instruction(&Instruction::I64Const(n as i64));
							self.emit_call(func, "new_int");
						} else {
							func.instruction(&Instruction::I64Const(*c as i64));
							self.emit_call(func, "new_int");
						}
					}
					_ => {
						// Already numeric
						self.emit_node_instructions(func, value);
					}
				}
			}
			_ => {
				// Unknown type, emit as key node for dynamic dispatch
				self.emit_node_instructions(func, value);
				self.emit_node_instructions(func, target_type);
				func.instruction(&Instruction::I64Const(op_to_code(&Op::As)));
				self.emit_call(func, "new_key");
			}
		}
	}

	/// Extract numeric value from a block { expr } or plain expr
	fn emit_block_value(&mut self, func: &mut Function, node: &Node) {
		match node.drop_meta() {
			Node::List(items, Bracket::Curly, _) if items.len() == 1 => {
				// Block with single item: { expr } -> extract expr
				self.emit_numeric_value(func, &items[0]);
			}
			_ => self.emit_numeric_value(func, node),
		}
	}

	/// Emit the numeric value of a node onto the stack (as i64)
	fn emit_numeric_value(&mut self, func: &mut Function, node: &Node) {
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
					Number::Int(n) => func.instruction(&Instruction::I64Const(*n)),
					Number::Float(f) => func.instruction(&Instruction::I64Const(*f as i64)),
					Number::Quotient(n, d) => func.instruction(&Instruction::I64Const(n / d)),
					Number::Complex(r, _i) => func.instruction(&Instruction::I64Const(*r as i64)),
					Number::Nan | Number::Inf | Number::NegInf => {
						func.instruction(&Instruction::I64Const(0)) // special values → 0
					}
				};
			}
			Node::True => {
				func.instruction(&Instruction::I64Const(1));
			}
			Node::False => {
				func.instruction(&Instruction::I64Const(0));
			}
			Node::Char(c) => {
				func.instruction(&Instruction::I64Const(*c as i64));
			}
			// Variable definition/assignment: x:=42 or x=42 → store and return value
			Node::Key(left, Op::Define | Op::Assign, right) => {
				// Handle index assignment: node#index = value → node_set_at returns value as i64
				if let Node::Key(node_expr, Op::Hash, index_expr) = left.drop_meta() {
					// Emit node (string or list ref)
					self.emit_node_instructions(func, node_expr);
					// Emit index (as i64)
					self.emit_numeric_value(func, index_expr);
					// Emit value (as i64)
					self.emit_numeric_value(func, right);
					// Call node_set_at which returns the value that was set
					self.emit_call(func, "node_set_at");
					return;
				}
				if let Node::Symbol(name) = left.drop_meta() {
					// Emit value
					self.emit_numeric_value(func, right);
					// Duplicate value on stack (tee = set + get)
					if let Some(local) = self.scope.lookup(name) {
						func.instruction(&Instruction::LocalTee(local.position));
					} else {
						panic!("Undefined variable: {}", name);
					}
				} else {
					panic!("Expected symbol in definition, got {:?}", left);
				}
			}
			// Increment/decrement: i++ or i--
			Node::Key(left, op, _right) if *op == Op::Inc || *op == Op::Dec => {
				if let Node::Symbol(name) = left.drop_meta() {
					let local_pos = self.scope
						.lookup(name)
						.map(|l| l.position)
						.unwrap_or_else(|| panic!("Undefined variable: {}", name));
					// Get current value
					func.instruction(&Instruction::LocalGet(local_pos));
					// Add/subtract 1
					func.instruction(&Instruction::I64Const(1));
					if *op == Op::Inc {
						func.instruction(&Instruction::I64Add);
					} else {
						func.instruction(&Instruction::I64Sub);
					}
					// Store and return new value
					func.instruction(&Instruction::LocalTee(local_pos));
				} else {
					panic!("Expected symbol for increment/decrement, got {:?}", left);
				}
			}
			// Compound assignment: x += y → x = x + y
			Node::Key(left, op, right) if op.is_compound_assign() => {
				if let Node::Symbol(name) = left.drop_meta() {
					// Get local position first to avoid borrow issues
					let local_pos = self.scope
						.lookup(name)
						.map(|l| l.position)
						.unwrap_or_else(|| panic!("Undefined variable: {}", name));
					let base_op = op.base_op();
					// Get current value of x
					func.instruction(&Instruction::LocalGet(local_pos));
					// Emit y
					self.emit_numeric_value(func, right);
					// Apply base operation
					match base_op {
						Op::Add => {
							func.instruction(&Instruction::I64Add);
						}
						Op::Sub => {
							func.instruction(&Instruction::I64Sub);
						}
						Op::Mul => {
							func.instruction(&Instruction::I64Mul);
						}
						Op::Div => {
							func.instruction(&Instruction::I64DivS);
						}
						Op::Mod => {
							func.instruction(&Instruction::I64RemS);
						}
						Op::Pow => {
							self.emit_call(func, "i64_pow");
						}
						Op::And => {
							func.instruction(&Instruction::I64And);
						}
						Op::Or => {
							func.instruction(&Instruction::I64Or);
						}
						Op::Xor => {
							func.instruction(&Instruction::I64Xor);
						}
						_ => panic!("Unexpected base op: {:?}", base_op),
					}
					// Store result and leave on stack
					func.instruction(&Instruction::LocalTee(local_pos));
				} else {
					panic!("Expected symbol in compound assignment, got {:?}", left);
				}
			}
			// Arithmetic operators
			Node::Key(left, op, right) if op.is_arithmetic() => {
				self.emit_numeric_value(func, left);
				self.emit_numeric_value(func, right);
				match op {
					Op::Add => {
						func.instruction(&Instruction::I64Add);
					}
					Op::Sub => {
						func.instruction(&Instruction::I64Sub);
					}
					Op::Mul => {
						func.instruction(&Instruction::I64Mul);
					}
					Op::Div => {
						func.instruction(&Instruction::I64DivS);
					}
					Op::Mod => {
						func.instruction(&Instruction::I64RemS);
					}
					Op::Pow => {
						self.emit_call(func, "i64_pow");
					}
					_ => unreachable!(),
				}
			}
			// Logical operators (and, or) use truthy semantics, xor uses bitwise
			Node::Key(left, Op::And, right) => {
				// Truthy and: if left is 0, return 0; else return right
				self.emit_numeric_value(func, left);
				func.instruction(&Instruction::I64Eqz);
				func.instruction(&Instruction::If(BlockType::Result(ValType::I64)));
				func.instruction(&Instruction::I64Const(0));
				func.instruction(&Instruction::Else);
				self.emit_numeric_value(func, right);
				func.instruction(&Instruction::End);
			}
			Node::Key(left, Op::Or, right) => {
				// Truthy or: if left is non-0, return left; else return right
				self.emit_numeric_value(func, left);
				func.instruction(&Instruction::I64Eqz);
				func.instruction(&Instruction::If(BlockType::Result(ValType::I64)));
				self.emit_numeric_value(func, right);
				func.instruction(&Instruction::Else);
				self.emit_numeric_value(func, left);
				func.instruction(&Instruction::End);
			}
			Node::Key(left, Op::Xor, right) => {
				self.emit_numeric_value(func, left);
				self.emit_numeric_value(func, right);
				func.instruction(&Instruction::I64Xor);
			}
			// Comparison operators
			Node::Key(left, op, right) if op.is_comparison() => {
				self.emit_numeric_value(func, left);
				self.emit_numeric_value(func, right);
				self.emit_comparison(func, op);
			}
			// Prefix operators: √x, -x, !x, ‖x‖, #x (count)
			Node::Key(left, op, right) if op.is_prefix() && matches!(left.drop_meta(), Node::Empty) => {
				match op {
					Op::Sqrt => {
						// √x = sqrt(x), need f64 for sqrt then convert back to i64
						self.emit_float_value(func, right);
						func.instruction(&Instruction::F64Sqrt);
						func.instruction(&Instruction::I64TruncF64S);
					}
					Op::Neg => {
						// -x = 0 - x
						func.instruction(&Instruction::I64Const(0));
						self.emit_numeric_value(func, right);
						func.instruction(&Instruction::I64Sub);
					}
					Op::Not => {
						// !x = x == 0
						self.emit_numeric_value(func, right);
						func.instruction(&Instruction::I64Eqz);
						func.instruction(&Instruction::I64ExtendI32U);
					}
					Op::Abs => {
						// |x| = if x < 0 then -x else x
						let temp = self.next_temp_local;
						self.emit_numeric_value(func, right);
						func.instruction(&Instruction::LocalTee(temp));
						func.instruction(&Instruction::I64Const(0));
						func.instruction(&Instruction::I64LtS);
						func.instruction(&Instruction::If(BlockType::Result(ValType::I64)));
						func.instruction(&Instruction::I64Const(0));
						func.instruction(&Instruction::LocalGet(temp));
						func.instruction(&Instruction::I64Sub);
						func.instruction(&Instruction::Else);
						func.instruction(&Instruction::LocalGet(temp));
						func.instruction(&Instruction::End);
					}
					_ => panic!("Unhandled prefix operator: {:?}", op),
				}
			}
			// Prefix # means count/length: #list returns element count
			Node::Key(left, Op::Hash, right) if matches!(left.drop_meta(), Node::Empty) => {
				self.emit_node_instructions(func, right);
				self.emit_call(func, "node_count");
			}
			// Index operator: list#index (1-based)
			Node::Key(list, Op::Hash, index) => {
				// Emit the list as a Node reference
				self.emit_node_instructions(func, list);
				// Emit the index
				self.emit_numeric_value(func, index);
				// Call list_at
				self.emit_call(func, "list_at");
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
			Node::Symbol(name) => {
				// Handle $n parameter reference (e.g., $0 = first param)
				if let Some(rest) = name.strip_prefix('$') {
					if let Ok(idx) = rest.parse::<u32>() {
						func.instruction(&Instruction::LocalGet(idx));
						return;
					}
				}
				if let Some(local) = self.scope.lookup(name) {
					func.instruction(&Instruction::LocalGet(local.position));
				} else if let Some(&(idx, kind)) = self.ctx.user_globals.get(name) {
					func.instruction(&Instruction::GlobalGet(idx));
					if kind.is_float() {
						// Convert f64 global to i64 for integer operations
						func.instruction(&Instruction::I64TruncF64S);
					}
				} else {
					panic!("Undefined variable: {}", name);
				}
			}
			// Statement sequence or function call
			Node::List(items, bracket, _) if !items.is_empty() => {
				// Check for zero-argument function call: (funcname)
				if items.len() == 1 && *bracket == Bracket::Round {
					if let Node::Symbol(fn_name) = items[0].drop_meta() {
						if self.ctx.user_functions.contains_key(fn_name) {
							self.emit_user_function_call_numeric(func, fn_name, &[]);
							return;
						}
						// Check for zero-arg FFI function call
						if self.ctx.ffi_imports.contains_key(fn_name) {
							self.emit_ffi_call(func, fn_name, &[], Some(Kind::Int));
							return;
						}
					}
				}
				// Check for user function call: [Symbol("funcname"), arg1, arg2, ...]
				if items.len() >= 2 {
					if let Node::Symbol(fn_name) = items[0].drop_meta() {
						if self.ctx.user_functions.contains_key(fn_name) {
							// Check if it's a zero-arg call: (funcname Empty)
							if items.len() == 2 && matches!(items[1].drop_meta(), Node::Empty) {
								self.emit_user_function_call_numeric(func, fn_name, &[]);
							} else {
								self.emit_user_function_call_numeric(func, fn_name, &items[1..]);
							}
							return;
						}
						// Check for FFI function call
						if self.ctx.ffi_imports.contains_key(fn_name) {
							self.emit_ffi_call(func, fn_name, &items[1..], Some(Kind::Int));
							return;
						}
					}
				}
				// Otherwise treat as statement sequence: execute all, return last
				for (i, item) in items.iter().enumerate() {
					self.emit_numeric_value(func, item);
					// Drop all values except the last
					if i < items.len() - 1 {
						func.instruction(&Instruction::Drop);
					}
				}
			}
			// While loop: emit loop and get numeric result
			Node::Key(left, Op::Do, right) => {
				self.emit_while_loop_value(func, left, right);
			}
			other => {
				panic!("Cannot extract numeric value from {:?}", other)
			}
		}
	}

	/// Emit the float value of a node onto the stack (as f64)
	/// Integers are converted to f64 for type upgrading
	fn emit_float_value(&mut self, func: &mut Function, node: &Node) {
		let node = node.drop_meta();
		match node {
			Node::Number(num) => {
				match num {
					Number::Int(n) => {
						// Convert integer to float
						func.instruction(&Instruction::F64Const(Ieee64::new((*n as f64).to_bits())));
					}
					Number::Float(f) => {
						func.instruction(&Instruction::F64Const(Ieee64::new(f.to_bits())));
					}
					Number::Quotient(n, d) => {
						func.instruction(&Instruction::F64Const(Ieee64::new((*n as f64 / *d as f64).to_bits())));
					}
					Number::Complex(r, _i) => {
						func.instruction(&Instruction::F64Const(Ieee64::new(r.to_bits())));
					}
					Number::Nan => {
						func.instruction(&Instruction::F64Const(Ieee64::new(f64::NAN.to_bits())));
					}
					Number::Inf => {
						func.instruction(&Instruction::F64Const(Ieee64::new(f64::INFINITY.to_bits())));
					}
					Number::NegInf => {
						func.instruction(&Instruction::F64Const(Ieee64::new(f64::NEG_INFINITY.to_bits())));
					}
				};
			}
			// Variable definition/assignment: x:=42 or x=42
			Node::Key(left, Op::Define | Op::Assign, right) => {
				if let Node::Symbol(name) = left.drop_meta() {
					self.emit_float_value(func, right);
					if let Some(local) = self.scope.lookup(name) {
						func.instruction(&Instruction::LocalTee(local.position));
					} else {
						panic!("Undefined variable: {}", name);
					}
				} else {
					panic!("Expected symbol in definition, got {:?}", left);
				}
			}
			// Arithmetic operators with float
			Node::Key(left, op, right) if op.is_arithmetic() => {
				self.emit_float_value(func, left);
				self.emit_float_value(func, right);
				match op {
					Op::Add => {
						func.instruction(&Instruction::F64Add);
					}
					Op::Sub => {
						func.instruction(&Instruction::F64Sub);
					}
					Op::Mul => {
						func.instruction(&Instruction::F64Mul);
					}
					Op::Div => {
						func.instruction(&Instruction::F64Div);
					}
					Op::Mod => {
						// WASM doesn't have F64Rem. Drop f64 values and use i64 path.
						func.instruction(&Instruction::Drop);
						func.instruction(&Instruction::Drop);
						self.emit_numeric_value(func, left);
						self.emit_numeric_value(func, right);
						func.instruction(&Instruction::I64RemS);
						func.instruction(&Instruction::F64ConvertI64S);
					}
					Op::Pow => {
						// Drop f64 values and use i64_pow
						func.instruction(&Instruction::Drop);
						func.instruction(&Instruction::Drop);
						self.emit_numeric_value(func, left);
						self.emit_numeric_value(func, right);
						self.emit_call(func, "i64_pow");
						func.instruction(&Instruction::F64ConvertI64S);
					}
					_ => unreachable!(),
				}
			}
			// Suffix operators: x² = x*x, x³ = x*x*x (returns f64)
			Node::Key(left, Op::Square, _) => {
				self.emit_float_value(func, left);
				self.emit_float_value(func, left);
				func.instruction(&Instruction::F64Mul);
			}
			Node::Key(left, Op::Cube, _) => {
				self.emit_float_value(func, left);
				self.emit_float_value(func, left);
				func.instruction(&Instruction::F64Mul);
				self.emit_float_value(func, left);
				func.instruction(&Instruction::F64Mul);
			}
			// Prefix operators: √x = sqrt(x) (returns f64)
			Node::Key(left, Op::Sqrt, right) if matches!(left.drop_meta(), Node::Empty) => {
				self.emit_float_value(func, right);
				func.instruction(&Instruction::F64Sqrt);
			}
			// Prefix negation: -x (returns f64)
			Node::Key(left, Op::Neg, right) if matches!(left.drop_meta(), Node::Empty) => {
				func.instruction(&Instruction::F64Const(0.0.into()));
				self.emit_float_value(func, right);
				func.instruction(&Instruction::F64Sub);
			}
			// Prefix abs: ‖x‖ (returns f64)
			Node::Key(left, Op::Abs, right) if matches!(left.drop_meta(), Node::Empty) => {
				self.emit_float_value(func, right);
				func.instruction(&Instruction::F64Abs);
			}
			// Variable lookup (local or global) - convert i64 to f64 if needed
			Node::Symbol(name) => {
				if let Some(local) = self.scope.lookup(name) {
					func.instruction(&Instruction::LocalGet(local.position));
					if !local.kind.is_float() {
						// Convert i64 local to f64
						func.instruction(&Instruction::F64ConvertI64S);
					}
				} else if let Some(&(idx, kind)) = self.ctx.user_globals.get(name) {
					func.instruction(&Instruction::GlobalGet(idx));
					if !kind.is_float() {
						// Convert i64 global to f64
						func.instruction(&Instruction::F64ConvertI64S);
					}
				} else {
					panic!("Undefined variable: {}", name);
				}
			}
			// Function calls and statement sequences
			Node::List(items, bracket, _) if !items.is_empty() => {
				// Check for function call: [Symbol("funcname"), arg1, arg2, ...]
				if items.len() >= 2 {
					if let Node::Symbol(fn_name) = items[0].drop_meta() {
						// Check for FFI function call
						if self.ctx.ffi_imports.contains_key(fn_name) {
							self.emit_ffi_call(func, fn_name, &items[1..], Some(Kind::Float));
							return;
						}
						// Check for user function call
						if self.ctx.user_functions.contains_key(fn_name) {
							self.emit_user_function_call_numeric(func, fn_name, &items[1..]);
							func.instruction(&Instruction::F64ConvertI64S);
							return;
						}
					}
				}
				// Check for zero-arg function call: (funcname)
				if items.len() == 1 && *bracket == Bracket::Round {
					if let Node::Symbol(fn_name) = items[0].drop_meta() {
						if self.ctx.ffi_imports.contains_key(fn_name) {
							self.emit_ffi_call(func, fn_name, &[], Some(Kind::Float));
							return;
						}
					}
				}
				// Statement sequence: execute all, return last as float
				for (i, item) in items.iter().enumerate() {
					if i < items.len() - 1 {
						// For non-last items, use regular emit and drop
						self.emit_numeric_value(func, item);
						func.instruction(&Instruction::Drop);
					} else {
						// Last item as float
						self.emit_float_value(func, item);
					}
				}
			}
			_ => panic!("Cannot extract float value from {:?}", node),
		}
	}

	/// Emit a range as a list of integers
	/// inclusive: true for .../ (0...3 = [0,1,2,3]), false for .. (0..3 = [0,1,2])
	fn emit_range(&mut self, func: &mut Function, start: &Node, end: &Node, inclusive: bool) {
		let start_val = match start.drop_meta() {
			Node::Number(Number::Int(i)) => *i,
			_ => panic!("Range start must be a constant integer, got {:?}", start),
		};
		let end_val = match end.drop_meta() {
			Node::Number(Number::Int(i)) => *i,
			_ => panic!("Range end must be a constant integer, got {:?}", end),
		};
		let actual_end = if inclusive { end_val + 1 } else { end_val };
		let items: Vec<Node> = (start_val..actual_end).map(|i| Node::Number(Number::Int(i))).collect();
		if items.is_empty() {
			self.emit_call(func, "new_empty");
			return;
		}
		self.emit_list_structure(func, &items, &Bracket::Square);
	}

	/// Emit a list as linked cons cells
	fn emit_list_structure(&mut self, func: &mut Function, items: &[Node], bracket: &Bracket) {
		let bracket_info = match bracket {
			Bracket::Curly => 0i64,
			Bracket::Square => 1,
			Bracket::Round => 2,
			Bracket::Less => 3,
			Bracket::Other(_, _) => 4,
			Bracket::None => 5,
		};

		// Emit first item
		self.emit_node_instructions(func, &items[0]);

		// Emit rest as a proper linked list
		// The value field must always be a list node (or null), never an element directly
		if items.len() > 1 {
			// Recursively build the rest of the list
			// This ensures proper cons-cell structure: (data=first, value=list_node_for_rest)
			self.emit_list_structure(func, &items[1..], bracket);
		} else {
			// Single element list: rest is null
			self.emit_node_null(func);
		}

		// bracket_info
		func.instruction(&Instruction::I64Const(bracket_info));

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

	fn emit_inline_list(&mut self, func: &mut Function, _bracket_info: i64) {
		// Stack: first, rest, bracket_info
		// Need: kind, data(first), value(rest)
		// Use struct.new directly with proper ordering

		// This is complex due to stack order. For now, require new_list function.
		// Pop bracket_info (already on stack as i64)
		// Compute kind
		func.instruction(&Instruction::I64Const(8));
		func.instruction(&Instruction::I64Shl);
		self.emit_kind(func, Kind::List);
		func.instruction(&Instruction::I64Or);
		// But now we have: first, rest, kind - wrong order!
		// We need: kind, first, rest
		// This requires locals or restructuring.

		// For now, panic - caller should ensure new_list is available
		panic!("new_list function required but not available");
	}

	fn validate_wasm(bytes: &[u8]) {
		if let Err(e) = Self::try_validate_wasm(bytes) {
			panic!("WASM validation failed: {}", e);
		}
	}

	fn try_validate_wasm(bytes: &[u8]) -> Result<(), String> {
		let mut features = WasmFeatures::default();
		features.set(WasmFeatures::REFERENCE_TYPES, true);
		features.set(WasmFeatures::GC, true);
		let mut validator = Validator::new_with_features(features);
		match validator.validate_all(bytes) {
			Ok(_) => {
				trace!("✓ WASM validation with GC features passed");
				Ok(())
			}
			Err(e) => Err(format!("{}", e)),
		}
	}

	pub fn finish(mut self) -> Vec<u8> {
		// WASM section order: types, imports, functions, memory, globals, exports, code, data, names
		self.module.section(&self.types);
		if self.ctx.func_registry.import_count() > 0 {
			self.module.section(&self.imports);
		}
		self.module.section(&self.functions);
		self.module.section(&self.memory);
		if self.next_global_idx > 0 {
			self.module.section(&self.globals);
		}
		self.module.section(&self.exports);
		self.module.section(&self.code);
		self.module.section(&self.data);
		self.emit_names();
		self.module.section(&self.names);

		let bytes = self.module.finish();
		std::fs::write("test.wasm", &bytes).expect("Failed to write test.wasm");
		Self::validate_wasm(&bytes);
		bytes
	}

	fn emit_names(&mut self) {
		// Module name
		self.names.module("wasp_compact");

		// Type names
		let mut type_names = NameMap::new();
		type_names.append(self.string_type, "String");
		type_names.append(self.i64_box_type, "i64box");
		type_names.append(self.f64_box_type, "f64box");
		type_names.append(self.node_type, "Node");
		// User-defined type names
		for (name, &idx) in &self.ctx.user_type_indices {
			type_names.append(idx, name);
		}
		self.names.types(&type_names);

		// Field names for struct types
		let mut type_field_names = IndirectNameMap::new();

		// $Node fields
		let mut node_fields = NameMap::new();
		node_fields.append(0, "kind");
		node_fields.append(1, "data");
		node_fields.append(2, "value");
		type_field_names.append(self.node_type, &node_fields);

		// $String fields
		Self::append_string_field_names(&mut type_field_names, self.string_type);

		// $i64box field
		let mut i64box_fields = NameMap::new();
		i64box_fields.append(0, "value");
		type_field_names.append(self.i64_box_type, &i64box_fields);

		// $f64box field
		let mut f64box_fields = NameMap::new();
		f64box_fields.append(0, "value");
		type_field_names.append(self.f64_box_type, &f64box_fields);

		// User-defined type fields
		for type_def in self.ctx.type_registry.types() {
			if let Some(&type_idx) = self.ctx.user_type_indices.get(&type_def.name) {
				let mut field_names = NameMap::new();
				for (i, field) in type_def.fields.iter().enumerate() {
					field_names.append(i as u32, &field.name);
				}
				type_field_names.append(type_idx, &field_names);
			}
		}

		self.names.fields(&type_field_names);

		// Function names - sort by index for deterministic output
		let mut func_names = NameMap::new();
		let mut sorted: Vec<_> = self.ctx
			.func_registry
			.all()
			.iter()
			.map(|f| (f.name.as_str(), f.call_index as u32))
			.collect();
		sorted.sort_by_key(|(_, idx)| *idx);
		for (name, idx) in sorted {
			func_names.append(idx, name);
		}
		self.names.functions(&func_names);

		// Global names for Kind constants
		if self.next_global_idx > 0 {
			let global_names_list = [
				"kind_empty",
				"kind_int",
				"kind_float",
				"kind_text",
				"kind_codepoint",
				"kind_symbol",
				"kind_key",
				"kind_block",
				"kind_list",
				"kind_data",
				"kind_meta",
				"kind_error",
			];
			let mut global_names = NameMap::new();
			for (idx, name) in global_names_list.iter().enumerate() {
				if (idx as u32) < self.next_global_idx {
					global_names.append(idx as u32, name);
				}
			}
			self.names.globals(&global_names);
		}
	}

	pub fn get_unused_functions(&self) -> Vec<String> {
		self.ctx.func_registry
			.all()
			.iter()
			.map(|f| f.name.clone())
			.filter(|name: &String| !self.ctx.used_functions.contains(name.as_str()))
			.collect()
	}

	pub fn get_used_functions(&self) -> Vec<&'static str> {
		self.ctx.used_functions.iter().copied().collect()
	}

	/// Emit a standalone WASM module that returns a raw GC struct instance.
	/// This is the standard path for returning user-defined GC objects.
	///
	/// # Arguments
	/// * `type_def` - The type definition for the struct
	/// * `field_values` - Field values as (name, RawFieldValue) pairs
	///
	/// # Returns
	/// WASM bytes that can be loaded and executed to get the GC struct
	pub fn emit_raw_struct(type_def: &TypeDef, field_values: &[RawFieldValue]) -> Vec<u8> {
		use wasm_encoder::StorageType::Val;
		use wasm_encoder::*;

		let mut module = Module::new();

		// Collect string data and compute offsets
		let mut string_data = Vec::new();
		let mut string_offsets: Vec<(usize, usize)> = Vec::new(); // (offset, len) for each string field
		let mut current_offset = 0usize;

		for value in field_values {
			if let RawFieldValue::String(s) = value {
				string_offsets.push((current_offset, s.len()));
				string_data.extend_from_slice(s.as_bytes());
				current_offset += s.len();
			} else {
				string_offsets.push((0, 0)); // placeholder for non-strings
			}
		}

		// Build types section
		let mut types = TypeSection::new();

		// Type 0: $String = struct { ptr: i32, len: i32 }
		types.ty().struct_(vec![
			FieldType {
				element_type: Val(ValType::I32),
				mutable: false,
			},
			FieldType {
				element_type: Val(ValType::I32),
				mutable: false,
			},
		]);
		let string_type_idx = 0u32;

		// Type 1: User struct type
		let string_ref = RefType {
			nullable: false,
			heap_type: HeapType::Concrete(string_type_idx),
		};
		let struct_fields: Vec<FieldType> = type_def
			.fields
			.iter()
			.map(|f| {
				let element_type = match f.type_name.as_str() {
					"i64" | "Int" | "long" => Val(ValType::I64),
					"i32" | "int" => Val(ValType::I32),
					"f64" | "Float" | "double" => Val(ValType::F64),
					"f32" | "float" => Val(ValType::F32),
					"String" | "Text" | "string" => Val(Ref(string_ref)),
					_ => Val(ValType::I64), // default
				};
				FieldType {
					element_type,
					mutable: false,
				}
			})
			.collect();
		types.ty().struct_(struct_fields);
		let struct_type_idx = 1u32;

		// Type 2: main() -> ref $StructType
		let struct_ref = RefType {
			nullable: false,
			heap_type: HeapType::Concrete(struct_type_idx),
		};
		types.ty().func_type(&FuncType::new([], [Ref(struct_ref)]));

		module.section(&types);

		// Function section
		let mut functions = FunctionSection::new();
		functions.function(2); // main uses type 2
		module.section(&functions);

		// Memory section (only if we have strings)
		let has_strings = !string_data.is_empty();
		if has_strings {
			let mut memories = MemorySection::new();
			memories.memory(MemoryType {
				minimum: 1,
				maximum: None,
				memory64: false,
				shared: false,
				page_size_log2: None,
			});
			module.section(&memories);
		}

		// Export section
		let mut exports = ExportSection::new();
		if has_strings {
			exports.export("memory", ExportKind::Memory, 0);
		}
		exports.export("main", ExportKind::Func, 0);
		module.section(&exports);

		// Code section
		let mut codes = CodeSection::new();
		let mut func = Function::new([]);

		// Emit field values in order
		let mut string_idx = 0usize;
		for value in field_values.iter() {
			match value {
				RawFieldValue::I64(v) => {
					func.instruction(&Instruction::I64Const(*v));
				}
				RawFieldValue::I32(v) => {
					func.instruction(&I32Const(*v));
				}
				RawFieldValue::F64(v) => {
					func.instruction(&Instruction::F64Const(Ieee64::new(v.to_bits())));
				}
				RawFieldValue::F32(v) => {
					func.instruction(&Instruction::F32Const(Ieee32::new(v.to_bits())));
				}
				RawFieldValue::String(_) => {
					let (ptr, len) = string_offsets[string_idx];
					func.instruction(&I32Const(ptr as i32));
					func.instruction(&I32Const(len as i32));
					func.instruction(&Instruction::StructNew(string_type_idx));
					string_idx += 1;
				}
			}
		}
		func.instruction(&Instruction::StructNew(struct_type_idx));
		func.instruction(&Instruction::End);
		codes.function(&func);
		module.section(&codes);

		// Data section for strings
		if has_strings {
			let mut data = DataSection::new();
			data.active(0, &ConstExpr::i32_const(0), string_data.iter().copied());
			module.section(&data);
		}

		// Name section for field name resolution
		let mut names = NameSection::new();

		// Type names
		let mut type_names = NameMap::new();
		type_names.append(string_type_idx, "String");
		type_names.append(struct_type_idx, &type_def.name);
		names.types(&type_names);

		// Field names for structs
		let mut type_field_names = IndirectNameMap::new();

		// String struct fields
		Self::append_string_field_names(&mut type_field_names, string_type_idx);

		// User struct fields
		let mut struct_fields_names = NameMap::new();
		for (i, field) in type_def.fields.iter().enumerate() {
			struct_fields_names.append(i as u32, &field.name);
		}
		type_field_names.append(struct_type_idx, &struct_fields_names);
		names.fields(&type_field_names);

		// Function names
		let mut func_names = NameMap::new();
		func_names.append(0, "main");
		names.functions(&func_names);

		module.section(&names);

		let bytes = module.finish();
		std::fs::write("test.wasm", &bytes).expect("Failed to write test.wasm");
		bytes
	}
}

/// Run raw struct WASM and return GcObject wrapped in Node::Data
pub fn run_raw_struct(wasm_bytes: &[u8]) -> Result<Node, String> {
	use wasmtime::{Linker, Module, Store, Val};

	// Register WASM metadata for field name lookup in Debug output
	let _ = crate::gc_traits::register_gc_types_from_wasm(wasm_bytes);

	let engine = gc_engine();
	let mut store = Store::new(&engine, ());
	let module = Module::new(&engine, wasm_bytes).map_err(|e: wasmtime::Error| e.to_string())?;

	let linker = Linker::new(&engine);
	let instance = linker
		.instantiate(&mut store, &module)
		.map_err(|e: wasmtime::Error| e.to_string())?;

	let main = instance
		.get_func(&mut store, "main")
		.ok_or_else(|| "no main function".to_string())?;

	let mut results = vec![Val::I32(0)];
	main.call(&mut store, &[], &mut results)
		.map_err(|e: wasmtime::Error| e.to_string())?;

	let gc_obj = ErgonomicGcObject::new(results[0], store, Some(instance)).map_err(|e: anyhow::Error| e.to_string())?;

	Ok(crate::node::data(gc_obj))
}

/// Check if code uses fetch (needs host imports)
///  todo get rid of hard-coded logic, see usage
fn uses_fetch(code: &str) -> bool {
	code.contains("fetch ")
}

/// Check if code uses WASI functions (puts, puti, putl, putf, fd_write)
fn uses_wasi(code: &str) -> bool {
	code.contains("puts ")
		|| code.contains("puti ")
		|| code.contains("putl ")
		|| code.contains("putf ")
		|| code.contains("fd_write")
}

/// Check if code uses FFI imports (import X from Y, use m/c)
fn uses_ffi(code: &str) -> bool {
	// Todo: we need to pre-load the libraries in analyzer anyways,
	// and in that process, we can check if the library is FFI or not.
	if code.contains("import ") { return true;}
	if code.contains("use "){ return true; }
	false
}

/// Find a struct instantiation anywhere in the AST using TypeRegistry
/// todo instead of recursing different types individually, we should have one central walker and delegate from there.
fn find_struct_instantiation(registry: &TypeRegistry, node: &Node) -> Option<(TypeDef, Vec<RawFieldValue>)> {
	find_instantiation_recursive(registry, node)
}


fn find_instantiation_recursive(registry: &TypeRegistry, node: &Node) -> Option<(TypeDef, Vec<RawFieldValue>)> {
/// todo instead of recursing different types individually, we should have one central walker and delegate from there.
	use crate::type_kinds::extract_instance_values;
	let node = node.drop_meta();
	match node {
		Node::Key(left, Op::Colon, right) => {
			if let Node::Symbol(name) = left.drop_meta() {
				if let Some(type_def) = registry.get_by_name(name) {
					if let Some((_, values)) = extract_instance_values(node) {
						return Some((type_def.clone(), values));
					}
				}
			}
			// Recurse into right
			find_instantiation_recursive(registry, right)
		}
		Node::List(items, _, _) => {
			for item in items {
				if let Some(result) = find_instantiation_recursive(registry, item) {
					return Some(result);
				}
			}
			None
		}
		_ => None,
	}
}

// Re-export eval function for tests
pub fn eval(code: &str) -> Node {
	use crate::analyzer::collect_all_types;
	use crate::type_kinds::TypeRegistry;
	use crate::wasm_reader::{read_bytes_with_host, read_bytes_with_wasi, read_bytes_with_ffi};

	// Detect file path and load file content
	let code = if !code.contains('\n') && (code.ends_with(".wasp") || code.ends_with(".warp")) {
		if let Ok(content) = std::fs::read_to_string(code) {
			content
		} else {
			code.to_string()
		}
	} else {
		code.to_string()
	};

	let node = WaspParser::parse(&code);

	// Pre-scan: collect all type definitions (supports forward references)
	let mut type_registry = TypeRegistry::new();
	collect_all_types(&mut type_registry, &node);

	// Check for struct instantiation: TypeName:{field:value, ...}
	if let Some((type_def, field_values)) = find_struct_instantiation(&type_registry, &node) {
		let wasm_bytes = WasmGcEmitter::emit_raw_struct(&type_def, &field_values);
		match run_raw_struct(&wasm_bytes) {
			Ok(result) => return result,
			Err(e) => warn!("raw struct eval failed: {}", e),
		}
	}

	// Fallback to standard Node encoding
	let mut emitter = WasmGcEmitter::new();
	let needs_host = uses_fetch(&code); // todo iterate over all used functions and check if they are a host import.
	let needs_wasi = uses_wasi(&code);
	let needs_ffi = uses_ffi(&code);
	if needs_host {
		emitter.set_host_imports(true);
	}
	if needs_wasi {
		emitter.set_wasi_imports(true);
	}
	emitter.emit_for_node(&node);
	let bytes = emitter.finish();

	// Use appropriate linker based on imports needed
	if needs_ffi {
		match read_bytes_with_ffi(&bytes) {
			Ok(result) => return result,
			Err(e) => {
				warn!("FFI eval failed: {}", e);
				return node;
			}
		}
	}

	if needs_wasi {
		match read_bytes_with_wasi(&bytes) {
			Ok(result) => return Node::Number(Number::Int(result)),
			Err(e) => {
				warn!("WASI eval failed: {}", e);
				return node;
			}
		}
	}

	let result = if needs_host {
		read_bytes_with_host(&bytes)
	} else {
		read_bytes(&bytes)
	};

	match result {
		Ok(result) => result,
		Err(e) => {
			warn!("eval failed: {}", e);
			node // Return parsed node on failure
		}
	}
}
