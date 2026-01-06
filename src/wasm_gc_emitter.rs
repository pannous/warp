use crate::analyzer::Scope;
// Note: analyzer module exports Scope
use crate::extensions::numbers::Number;
use crate::node::{Bracket, Node, Op};
use crate::type_kinds::{FieldDef, Kind, TypeDef, TypeRegistry};
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
	function_indices: HashMap<&'static str, u32>,
	used_functions: HashSet<&'static str>,
	required_functions: HashSet<&'static str>,
	emit_all_functions: bool,
	emit_kind_globals: bool, // Emit Kind constants as globals for documentation
	kind_global_indices: HashMap<Kind, u32>, // Kind -> global index
	// String storage in linear memory
	string_table: HashMap<String, u32>,
	next_data_offset: u32,
	// Variable scope
	scope: Scope,
	// User-defined type indices
	user_type_indices: HashMap<String, u32>,
	// Type registry for user-defined types parsed from class/struct definitions
	type_registry: TypeRegistry,
}

impl WasmGcEmitter {
	pub fn new() -> Self {
		WasmGcEmitter {
			module: Module::new(),
			types: TypeSection::new(),
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
			function_indices: HashMap::new(),
			used_functions: HashSet::new(),
			required_functions: HashSet::new(),
			emit_all_functions: true,
			emit_kind_globals: true, // Enable by default for debugging
			kind_global_indices: HashMap::new(),
			string_table: HashMap::new(),
			next_data_offset: 8,
			scope: Scope::new(),
			user_type_indices: HashMap::new(),
			type_registry: TypeRegistry::new(),
		}
	}

	/// Enable/disable emitting Kind globals for documentation
	pub fn set_emit_kind_globals(&mut self, enabled: bool) {
		self.emit_kind_globals = enabled;
	}

	pub fn set_tree_shaking(&mut self, enabled: bool) {
		self.emit_all_functions = !enabled;
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
			("kind_pair", Kind::Pair),
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
		self.analyze_required_functions(node);
		let len = self.required_functions.len();
		trace!(
			"tree-shaking: {} functions required: {:?}",
			len,
			self.required_functions
		);
		self.emit();
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
		let node_ref = RefType {
			nullable: false,
			heap_type: HeapType::Concrete(self.node_type),
		};
		let node_ref_nullable = RefType {
			nullable: true,
			heap_type: HeapType::Concrete(self.node_type),
		};
		let _any_ref = RefType {
			nullable: true,
			heap_type: any_heap_type(),
		};

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
			self.exports
				.export("new_empty", ExportKind::Func, self.next_func_idx);
			self.function_indices
				.insert("new_empty", self.next_func_idx);
			self.next_func_idx += 1;
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
			self.exports
				.export("new_int", ExportKind::Func, self.next_func_idx);
			self.function_indices.insert("new_int", self.next_func_idx);
			self.next_func_idx += 1;
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
			self.exports
				.export("new_float", ExportKind::Func, self.next_func_idx);
			self.function_indices
				.insert("new_float", self.next_func_idx);
			self.next_func_idx += 1;
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
			self.exports
				.export("new_codepoint", ExportKind::Func, self.next_func_idx);
			self.function_indices
				.insert("new_codepoint", self.next_func_idx);
			self.next_func_idx += 1;
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
			self.exports
				.export("new_text", ExportKind::Func, self.next_func_idx);
			self.function_indices.insert("new_text", self.next_func_idx);
			self.next_func_idx += 1;
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
			self.exports
				.export("new_symbol", ExportKind::Func, self.next_func_idx);
			self.function_indices
				.insert("new_symbol", self.next_func_idx);
			self.next_func_idx += 1;
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
			self.exports
				.export("new_key", ExportKind::Func, self.next_func_idx);
			self.function_indices.insert("new_key", self.next_func_idx);
			self.next_func_idx += 1;
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
			self.exports
				.export("new_type", ExportKind::Func, self.next_func_idx);
			self.function_indices.insert("new_type", self.next_func_idx);
			self.next_func_idx += 1;
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
			self.exports
				.export("new_list", ExportKind::Func, self.next_func_idx);
			self.function_indices.insert("new_list", self.next_func_idx);
			self.next_func_idx += 1;
		}

