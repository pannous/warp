use crate::analyzer::{collect_variables, infer_type, Scope};
use crate::extensions::numbers::Number;
use crate::function::{Function as FuncDef, FunctionRegistry, Signature};
use crate::gc_traits::GcObject as ErgonomicGcObject;
use crate::node::{Bracket, Node, Op};
use crate::type_kinds::{FieldDef, Kind, TypeDef, TypeRegistry};
use crate::util::gc_engine;
use crate::wasm_gc_reader::read_bytes;
use crate::wasp_parser::WaspParser;
use log::{trace, warn};
use std::collections::{HashMap, HashSet};
use wasm_encoder::*;
use wasmparser::{Validator, WasmFeatures};
use Instruction::I32Const;
use StorageType::Val;
use ValType::Ref;

/// Encode Op as i64 for storage in kind field
fn op_to_code(op: &Op) -> i64 {
	match op {
		Op::None => 0,
		Op::Colon => 1,
		Op::Assign => 2,
		Op::Define => 3,
		Op::Dot => 4,
		_ => 0, // Default to None for other ops
	}
}

/// Decode i64 back to Op
pub fn code_to_op(code: i64) -> Op {
	match code {
		0 => Op::None,
		1 => Op::Colon,
		2 => Op::Assign,
		3 => Op::Define,
		4 => Op::Dot,
		_ => Op::None,
	}
}

/// Helper to create abstract heap type refs
fn any_heap_type() -> HeapType {
	HeapType::Abstract {
		shared: false,
		ty: AbstractHeapType::Any,
	}
}

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
	string_type: u32,  // (struct (field $ptr i32) (field $len i32))
	i64_box_type: u32, // (struct (field i64)) for ints
	f64_box_type: u32, // (struct (field f64)) for floats
	node_type: u32,    // Main 3-field Node struct
	next_type_idx: u32,
	next_func_idx: u32,
	next_global_idx: u32,
	func_registry: FunctionRegistry,
	used_functions: HashSet<&'static str>,
	required_functions: HashSet<&'static str>,
	emit_all_functions: bool,
	emit_kind_globals: bool, // Emit Kind constants as globals for documentation
	emit_host_imports: bool, // Emit host function imports (fetch, run)
	kind_global_indices: HashMap<Kind, u32>, // Kind -> global index
	// String storage in linear memory
	string_table: HashMap<String, u32>,
	next_data_offset: u32,
	// Variable scope
	scope: Scope,
	// Temp local index for while loops etc
	next_temp_local: u32,
	// User-defined type indices
	user_type_indices: HashMap<String, u32>,
	// Type registry for user-defined types parsed from class/struct definitions
	type_registry: TypeRegistry,
	// User-defined global variables: name → (index, kind)
	user_globals: HashMap<String, (u32, Kind)>,
	// User-defined functions: name → (params, body, return_kind)
	user_functions: HashMap<String, UserFunctionDef>,
}

/// User-defined function definition
#[derive(Clone, Debug)]
pub struct UserFunctionDef {
	pub name: String,
	pub params: Vec<String>,     // Parameter names
	pub body: Box<Node>,         // Function body AST
	pub return_kind: Kind,       // Return type
	pub func_index: Option<u32>, // WASM function index when compiled
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
			func_registry: FunctionRegistry::new(),
			used_functions: HashSet::new(),
			required_functions: HashSet::new(),
			emit_all_functions: true,
			emit_kind_globals: true, // Enable by default for debugging
			emit_host_imports: false, // Disabled by default for simpler modules
			kind_global_indices: HashMap::new(),
			string_table: HashMap::new(),
			next_data_offset: 8,
			scope: Scope::new(),
			next_temp_local: 0,
			user_type_indices: HashMap::new(),
			type_registry: TypeRegistry::new(),
			user_globals: HashMap::new(),
			user_functions: HashMap::new(),
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
		self.types.ty().function(
			vec![ValType::I32, ValType::I32],
			vec![ValType::I32, ValType::I32],
		);
		self.next_type_idx += 1;

		// Type for run: (i32, i32) -> i64
		// Takes (wasm_ptr, wasm_len), returns result value
		let run_type_idx = self.next_type_idx;
		self.types.ty().function(
			vec![ValType::I32, ValType::I32],
			vec![ValType::I64],
		);
		self.next_type_idx += 1;

		// Import fetch from "host" module
		self.imports.import("host", "fetch", EntityType::Function(fetch_type_idx));
		self.register_import("host_fetch");