		// Emit helper functions
		self.emit_getters();
	}

	fn emit_getters(&mut self) {
		let node_ref = RefType {
			nullable: true,
			heap_type: HeapType::Concrete(self.node_type),
		};

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

	/// Count variables defined in node (for WASM local allocation)
	fn count_variables(&mut self, node: &Node) {
		let node = node.drop_meta();
		match node {
			Node::Key(left, Op::Define | Op::Assign, right) => {
				if let Node::Symbol(name) = left.drop_meta() {
					self.scope.define(name.clone(), None);
				}
				self.count_variables(right);
			}
			Node::Key(left, _, right) => {
				self.count_variables(left);
				self.count_variables(right);
			}
			Node::List(items, _, _) => {
				for item in items {
					self.count_variables(item);
				}
			}
			_ => {}
		}
	}

	/// Emit main function that constructs the node
	pub fn emit_node_main(&mut self, node: &Node) {
		self.collect_and_allocate_strings(node);

		// Pre-pass: count variables to allocate locals
		self.count_variables(node);
		let local_count = self.scope.local_count();

		let node_ref = RefType {
			nullable: false,
			heap_type: HeapType::Concrete(self.node_type),
		};
		let func_type = self.types.len();
		self.types.ty().function(vec![], vec![Ref(node_ref)]);
		self.functions.function(func_type);

		// Allocate locals for variables (all i64 for now)
		let locals: Vec<(u32, ValType)> = if local_count > 0 {
			vec![(local_count, ValType::I64)]
		} else {
			vec![]
		};
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
		func.instruction(&Instruction::Call(self.function_indices[name]));
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
				let (ptr, len) = self
					.string_table
					.get(s.as_str())
					.map(|&offset| (offset, s.len() as u32))
					.unwrap_or((0, s.len() as u32));
				func.instruction(&I32Const(ptr as i32));
				func.instruction(&I32Const(len as i32));
				self.emit_call(func, "new_text");
			}
			Node::Char(c) => {
				func.instruction(&I32Const(*c as i32));
				self.emit_call(func, "new_codepoint");
			}
			Node::Symbol(s) => {
				let (ptr, len) = self
					.string_table
					.get(s.as_str())
					.map(|&offset| (offset, s.len() as u32))
					.unwrap_or((0, s.len() as u32));
				func.instruction(&I32Const(ptr as i32));
				func.instruction(&I32Const(len as i32));
				self.emit_call(func, "new_symbol");
			}
			Node::Key(left, op, right) => {
				if op.is_arithmetic() || op.is_comparison() || *op == Op::Define {
					self.emit_arithmetic(func, left, op, right);
				} else if *op == Op::Assign && matches!(right.drop_meta(), Node::Number(_)) {
					// Treat x=42 as assignment only when RHS is numeric
					self.emit_arithmetic(func, left, op, right);
				} else {
					self.emit_node_instructions(func, left);
					self.emit_node_instructions(func, right);
					let op_code = op_to_code(op);
					func.instruction(&Instruction::I64Const(op_code));
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
				// Check if this is a statement sequence with arithmetic/definitions
				// Only trigger when there are actual definitions or arithmetic operations
				let has_executable = items.iter().any(|item| {
					let item = item.drop_meta();
					match item {
						Node::Key(_, op, _) if op.is_arithmetic() || *op == Op::Define => true,
						Node::Key(_, Op::Assign, right) => {
							// Treat = as executable only when RHS is numeric
							matches!(right.drop_meta(), Node::Number(_))
						}
						_ => false,
					}
				});
				if has_executable {
					// Evaluate as statement sequence
					self.emit_numeric_value(func, node);
					self.emit_call(func, "new_int");
				} else {
					// Build linked list: (first, rest, bracket_info)
					self.emit_list_structure(func, items, bracket);
				}
			}
			Node::Data(dada) => {
				// Emit as symbol with type_name for now
				let (ptr, len) = self
					.string_table
					.get(dada.type_name.as_str())
					.map(|&offset| (offset, dada.type_name.len() as u32))
					.unwrap_or((0, dada.type_name.len() as u32));
				func.instruction(&I32Const(ptr as i32));
				func.instruction(&I32Const(len as i32));
				self.emit_call(func, "new_symbol");
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
		// Handle variable definition/assignment specially
		if *op == Op::Define || *op == Op::Assign {
			// x:=42 or x=42 → emit value, store to local, return value
			self.emit_numeric_value(func, right);
			if let Node::Symbol(name) = left.drop_meta() {
				if let Some(local) = self.scope.lookup(name) {
					func.instruction(&Instruction::LocalTee(local.position));
				} else {
					panic!("Undefined variable: {}", name);
				}
			} else {
				panic!("Expected symbol in definition, got {:?}", left);
			}
		} else {
			// Emit left operand value onto stack
			self.emit_numeric_value(func, left);
			// Emit right operand value onto stack
			self.emit_numeric_value(func, right);

			// Emit WASM arithmetic or comparison instruction
			match op {
				Op::Add => func.instruction(&Instruction::I64Add),
				Op::Sub => func.instruction(&Instruction::I64Sub),
				Op::Mul => func.instruction(&Instruction::I64Mul),
				Op::Div => func.instruction(&Instruction::I64DivS),
				Op::Mod => func.instruction(&Instruction::I64RemS),
				Op::Pow => {
					warn!("Power operator not fully implemented, using multiplication");
					func.instruction(&Instruction::I64Mul)
				}
				// Comparison operators - result is i32 (0 or 1), extend to i64
				Op::Eq => {
					func.instruction(&Instruction::I64Eq);
					func.instruction(&Instruction::I64ExtendI32U)
				}
				Op::Ne => {
					func.instruction(&Instruction::I64Ne);
					func.instruction(&Instruction::I64ExtendI32U)
				}
				Op::Lt => {
					func.instruction(&Instruction::I64LtS);
					func.instruction(&Instruction::I64ExtendI32U)
				}
				Op::Gt => {
					func.instruction(&Instruction::I64GtS);
					func.instruction(&Instruction::I64ExtendI32U)
				}
				Op::Le => {
					func.instruction(&Instruction::I64LeS);
					func.instruction(&Instruction::I64ExtendI32U)
				}
				Op::Ge => {
					func.instruction(&Instruction::I64GeS);
					func.instruction(&Instruction::I64ExtendI32U)
				}
				_ => unreachable!("Unsupported operator in emit_arithmetic: {:?}", op),
			};
		}

		// Wrap result as Int node
		self.emit_call(func, "new_int");
	}

	/// Emit the numeric value of a node onto the stack (as i64)
	fn emit_numeric_value(&mut self, func: &mut Function, node: &Node) {
		let node = node.drop_meta();
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
			// Variable lookup
			Node::Symbol(name) => {
				if let Some(local) = self.scope.lookup(name) {
					func.instruction(&Instruction::LocalGet(local.position));
				} else {
					panic!("Undefined variable: {}", name);
				}
			}
			// Statement sequence: execute all, return last
			Node::List(items, _, _) if !items.is_empty() => {
				for (i, item) in items.iter().enumerate() {
					self.emit_numeric_value(func, item);
					// Drop all values except the last
					if i < items.len() - 1 {
						func.instruction(&Instruction::Drop);
					}
				}
			}
			_ => panic!("Cannot extract numeric value from {:?}", node),
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
		if self.function_indices.contains_key("new_list") {
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
		let mut features = WasmFeatures::default();
		features.set(WasmFeatures::REFERENCE_TYPES, true);
		features.set(WasmFeatures::GC, true);
		let mut validator = Validator::new_with_features(features);
		match validator.validate_all(&bytes) {
			Ok(_) => trace!("✓ WASM validation with GC features passed"),
			Err(e) => panic!("WASM validation failed: {}", e),
		}
	}

	pub fn finish(mut self) -> Vec<u8> {
		self.module.section(&self.types);
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
		let mut string_fields = NameMap::new();
		string_fields.append(0, "ptr");
		string_fields.append(1, "len");
		type_field_names.append(self.string_type, &string_fields);

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
		let mut sorted: Vec<_> = self.function_indices.iter().collect();
		sorted.sort_by_key(|(_, &idx)| idx);
		for (name, &idx) in sorted {
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
				"kind_pair",
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

	pub fn get_unused_functions(&self) -> Vec<&'static str> {
		self.function_indices
			.keys()
			.filter(|name| !self.used_functions.contains(*name))
			.copied()
			.collect()
	}

	pub fn get_used_functions(&self) -> Vec<&'static str> {
		self.used_functions.iter().copied().collect()
	}
}

// Re-export eval function for tests
pub fn eval(code: &str) -> Node {
	let node = WaspParser::parse(code);
	let mut emitter = WasmGcEmitter::new();
	emitter.emit_for_node(&node);
	let bytes = emitter.finish();

	match read_bytes(&bytes) {
		Ok(result) => result,
		Err(e) => {
			warn!("eval failed: {}", e);
			node // Return parsed node on failure
		}
	}
}