		// Import run from "host" module
		self.imports.import("host", "run", EntityType::Function(run_type_idx));
		self.register_import("host_run");
	}

	/// Register an import function
	fn register_import(&mut self, name: &'static str) -> u32 {
		let func = FuncDef::host(name);
		let idx = self.func_registry.register(func);
		self.next_func_idx = self.func_registry.import_count() + self.func_registry.code_count();
		idx
	}

	/// Register a code function
	fn register_func(&mut self, name: &'static str) -> u32 {
		let func = FuncDef::builtin(name);
		let idx = self.func_registry.register(func);
		self.next_func_idx = self.func_registry.import_count() + self.func_registry.code_count();
		idx
	}

	/// Get function call index by name
	fn func_index(&self, name: &str) -> u32 {
		// First check user functions
		if let Some(user_fn) = self.user_functions.get(name) {
			if let Some(idx) = user_fn.func_index {
				return idx;
			}
		}
		// Then check registry (builtins/imports)
		self.func_registry.get(name)
			.map(|f| f.call_index as u32)
			.unwrap_or_else(|| panic!("Unknown function: {}", name))
	}

	// ═══════════════════════════════════════════════════════════════════════════
	// User-defined function handling
	// ═══════════════════════════════════════════════════════════════════════════

	/// Extract user-defined function definitions from AST
	/// Recognizes patterns:
	/// - `name(param) = body` → Key(List[name, param], Assign, body)
	/// - `name := body` → Key(Symbol(name), Define, body) (uses implicit `it`)
	fn extract_user_functions(&mut self, node: &Node) {
		self.extract_user_functions_inner(node);
	}

	fn extract_user_functions_inner(&mut self, node: &Node) {
		let node = node.drop_meta();
		match node {
			// Pattern: name(param1, param2, ...) = body
			// Parses as Key(List([name, param1, param2, ...]), Assign, body)
			Node::Key(left, Op::Assign, body) => {
				if let Node::List(items, _, _) = left.drop_meta() {
					if items.len() >= 1 {
						if let Node::Symbol(name) = items[0].drop_meta() {
							// Extract parameter names
							let params: Vec<String> = items.iter().skip(1)
								.filter_map(|item| {
									match item.drop_meta() {
										Node::Symbol(s) => Some(s.clone()),
										// name:Type pattern
										Node::Key(n, Op::Colon, _) => {
											if let Node::Symbol(s) = n.drop_meta() {
												Some(s.clone())
											} else {
												None
											}
										}
										_ => None,
									}
								})
								.collect();

							let func_def = UserFunctionDef {
								name: name.clone(),
								params,
								body: body.clone(),
								return_kind: Kind::Int, // Infer later
								func_index: None,
							};
							self.user_functions.insert(name.clone(), func_def);
							return;
						}
					}
				}
				// Not a function definition, recurse
				self.extract_user_functions_inner(left);
				self.extract_user_functions_inner(body);
			}
			// Pattern: name := body (uses implicit `it` parameter)
			Node::Key(left, Op::Define, body) => {
				if let Node::Symbol(name) = left.drop_meta() {
					// Check if body uses `it` - if so, this is a function
					if Self::uses_it(body) {
						let func_def = UserFunctionDef {
							name: name.clone(),
							params: vec!["it".to_string()],
							body: body.clone(),
							return_kind: Kind::Int,
							func_index: None,
						};
						self.user_functions.insert(name.clone(), func_def);
						return;
					}
				}
				// Not a function definition, recurse
				self.extract_user_functions_inner(left);
				self.extract_user_functions_inner(body);
			}
			// Recurse into lists, but check for def syntax first
			Node::List(items, _, _) => {
				// Check for: def name(params): body or def name(params){body}
				if items.len() >= 2 {
					if let Node::Symbol(s) = items[0].drop_meta() {
						if s == "def" || s == "define" || s == "fun" || s == "fn" || s == "function" {
							if let Some(func_def) = self.extract_def_function(&items[1..]) {
								self.user_functions.insert(func_def.name.clone(), func_def);
								return;
							}
						}
					}
				}
				for item in items {
					self.extract_user_functions_inner(item);
				}
			}
			// Recurse into other key nodes
			Node::Key(left, _, right) => {
				self.extract_user_functions_inner(left);
				self.extract_user_functions_inner(right);
			}
			_ => {}
		}
	}

	/// Check if a node uses the implicit `it` parameter
	fn uses_it(node: &Node) -> bool {
		let node = node.drop_meta();
		match node {
			Node::Symbol(s) if s == "it" => true,
			Node::Key(left, _, right) => Self::uses_it(left) || Self::uses_it(right),
			Node::List(items, _, _) => items.iter().any(Self::uses_it),
			_ => false,
		}
	}

	/// Extract function from def/fun/fn syntax
	/// Handles: def (name params...): body or def ((name params...) {body})
	fn extract_def_function(&self, items: &[Node]) -> Option<UserFunctionDef> {
		if items.is_empty() {
			return None;
		}

		let first = items[0].drop_meta();

		// Pattern 1: def (name params...): body
		// Parses as: Key(List([name, params...]), Colon, body)
		if let Node::Key(sig, Op::Colon, body) = first {
			if let Node::List(sig_items, _, _) = sig.drop_meta() {
				if !sig_items.is_empty() {
					if let Node::Symbol(name) = sig_items[0].drop_meta() {
						let params: Vec<String> = sig_items.iter().skip(1)
							.filter_map(|item| {
								match item.drop_meta() {
									Node::Symbol(s) => Some(s.clone()),
									Node::Key(n, Op::Colon, _) => {
										if let Node::Symbol(s) = n.drop_meta() {
											Some(s.clone())
										} else {
											None
										}
									}
									_ => None,
								}
							})
							.collect();

						return Some(UserFunctionDef {
							name: name.clone(),
							params,
							body: body.clone(),
							return_kind: Kind::Int,
							func_index: None,
						});
					}
				}
			}
		}

		// Pattern 2: def ((name params...) {body})
		// Parses as: List([List([name, List([params...])]), Block({body})])
		// Note: parameters may be wrapped in a list like (x) instead of just x
		if let Node::List(inner_items, _, _) = first {
			if inner_items.len() >= 2 {
				// First item should be signature: (name (params...))
				if let Node::List(sig_items, _, _) = inner_items[0].drop_meta() {
					if !sig_items.is_empty() {
						if let Node::Symbol(name) = sig_items[0].drop_meta() {
							// Parameters might be in a nested list or directly
							let params: Vec<String> = sig_items.iter().skip(1)
								.flat_map(|item| {
									match item.drop_meta() {
										Node::Symbol(s) => vec![s.clone()],
										// Parameter wrapped in list: (x)
										Node::List(param_items, _, _) => {
											param_items.iter()
												.filter_map(|p| {
													match p.drop_meta() {
														Node::Symbol(s) => Some(s.clone()),
														Node::Key(n, Op::Colon, _) => {
															if let Node::Symbol(s) = n.drop_meta() {
																Some(s.clone())
															} else {
																None
															}
														}
														_ => None,
													}
												})
												.collect()
										}
										Node::Key(n, Op::Colon, _) => {
											if let Node::Symbol(s) = n.drop_meta() {
												vec![s.clone()]
											} else {
												vec![]
											}
										}
										_ => vec![],
									}
								})
								.collect();

							// Body is the second item (block)
							let body = inner_items[1].clone();

							return Some(UserFunctionDef {
								name: name.clone(),
								params,
								body: Box::new(body),
								return_kind: Kind::Int,
								func_index: None,
							});
						}
					}
				}
			}
		}

		None
	}

	/// Compile all extracted user functions to WASM
	fn compile_user_functions(&mut self) {
		// Clone the function names to avoid borrow issues
		let func_names: Vec<String> = self.user_functions.keys().cloned().collect();

		for name in func_names {
			self.compile_user_function(&name);
		}
	}

	/// Compile a single user function to WASM
	fn compile_user_function(&mut self, name: &str) {
		let user_fn = self.user_functions.get(name).unwrap().clone();

		// Create function type: (params...) -> i64
		// Use types.len() to get correct index (next_type_idx is stale after emit_constructors)
		let func_type_idx = self.types.len();
		let param_types: Vec<ValType> = user_fn.params.iter().map(|_| ValType::I64).collect();
		self.types.ty().function(param_types, vec![ValType::I64]);

		// Register function in function section
		self.functions.function(func_type_idx);
		let func_idx = self.next_func_idx;
		self.next_func_idx += 1;

		// Store the function index
		if let Some(fn_def) = self.user_functions.get_mut(name) {
			fn_def.func_index = Some(func_idx);
		}

		// Create function scope with parameters
		let saved_scope = std::mem::replace(&mut self.scope, Scope::new());
		for (_i, param_name) in user_fn.params.iter().enumerate() {
			self.scope.define(param_name.clone(), None, Kind::Int);
		}

		// Collect any additional variables in the body
		collect_variables(&user_fn.body, &mut self.scope);

		// Declare locals (parameters are already accounted for)
		let num_params = user_fn.params.len() as u32;
		let num_locals = self.scope.local_count();
		let extra_locals = if num_locals > num_params { num_locals - num_params } else { 0 };

		let mut func = Function::new(vec![(extra_locals, ValType::I64)]);

		// Compile the function body
		self.emit_numeric_value(&mut func, &user_fn.body);
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
		// Get numeric result
		self.emit_user_function_call_numeric(func, fn_name, args);
		// Wrap in new_int for Node return
		self.emit_call(func, "new_int");
	}

	/// Emit a call to a user-defined function (returns raw i64)
	fn emit_user_function_call_numeric(&mut self, func: &mut Function, fn_name: &str, args: &[Node]) {
		let user_fn = match self.user_functions.get(fn_name) {
			Some(f) => f.clone(),
			None => panic!("Unknown user function: {}", fn_name),
		};

		let func_index = match user_fn.func_index {
			Some(idx) => idx,
			None => panic!("User function not yet compiled: {}", fn_name),
		};

		// Emit each argument value
		for arg in args {
			self.emit_numeric_value(func, arg);
		}

		// Call the function
		func.instruction(&Instruction::Call(func_index));
	}

	// ═══════════════════════════════════════════════════════════════════════════
	// Helper methods for clean, DRY code
	// ═══════════════════════════════════════════════════════════════════════════

	/// Create a RefType for node references
	fn node_ref(&self, nullable: bool) -> RefType {
		RefType { nullable, heap_type: HeapType::Concrete(self.node_type) }
	}

	/// Emit string lookup from table and call constructor
	fn emit_string_call(&mut self, func: &mut Function, s: &str, constructor: &'static str) {
		let (ptr, len) = self.string_table
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
				if let Some(&(_, kind)) = self.user_globals.get(name) {
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

	/// Check if an expression requires float type (convenience method)
	fn needs_float(&self, node: &Node) -> bool {
		self.get_type(node).is_float()
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

	pub fn analyze_required_functions(&mut self, node: &Node) {
		let node = node.drop_meta();
		match node {
			Node::Empty => {
				self.required_functions.insert("new_empty");
			}
			Node::Number(num) => match num {
				Number::Int(_) => {
					self.required_functions.insert("new_int");
				}
				Number::Float(_) => {
					self.required_functions.insert("new_float");
				}
				_ => {
					self.required_functions.insert("new_empty");
				}
			},
			Node::Text(_) => {
				self.required_functions.insert("new_text");
			}
			Node::Char(_) => {
				self.required_functions.insert("new_codepoint");
			}
			Node::Symbol(_) => {
				self.required_functions.insert("new_symbol");
			}
			Node::Key(key, op, value) => {
				// Check for x = fetch URL pattern: Key(Assign, x, List[fetch, URL])
				if *op == Op::Assign || *op == Op::Define {
					if let Node::List(items, _, _) = value.drop_meta() {
						if items.len() == 2 {
							if let Node::Symbol(s) = items[0].drop_meta() {
								if s == "fetch" {
									self.required_functions.insert("new_text");
									return;
								}
							}
						}
					}
				}
				if op.is_arithmetic() || op.is_comparison() {
					self.required_functions.insert("new_int");
				} else {
					self.required_functions.insert("new_key");
				}
				self.analyze_required_functions(key);
				self.analyze_required_functions(value);
			}
			Node::List(items, _, _) => {
				if items.is_empty() {
					self.required_functions.insert("new_empty");
				} else {
					// Check for fetch pattern: [Symbol("fetch"), url_node]
					if items.len() == 2 {
						if let Node::Symbol(s) = items[0].drop_meta() {
							if s == "fetch" {
								// fetch returns a string, needs new_text
								self.required_functions.insert("new_text");
								return;
							}
						}
					}
					self.required_functions.insert("new_list");
					for item in items {
						self.analyze_required_functions(item);
					}
				}
			}
			Node::Data(_) => {
				self.required_functions.insert("new_data");
			}
			Node::Meta { node, .. } => {
				self.analyze_required_functions(node);
			}
			Node::Type { name, body } => {
				self.required_functions.insert("new_type");
				// Register the type definition in the registry
				self.type_registry.register_from_node(node);
				self.analyze_required_functions(name);
				self.analyze_required_functions(body);
			}
			_ => {}
		}
	}

	fn should_emit_function(&self, name: &str) -> bool {
		self.emit_all_functions || self.required_functions.contains(name)
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
		let types: Vec<TypeDef> = self.type_registry.types().to_vec();
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
		self.user_type_indices
			.insert(type_def.name.clone(), self.next_type_idx);
		self.next_type_idx += 1;
	}

	/// Emit constructors for registered user types
	fn emit_registered_user_type_constructors(&mut self) {
		let types: Vec<TypeDef> = self.type_registry.types().to_vec();
		for type_def in &types {
			self.emit_user_type_constructor(&type_def);
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
			self.exports
				.export(name, ExportKind::Global, self.next_global_idx);
			self.kind_global_indices.insert(tag, self.next_global_idx);
			self.next_global_idx += 1;
		}
	}

	/// Emit instruction to get a Kind kind value
	fn emit_kind(&self, func: &mut Function, tag: Kind) {
		if let Some(&idx) = self.kind_global_indices.get(&tag) {
			func.instruction(&Instruction::GlobalGet(idx));
		} else {
			func.instruction(&Instruction::I64Const(tag as i64));
		}
	}

	pub fn emit_for_node(&mut self, node: &Node) {
		self.emit_all_functions = false;
		// Extract user-defined functions first
		self.extract_user_functions(node);
		self.analyze_required_functions(node);
		let len = self.required_functions.len();
		trace!(
			"tree-shaking: {} functions required: {:?}",
			len,
			self.required_functions
		);
		self.emit();
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
				mutable: false,
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
			self.user_type_indices
				.insert(type_def.name.clone(), self.next_type_idx);
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
				if let Some(&type_idx) = self.user_type_indices.get(other) {
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
		self.user_type_indices.get(name).copied()
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
		let type_idx = match self.user_type_indices.get(&type_def.name) {
			Some(&idx) => idx,
			None => return,
		};

		let type_ref = RefType {
			nullable: false,
			heap_type: HeapType::Concrete(type_idx),
		};

		// Build parameter types
		let params: Vec<ValType> = type_def
			.fields
			.iter()
			.map(|f| self.field_def_to_val_type(f))
			.collect();

		// Function type: (params...) -> (ref $TypeName)
		let func_type = self.types.len();
		self.types
			.ty()
			.function(params.clone(), vec![Ref(type_ref)]);
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

	/// Convert FieldDef to ValType for function parameters
	fn field_def_to_val_type(&self, field: &FieldDef) -> ValType {
		match field.type_name.as_str() {
			"Int" | "i64" | "long" => ValType::I64,
			"Float" | "f64" | "double" => ValType::F64,
			"i32" | "int" => ValType::I32,
			"f32" | "float" => ValType::F32,
			"Text" | "String" | "string" => Ref(RefType {
				nullable: true,
				heap_type: HeapType::Concrete(self.string_type),
			}),
			"Node" => Ref(RefType {
				nullable: true,
				heap_type: HeapType::Concrete(self.node_type),
			}),
			other => {
				if let Some(&type_idx) = self.user_type_indices.get(other) {
					Ref(RefType {
						nullable: true,
						heap_type: HeapType::Concrete(type_idx),
					})
				} else {
					Ref(RefType {
						nullable: true,
						heap_type: any_heap_type(),
					})
				}
			}
		}
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
			self.types
				.ty()
				.function(vec![ValType::I64], vec![Ref(node_ref)]);
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
			self.types
				.ty()
				.function(vec![ValType::F64], vec![Ref(node_ref)]);
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
			self.types
				.ty()
				.function(vec![ValType::I32], vec![Ref(node_ref)]);
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

		// Emit helper functions
		self.emit_getters();
	}

	fn emit_getters(&mut self) {
		let node_ref = self.node_ref(true);

		// get_kind(node: ref $Node) -> i64
		let func_type = self.types.len();
		self.types
			.ty()
			.function(vec![Ref(node_ref)], vec![ValType::I64]);
		self.functions.function(func_type);
		let mut func = Function::new(vec![]);
		func.instruction(&Instruction::LocalGet(0));
		func.instruction(&Instruction::StructGet {
			struct_type_index: self.node_type,
			field_index: 0,
		});
		func.instruction(&Instruction::End);
		self.code.function(&func);
		self.exports
			.export("get_kind", ExportKind::Func, self.next_func_idx);
		self.next_func_idx += 1;
	}

	/// Allocate a string in linear memory
	fn allocate_string(&mut self, s: &str) -> (u32, u32) {
		if let Some(&offset) = self.string_table.get(s) {
			return (offset, s.len() as u32);
		}
		let offset = self.next_data_offset;
		let bytes = s.as_bytes();
		self.data.active(
			0,
			&ConstExpr::i32_const(offset as i32),
			bytes.iter().copied(),
		);
		self.string_table.insert(s.to_string(), offset);
		self.next_data_offset += bytes.len() as u32;
		(offset, bytes.len() as u32)
	}

	/// Emit main function that constructs the node
	pub fn emit_node_main(&mut self, node: &Node) {
		self.collect_and_allocate_strings(node);

		// Pre-pass: collect variables and count temp locals needed
		let temp_locals = collect_variables(node, &mut self.scope);
		let var_count = self.scope.local_count();
		self.next_temp_local = var_count; // Temp locals start after variables

		let node_ref = self.node_ref(false);
		let func_type = self.types.len();
		self.types.ty().function(vec![], vec![Ref(node_ref)]);
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
		self.exports
			.export("main", ExportKind::Func, self.next_func_idx);
		self.next_func_idx += 1;
	}

	fn collect_and_allocate_strings(&mut self, node: &Node) {
		let node = node.drop_meta();
		match node {
			Node::Text(s) | Node::Symbol(s) => {
				self.allocate_string(s);
			}
			Node::Key(key, _, value) => {
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
		self.used_functions.insert(name);
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
						return;  // Already a Node reference
					} else if local.kind.is_float() {
						self.emit_call(func, "new_float");
					} else {
						self.emit_call(func, "new_int");
					}
					return;
				}
				// Check if this is a global variable lookup
				if let Some(&(idx, kind)) = self.user_globals.get(s) {
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
						if self.user_functions.contains_key(name) {
							// Function definitions don't produce a value
							// The caller (statement sequence handler) should skip this
							return;
						}
					}
				}
				// Route to emit_arithmetic for:
				// - Arithmetic/comparison ops
				// - Define (:=) always creates a numeric local
				// - Assign (=) only if LHS is a known variable (numeric context)
				// - Compound assignments (+=, -=, etc.)
				let is_numeric_assign = *op == Op::Assign && matches!(left.drop_meta(), Node::Symbol(s) if self.scope.lookup(s).is_some());
				if op.is_arithmetic() || op.is_comparison() || *op == Op::Define || is_numeric_assign || op.is_compound_assign() {
					self.emit_arithmetic(func, left, op, right);
				} else if *op == Op::Question {
					// Ternary: condition ? then : else
					self.emit_ternary(func, left, right);
				} else if *op == Op::Else {
					// If-then-else: ((if condition) then then_expr) else else_expr
					self.emit_if_then_else(func, left, Some(right));
				} else if *op == Op::Then {
					// If-then (no else): (if condition) then then_expr
					// Construct the full structure for emit_if_then_else
					let full_node = Node::Key(left.clone(), Op::Then, right.clone());
					self.emit_if_then_else(func, &full_node, None);
				} else if *op == Op::Do {
					// While loop: (while condition) do body
					self.emit_while_loop(func, left, right);
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
				// Check for user function call: [Symbol("funcname"), arg1, arg2, ...]
				if items.len() >= 2 {
					if let Node::Symbol(fn_name) = items[0].drop_meta() {
						if self.user_functions.contains_key(fn_name) {
							self.emit_user_function_call(func, fn_name, &items[1..]);
							return;
						}
					}
				}
				// Check if this list contains type definitions (class/struct)
				// If so, treat as statement sequence and return last non-Type item
				let has_type_def = items.iter().any(|item| {
					matches!(item.drop_meta(), Node::Type { .. })
				});
				if has_type_def {
					// Find last non-Type item to return
					let last_expr = items.iter().rev().find(|item| {
						!matches!(item.drop_meta(), Node::Type { .. })
					});
					if let Some(expr) = last_expr {
						self.emit_node_instructions(func, expr);
					} else {
						// All items are Type definitions, return empty
						self.emit_call(func, "new_empty");
					}
					return;
				}
				// Check if this is a statement sequence with assignments/definitions/globals
				let is_statement_sequence = items.iter().any(|item| {
					let item = item.drop_meta();
					match item {
						Node::Key(_, Op::Assign | Op::Define, _) => true,
						Node::Key(_, op, _) if op.is_compound_assign() => true,
						// global:... is a statement
						Node::Key(left, Op::Colon, _) => {
							matches!(left.drop_meta(), Node::Symbol(s) if s == "global")
						}
						// def/fun/fn syntax starts a statement sequence
						Node::List(list_items, _, _) if list_items.len() >= 2 => {
							if let Node::Symbol(s) = list_items[0].drop_meta() {
								s == "def" || s == "define" || s == "fun" || s == "fn" || s == "function"
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
					let non_func_items: Vec<_> = items.iter()
						.filter(|item| {
							match item.drop_meta() {
								// Pattern: name := body (uses implicit `it` parameter)
								Node::Key(left, Op::Define, _) => {
									if let Node::Symbol(name) = left.drop_meta() {
										if self.user_functions.contains_key(name) {
											return false;
										}
									}
								}
								// Pattern: name(params...) = body
								Node::Key(left, Op::Assign, _) => {
									if let Node::List(items, _, _) = left.drop_meta() {
										if !items.is_empty() {
											if let Node::Symbol(name) = items[0].drop_meta() {
												if self.user_functions.contains_key(name) {
													return false;
												}
											}
										}
									}
								}
								// Pattern: def/fun/fn name(params...): body or {body}
								Node::List(list_items, _, _) => {
									if list_items.len() >= 2 {
										if let Node::Symbol(s) = list_items[0].drop_meta() {
											if s == "def" || s == "define" || s == "fun" || s == "fn" || s == "function" {
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
						if self.needs_float(node) {
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
		let use_float = self.needs_float(left) || self.needs_float(right);

		// Handle variable definition/assignment specially
		if *op == Op::Define || *op == Op::Assign {
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
				let local_pos = self.scope.lookup(name)
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
				let local_pos = self.scope.lookup(name)
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
			self.emit_float_value(func, left);
			self.emit_float_value(func, right);

			match op {
				Op::Add => { func.instruction(&Instruction::F64Add); }
				Op::Sub => { func.instruction(&Instruction::F64Sub); }
				Op::Mul => { func.instruction(&Instruction::F64Mul); }
				Op::Div => { func.instruction(&Instruction::F64Div); }
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
					warn!("Power operator not fully implemented for float");
					func.instruction(&Instruction::F64Mul);
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
			self.emit_numeric_value(func, left);
			self.emit_numeric_value(func, right);

			match op {
				Op::Add => { func.instruction(&Instruction::I64Add); }
				Op::Sub => { func.instruction(&Instruction::I64Sub); }
				Op::Mul => { func.instruction(&Instruction::I64Mul); }
				Op::Div => { func.instruction(&Instruction::I64DivS); }
				Op::Mod => { func.instruction(&Instruction::I64RemS); }
				Op::Pow => {
					warn!("Power operator not fully implemented, using multiplication");
					func.instruction(&Instruction::I64Mul);
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
		if let Some(f) = self.func_registry.get("host_fetch") {
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
				Node::List(items, _, _) => {
					items.iter().map(node_to_string).collect::<Vec<_>>().join("")
				}
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
		if let Some(&(global_idx, existing_kind)) = self.user_globals.get(&name) {
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
		self.user_globals.insert(name.clone(), (global_idx, kind));

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
		if let Some(&(global_idx, existing_kind)) = self.user_globals.get(&name) {
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
		self.user_globals.insert(name.clone(), (global_idx, kind));

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
		func.instruction(&Instruction::If(BlockType::Result(ValType::Ref(self.node_ref(false)))));

		// Then branch
		self.emit_numeric_value(func, &then_expr);
		self.emit_call(func, "new_int");

		func.instruction(&Instruction::Else);

		// Else branch
		self.emit_numeric_value(func, &else_expr);
		self.emit_call(func, "new_int");

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
		self.emit_numeric_value(func, &then_expr);

		func.instruction(&Instruction::Else);

		// Else branch
		self.emit_numeric_value(func, &else_expr);

		func.instruction(&Instruction::End);
	}

	/// Emit if-then-else returning i64: if condition then then_expr else else_expr
	fn emit_if_then_else_numeric(
		&mut self,
		func: &mut Function,
		left: &Node,
		else_expr: Option<&Node>,
	) {
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
	fn emit_if_then_else(
		&mut self,
		func: &mut Function,
		left: &Node,
		else_expr: Option<&Node>,
	) {
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
		self.emit_block_value(func, &condition);
		func.instruction(&Instruction::I32WrapI64);

		// if (condition) { then_expr } else { else_expr }
		func.instruction(&Instruction::If(BlockType::Result(ValType::Ref(self.node_ref(false)))));

		// Then branch - extract value from block if needed
		self.emit_block_value(func, &then_expr);
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

		self.emit_block_value(func, &condition);
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
			// Variable definition/assignment: x:=42 or x=42 → store and return value
			Node::Key(left, Op::Define | Op::Assign, right) => {
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
					let local_pos = self.scope.lookup(name)
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
					let local_pos = self.scope.lookup(name)
						.map(|l| l.position)
						.unwrap_or_else(|| panic!("Undefined variable: {}", name));
					let base_op = op.base_op();
					// Get current value of x
					func.instruction(&Instruction::LocalGet(local_pos));
					// Emit y
					self.emit_numeric_value(func, right);
					// Apply base operation
					match base_op {
						Op::Add => func.instruction(&Instruction::I64Add),
						Op::Sub => func.instruction(&Instruction::I64Sub),
						Op::Mul => func.instruction(&Instruction::I64Mul),
						Op::Div => func.instruction(&Instruction::I64DivS),
						Op::Mod => func.instruction(&Instruction::I64RemS),
						Op::Pow => func.instruction(&Instruction::I64Mul), // placeholder
						Op::And => func.instruction(&Instruction::I64And),
						Op::Or => func.instruction(&Instruction::I64Or),
						Op::Xor => func.instruction(&Instruction::I64Xor),
						_ => panic!("Unexpected base op: {:?}", base_op),
					};
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
					Op::Add => func.instruction(&Instruction::I64Add),
					Op::Sub => func.instruction(&Instruction::I64Sub),
					Op::Mul => func.instruction(&Instruction::I64Mul),
					Op::Div => func.instruction(&Instruction::I64DivS),
					Op::Mod => func.instruction(&Instruction::I64RemS),
					Op::Pow => func.instruction(&Instruction::I64Mul), // placeholder
					_ => unreachable!(),
				};
			}
			// Comparison operators
			Node::Key(left, op, right) if op.is_comparison() => {
				self.emit_numeric_value(func, left);
				self.emit_numeric_value(func, right);
				self.emit_comparison(func, op);
			}
			// Ternary operator: condition ? then_expr : else_expr
			Node::Key(condition, Op::Question, then_else) => {
				self.emit_ternary_numeric(func, condition, then_else);
			}
			// If-then-else: if condition then then_expr else else_expr
			Node::Key(if_then, Op::Else, else_expr) => {
				self.emit_if_then_else_numeric(func, if_then, Some(else_expr));
			}
			// Variable lookup (local or global)
			Node::Symbol(name) => {
				if let Some(local) = self.scope.lookup(name) {
					func.instruction(&Instruction::LocalGet(local.position));
				} else if let Some(&(idx, kind)) = self.user_globals.get(name) {
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
			Node::List(items, _, _) if !items.is_empty() => {
				// Check for user function call: [Symbol("funcname"), arg1, arg2, ...]
				if items.len() >= 2 {
					if let Node::Symbol(fn_name) = items[0].drop_meta() {
						if self.user_functions.contains_key(fn_name) {
							self.emit_user_function_call_numeric(func, fn_name, &items[1..]);
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
					Op::Add => { func.instruction(&Instruction::F64Add); }
					Op::Sub => { func.instruction(&Instruction::F64Sub); }
					Op::Mul => { func.instruction(&Instruction::F64Mul); }
					Op::Div => { func.instruction(&Instruction::F64Div); }
					Op::Mod => {
						// WASM doesn't have F64Rem. Drop f64 values and use i64 path.
						func.instruction(&Instruction::Drop);
						func.instruction(&Instruction::Drop);
						self.emit_numeric_value(func, left);
						self.emit_numeric_value(func, right);
						func.instruction(&Instruction::I64RemS);
						func.instruction(&Instruction::F64ConvertI64S);
					}
					Op::Pow => { func.instruction(&Instruction::F64Mul); } // placeholder
					_ => unreachable!(),
				}
			}
			// Variable lookup (local or global) - convert i64 to f64 if needed
			Node::Symbol(name) => {
				if let Some(local) = self.scope.lookup(name) {
					func.instruction(&Instruction::LocalGet(local.position));
					if !local.kind.is_float() {
						// Convert i64 local to f64
						func.instruction(&Instruction::F64ConvertI64S);
					}
				} else if let Some(&(idx, kind)) = self.user_globals.get(name) {
					func.instruction(&Instruction::GlobalGet(idx));
					if !kind.is_float() {
						// Convert i64 global to f64
						func.instruction(&Instruction::F64ConvertI64S);
					}
				} else {
					panic!("Undefined variable: {}", name);
				}
			}
			// Statement sequence: execute all, return last as float
			Node::List(items, _, _) if !items.is_empty() => {
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

		// Emit rest
		if items.len() > 2 {
			// Recursive: rest is another list
			let rest = Node::List(
				items[1..].to_vec(),
				bracket.clone(),
				crate::node::Separator::None,
			);
			self.emit_node_instructions(func, &rest);
		} else if items.len() == 2 {
			// Last pair: rest is the second item directly
			self.emit_node_instructions(func, &items[1]);
		} else {
			self.emit_node_null(func);
		}

		// bracket_info
		func.instruction(&Instruction::I64Const(bracket_info));

		// Call new_list if available, otherwise inline struct.new
		if self.func_registry.contains("new_list") {
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

	fn validate_wasm(bytes: &Vec<u8>) {
		if let Err(e) = Self::try_validate_wasm(bytes) {
			panic!("WASM validation failed: {}", e);
		}
	}

	fn try_validate_wasm(bytes: &Vec<u8>) -> Result<(), String> {
		let mut features = WasmFeatures::default();
		features.set(WasmFeatures::REFERENCE_TYPES, true);
		features.set(WasmFeatures::GC, true);
		let mut validator = Validator::new_with_features(features);
		match validator.validate_all(&bytes) {
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
		if self.func_registry.import_count() > 0 {
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
		for (name, &idx) in &self.user_type_indices {
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
		for type_def in self.type_registry.types() {
			if let Some(&type_idx) = self.user_type_indices.get(&type_def.name) {
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
		let mut sorted: Vec<_> = self.func_registry.all().iter()
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
		self.func_registry.all().iter()
			.map(|f| f.name.clone())
			.filter(|name: &String| !self.used_functions.contains(name.as_str()))
			.collect()
	}

	pub fn get_used_functions(&self) -> Vec<&'static str> {
		self.used_functions.iter().copied().collect()
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
		use wasm_encoder::*;
		use wasm_encoder::StorageType::Val;

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
			FieldType { element_type: Val(ValType::I32), mutable: false },
			FieldType { element_type: Val(ValType::I32), mutable: false },
		]);
		let string_type_idx = 0u32;

		// Type 1: User struct type
		let string_ref = RefType { nullable: false, heap_type: HeapType::Concrete(string_type_idx) };
		let struct_fields: Vec<FieldType> = type_def.fields.iter().map(|f| {
			let element_type = match f.type_name.as_str() {
				"i64" | "Int" | "long" => Val(ValType::I64),
				"i32" | "int" => Val(ValType::I32),
				"f64" | "Float" | "double" => Val(ValType::F64),
				"f32" | "float" => Val(ValType::F32),
				"String" | "Text" | "string" => Val(ValType::Ref(string_ref)),
				_ => Val(ValType::I64), // default
			};
			FieldType { element_type, mutable: false }
		}).collect();
		types.ty().struct_(struct_fields);
		let struct_type_idx = 1u32;

		// Type 2: main() -> ref $StructType
		let struct_ref = RefType { nullable: false, heap_type: HeapType::Concrete(struct_type_idx) };
		types.ty().func_type(&FuncType::new([], [ValType::Ref(struct_ref)]));

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
					func.instruction(&Instruction::I32Const(*v));
				}
				RawFieldValue::F64(v) => {
					func.instruction(&Instruction::F64Const(Ieee64::new(v.to_bits())));
				}
				RawFieldValue::F32(v) => {
					func.instruction(&Instruction::F32Const(Ieee32::new(v.to_bits())));
				}
				RawFieldValue::String(_) => {
					let (ptr, len) = string_offsets[string_idx];
					func.instruction(&Instruction::I32Const(ptr as i32));
					func.instruction(&Instruction::I32Const(len as i32));
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

/// Raw field values for emit_raw_struct
#[derive(Debug, Clone)]
pub enum RawFieldValue {
	I64(i64),
	I32(i32),
	F64(f64),
	F32(f32),
	String(String),
}

impl From<i64> for RawFieldValue {
	fn from(v: i64) -> Self { RawFieldValue::I64(v) }
}

impl From<i32> for RawFieldValue {
	fn from(v: i32) -> Self { RawFieldValue::I32(v) }
}

impl From<f64> for RawFieldValue {
	fn from(v: f64) -> Self { RawFieldValue::F64(v) }
}

impl From<f32> for RawFieldValue {
	fn from(v: f32) -> Self { RawFieldValue::F32(v) }
}

impl From<&str> for RawFieldValue {
	fn from(v: &str) -> Self { RawFieldValue::String(v.to_string()) }
}

impl From<String> for RawFieldValue {
	fn from(v: String) -> Self { RawFieldValue::String(v) }
}

/// Run raw struct WASM and return GcObject wrapped in Node::Data
pub fn run_raw_struct(wasm_bytes: &[u8]) -> Result<Node, String> {
	use wasmtime::{Store, Module, Linker, Val};

	// Register WASM metadata for field name lookup in Debug output
	let _ = crate::gc_traits::register_gc_types_from_wasm(wasm_bytes);

	let engine = gc_engine();
	let mut store = Store::new(&engine, ());
	let module = Module::new(&engine, wasm_bytes).map_err(|e: wasmtime::Error| e.to_string())?;

	let linker = Linker::new(&engine);
	let instance = linker.instantiate(&mut store, &module).map_err(|e: wasmtime::Error| e.to_string())?;

	let main = instance.get_func(&mut store, "main")
		.ok_or_else(|| "no main function".to_string())?;

	let mut results = vec![Val::I32(0)];
	main.call(&mut store, &[], &mut results).map_err(|e: wasmtime::Error| e.to_string())?;

	let gc_obj = ErgonomicGcObject::new(results[0].clone(), store, Some(instance))
		.map_err(|e: anyhow::Error| e.to_string())?;

	Ok(crate::node::data(gc_obj))
}

/// Check if parsed node is "class Foo{...}; Foo{...}" pattern
fn is_class_with_instance(node: &Node) -> Option<(Node, Node)> {
	match node.drop_meta() {
		Node::List(items, _, _) if items.len() >= 2 => {
			// First should be Type (class def), second should be Key (instance)
			let first = items[0].drop_meta();
			let second = items[1].drop_meta();
			if matches!(first, Node::Type { .. }) && matches!(second, Node::Key(_, _, _)) {
				Some((items[0].clone(), items[1].clone()))
			} else {
				None
			}
		}
		_ => None,
	}
}

/// Check if code uses fetch (needs host imports)
fn uses_fetch(code: &str) -> bool {
	code.contains("fetch ")
}

// Re-export eval function for tests
pub fn eval(code: &str) -> Node {
	use crate::type_kinds::{TypeDef, extract_instance_values};
	use crate::wasm_gc_reader::read_bytes_with_host;

	let node = WaspParser::parse(code);

	// Check for class definition + instance pattern
	if let Some((class_node, instance_node)) = is_class_with_instance(&node) {
		if let Some(type_def) = TypeDef::from_node(&class_node) {
			if let Some((_type_name, field_values)) = extract_instance_values(&instance_node) {
				let wasm_bytes = WasmGcEmitter::emit_raw_struct(&type_def, &field_values);
				match run_raw_struct(&wasm_bytes) {
					Ok(result) => return result,
					Err(e) => warn!("raw struct eval failed: {}", e),
				}
			}
		}
	}

	// Fallback to standard Node encoding
	let mut emitter = WasmGcEmitter::new();
	let needs_host = uses_fetch(code);
	if needs_host {
		emitter.set_host_imports(true);
	}
	emitter.emit_for_node(&node);
	let bytes = emitter.finish();

	// Use host linker if we need host functions
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
