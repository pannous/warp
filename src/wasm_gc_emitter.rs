use crate::extensions::numbers::Number;
use crate::node::{Bracket, DataType, Node};
use crate::wasm_gc_reader::read_bytes;
use crate::wasp_parser::WaspParser;
use log::{trace, warn};
use std::collections::HashMap;
use wasm_encoder::*;
use wasmparser::{Validator, WasmFeatures};
use Instruction::I32Const;
use StorageType::Val;
use ValType::Ref;
// use wast::type_kinds::NodeKind;
use crate::type_kinds::NodeKind;

/// WebAssembly GC emitter for Node AST
/// Generates WASM GC bytecode using struct and array types
pub struct WasmGcEmitter {
	module: Module,
	types: TypeSection,
	functions: FunctionSection,
	code: CodeSection,
	exports: ExportSection,
	names: NameSection,
	memory: MemorySection,
	data: DataSection,
	// Type indices for unified GC types
	node_base_type: u32,
	node_array_type: u32,
	next_type_idx: u32,
	next_func_idx: u32,
	function_indices: HashMap<&'static str, u32>, //  (Constructor and later others)
	// String storage for linear memory
	string_table: HashMap<String, u32>, // Maps string -> memory offset
	next_data_offset: u32,
	data_segment_names: Vec<(u32, String)>, // (data_index, name) for name section
	next_data_segment_idx: u32,
}

/// Specifies how a struct field gets its value in a constructor
#[derive(Debug, Clone, Copy)]
enum FieldValue {
	Zero,                // I32 zero constant
	I64Zero,             // I64 zero constant
	FloatZero,           // F64 zero constant
	Null,                // Ref null
	LocalI32(u32),       // LocalGet for i32 parameter
	LocalI64(u32),       // LocalGet for i64 parameter
	LocalF64(u32),       // LocalGet for f64 parameter
	LocalRef(u32),       // LocalGet for ref node parameter
	LocalI32AsI64(u32),  // LocalGet for i32 parameter, extend to i64 (unsigned)
	KindField(NodeKind), // Constant NodeKind value as i32
}

/// Descriptor for a node constructor function
struct NodeConstructor {
	export_name: &'static str,
	params: Vec<ValType>,
	fields: [FieldValue; 10], // Values for all 10 struct fields in order
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
			node_base_type: 0,
			node_array_type: 0,
			next_type_idx: 0,
			next_func_idx: 0,
			function_indices: HashMap::new(),
			string_table: HashMap::new(),
			next_data_offset: 8, // Start at offset 8 to avoid confusion with null (0)
			data_segment_names: Vec::new(),
			next_data_segment_idx: 0,
		}
	}

	/// Generate all type definitions and functions
	pub fn emit(&mut self) {
		// Initialize linear memory (1 page = 64KB)
		self.memory.memory(MemoryType {
			minimum: 1,
			maximum: None,
			memory64: false,
			shared: false,
			page_size_log2: None,
		});

		// Export memory so it can be accessed from the host
		self.exports.export("memory", ExportKind::Memory, 0);

		self.emit_gc_types();
		self.emit_constructor_functions();
	}

	/// Sanitize a string to create a valid WASM identifier name
	/// - Allow up to 20 characters, but cut at first space after position 10
	/// - Replace remaining spaces and special chars with underscores
	/// - Ensure it starts with a letter or underscore
	fn sanitize_data_name(s: &str) -> String {
		// Take up to 20 chars, but look for natural break point after char 10
		let truncated = if s.len() > 20 {
			// Find first space after position 10
			if let Some(space_pos) = s[10.min(s.len())..20.min(s.len())].find(' ') {
				&s[..10 + space_pos]
			} else {
				&s[..20.min(s.len())]
			}
		} else {
			s
		};

		let mut name = truncated
			.chars()
			.map(|c| if c.is_alphanumeric() { c } else { '_' })
			.collect::<String>();

		// Ensure it starts with letter or underscore
		if name.is_empty() || name.chars().next().unwrap().is_numeric() {
			name.insert(0, '_');
		}

		name
	}

	/// Allocate a string in linear memory and return its offset
	/// Returns (ptr, len) tuple
	fn allocate_string(&mut self, s: &str) -> (u32, u32) {
		if let Some(&offset) = self.string_table.get(s) {
			return (offset, s.len() as u32);
		}

		let offset = self.next_data_offset;
		let bytes = s.as_bytes();

		// Generate a sanitized name for this data segment
		let data_name = Self::sanitize_data_name(s);
		let data_idx = self.next_data_segment_idx;

		// Add string data to data section (passive data, offset 0, memory index 0)
		self.data.active(
			0,                                    // memory index
			&ConstExpr::i32_const(offset as i32), // offset expression
			bytes.iter().copied(),
		);

		// Track the data segment name for the name section
		self.data_segment_names.push((data_idx, data_name));
		self.next_data_segment_idx += 1;

		self.string_table.insert(s.to_string(), offset);
		self.next_data_offset += bytes.len() as u32;

		(offset, bytes.len() as u32)
	}

	/// Define GC struct types for Node variants
	fn emit_gc_types(&mut self) {
		// First, define the unified Node struct type per the design spec

		// Save the index where we'll define the node struct
		let node_type_idx = self.next_type_idx;
		self.next_type_idx += 1;

		// For recursive refs, use the type index that will be assigned
		let node_ref = RefType {
			nullable: true,
			heap_type: HeapType::Concrete(node_type_idx),
		};

		self.types.ty().struct_(vec![
			// TODO Node header? what for it's TYPED! todo ValType::I64 in 64bit wasm?
			FieldType {
				element_type: Val(ValType::I32),
				mutable: false,
			}, // name_ptr // TODO rename as text / string
			FieldType {
				element_type: Val(ValType::I32),
				mutable: false,
			}, // name_len
			FieldType {
				element_type: Val(ValType::I32),
				mutable: false,
			}, // kind // TODO move to top
			FieldType {
				element_type: Val(ValType::I64),
				mutable: false,
			}, // int_value // TODO  merge with float_value via cast
			FieldType {
				element_type: Val(ValType::F64),
				mutable: false,
			}, // float_value
			FieldType {
				element_type: Val(ValType::I32),
				mutable: false,
			}, // text_ptr // TODO  merge with name_ptr/name_len !
			FieldType {
				element_type: Val(ValType::I32),
				mutable: false,
			}, // text_len
			FieldType {
				element_type: Val(Ref(node_ref)),
				mutable: false,
			}, // left // TODO rename as child
			FieldType {
				element_type: Val(Ref(node_ref)),
				mutable: false,
			}, // right // TODO rename as next / value
			FieldType {
				element_type: Val(Ref(node_ref)),
				mutable: false,
			}, // meta // TODO REMOVE (use BETWEEN left/right)
			   // TODO REMOVE   node -> kind:META -> child ! BETWEEN!
		]);
		self.node_base_type = node_type_idx;

		// Define node array type: array of (ref null node)
		let node_ref = RefType {
			nullable: true,
			heap_type: HeapType::Concrete(self.node_base_type),
		};
		let storage_type = Val(Ref(node_ref));
		self.types.ty().array(&storage_type, true);
		self.node_array_type = self.next_type_idx;
		self.next_type_idx += 1;
	}

	/// Get descriptors for all node constructor functions
	fn get_node_constructors(&self) -> Vec<NodeConstructor> {
		use FieldValue::*;

		let node_ref_type = Ref(RefType {
			nullable: true,
			heap_type: HeapType::Concrete(self.node_base_type),
		});

		vec![
			// new_empty() -> (ref node)
			NodeConstructor {
				export_name: "new_empty",
				params: vec![],
				fields: [
					Zero,
					Zero,
					KindField(NodeKind::Empty),
					I64Zero,
					FloatZero,
					Zero,
					Zero,
					Null,
					Null,
					Null,
				],
			},
			// new_int(i64) -> (ref node)
			NodeConstructor {
				export_name: "new_int",
				params: vec![ValType::I64],
				fields: [
					Zero,
					Zero,
					KindField(NodeKind::Number),
					LocalI64(0),
					FloatZero,
					Zero,
					Zero,
					Null,
					Null,
					Null,
				],
			},
			// new_float(f64) -> (ref node)
			NodeConstructor {
				export_name: "new_float",
				params: vec![ValType::F64],
				fields: [
					Zero,
					Zero,
					KindField(NodeKind::Number),
					I64Zero,
					LocalF64(0),
					Zero,
					Zero,
					Null,
					Null,
					Null,
				],
			},
			// new_codepoint(i32) -> (ref node)
			NodeConstructor {
				export_name: "new_codepoint",
				params: vec![ValType::I32],
				fields: [
					Zero,
					Zero,
					KindField(NodeKind::Codepoint),
					LocalI32AsI64(0),
					FloatZero, // Store codepoint in int_value field as i64
					Zero,
					Zero,
					Null,
					Null,
					Null,
				],
			},
			// new_text(name*: i32, len: i32) -> (ref node)
			NodeConstructor {
				export_name: "new_text",
				params: vec![ValType::I32, ValType::I32],
				fields: [
					Zero,
					Zero,
					KindField(NodeKind::Text),
					I64Zero,
					FloatZero,
					LocalI32(0),
					LocalI32(1),
					Null,
					Null,
					Null,
				],
			},
			// new_symbol(name*: i32, len: i32) -> (ref node)
			NodeConstructor {
				export_name: "new_symbol",
				params: vec![ValType::I32, ValType::I32],
				fields: [
					Zero,
					Zero,
					KindField(NodeKind::Symbol),
					I64Zero,
					FloatZero,
					LocalI32(0),
					LocalI32(1),
					Null,
					Null,
					Null,
				],
			},
			// new_tag(name_ptr: i32, name_len: i32, params: ref node, body: ref node) -> (ref node)
			NodeConstructor {
				export_name: "new_tag",
				params: vec![ValType::I32, ValType::I32, node_ref_type, node_ref_type],
				fields: [
					LocalI32(0),
					LocalI32(1),
					KindField(NodeKind::Tag),
					I64Zero,
					FloatZero,
					Zero,
					Zero,
					LocalRef(2),
					LocalRef(3),
					Null,
				],
			},
			// new_pair(left: ref node, right: ref node) -> (ref node)
			NodeConstructor {
				export_name: "new_pair",
				params: vec![node_ref_type, node_ref_type],
				fields: [
					Zero,
					Zero,
					KindField(NodeKind::Pair),
					I64Zero,
					FloatZero,
					Zero,
					Zero,
					LocalRef(0),
					LocalRef(1),
					Null,
				],
			},
			// new_keyvalue(key_ptr: i32, key_len: i32, value: ref node) -> (ref node)
			NodeConstructor {
				export_name: "new_keyvalue",
				params: vec![ValType::I32, ValType::I32, node_ref_type],
				fields: [
					LocalI32(0),
					LocalI32(1),
					KindField(NodeKind::Key),
					I64Zero,
					FloatZero,
					Zero,
					Zero,
					Null,
					LocalRef(2),
					Null,
				],
			},
		]
	}

	/// Parametrized constructor emitter - replaces all individual emit_new_node_* methods
	fn emit_node_constructor(&mut self, desc: &NodeConstructor) -> u32 {
		// 1. Register function signature
		let node_ref = RefType {
			nullable: false,
			heap_type: HeapType::Concrete(self.node_base_type),
		};
		let func_type = self.types.len();
		self.types
			.ty()
			.function(desc.params.clone(), vec![Ref(node_ref)]);
		self.functions.function(func_type);

		// 2. Create function body (no additional locals needed beyond parameters)
		let mut func = Function::new(vec![]);

		// 3. Emit all 10 field values in order based on descriptor
		for field_value in &desc.fields {
			match field_value {
				FieldValue::Zero => {
					func.instruction(&I32Const(0));
				}
				FieldValue::I64Zero => {
					func.instruction(&Instruction::I64Const(0));
				}
				FieldValue::FloatZero => {
					func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits())));
				}
				FieldValue::Null => {
					self.emit_node_null(&mut func);
				}
				FieldValue::LocalI32(idx)
				| FieldValue::LocalI64(idx)
				| FieldValue::LocalF64(idx)
				| FieldValue::LocalRef(idx) => {
					func.instruction(&Instruction::LocalGet(*idx));
				}
				FieldValue::LocalI32AsI64(idx) => {
					func.instruction(&Instruction::LocalGet(*idx));
					func.instruction(&Instruction::I64ExtendI32U);
				}
				FieldValue::KindField(kind) => {
					func.instruction(&I32Const(*kind as i32));
				}
			}
		}

		// 4. Create struct and return
		func.instruction(&Instruction::StructNew(self.node_base_type));
		func.instruction(&Instruction::End);

		// 5. Register code and export
		self.code.function(&func);
		let fn_idx = self.next_func_idx;
		self.exports
			.export(desc.export_name, ExportKind::Func, fn_idx);
		self.next_func_idx += 1;

		fn_idx
	}

	/// Emit constructor functions for creating Node instances using unified struct
	fn emit_constructor_functions(&mut self) {
		let constructors = self.get_node_constructors();

		for desc in &constructors {
			let fn_idx = self.emit_node_constructor(desc);

			// Save function index for later use in emit_node_instructions
			self.function_indices.insert(desc.export_name, fn_idx);
		}

		self.emit_get_node_kind();
		self.emit_node_field_getters();
	}

	/// Helper: Emit all 10 node fields and StructNew instruction (empty node pattern)
	fn emit_empty_node(&self, func: &mut Function, kind: NodeKind) {
		func.instruction(&I32Const(0)); // name_ptr
		func.instruction(&I32Const(0)); // name_len
		func.instruction(&I32Const(kind as i32)); // kind
		func.instruction(&Instruction::I64Const(0)); // int_value
		func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits()))); // float_value
		func.instruction(&I32Const(0)); // text_ptr
		func.instruction(&I32Const(0)); // text_len
		self.emit_node_null(func); // left
		self.emit_node_null(func); // right
		self.emit_node_null(func); // meta
		func.instruction(&Instruction::StructNew(self.node_base_type));
	}

	/// Helper: Emit node with text in text_ptr/text_len fields
	fn emit_text_node_inline(&self, func: &mut Function, kind: NodeKind, s: &str) {
		let (ptr, len) = self
			.string_table
			.get(s)
			.map(|&offset| (offset, s.len() as u32))
			.unwrap_or((0, s.len() as u32));
		func.instruction(&I32Const(0)); // name_ptr
		func.instruction(&I32Const(0)); // name_len
		func.instruction(&I32Const(kind as i32)); // kind
		func.instruction(&Instruction::I64Const(0)); // int_value
		func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits()))); // float_value
		func.instruction(&I32Const(ptr as i32)); // text_ptr
		func.instruction(&I32Const(len as i32)); // text_len
		self.emit_node_null(func); // left
		self.emit_node_null(func); // right
		self.emit_node_null(func); // meta
		func.instruction(&Instruction::StructNew(self.node_base_type));
	}

	/// Helper: Emit node with name in name_ptr/name_len fields
	fn emit_named_node_inline(&self, func: &mut Function, kind: NodeKind, name: &str) {
		let (ptr, len) = self
			.string_table
			.get(name)
			.map(|&offset| (offset, name.len() as u32))
			.unwrap_or((0, name.len() as u32));
		func.instruction(&I32Const(ptr as i32)); // name_ptr
		func.instruction(&I32Const(len as i32)); // name_len
		func.instruction(&I32Const(kind as i32)); // kind
		func.instruction(&Instruction::I64Const(0)); // int_value
		func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits()))); // float_value
		func.instruction(&I32Const(0)); // text_ptr
		func.instruction(&I32Const(0)); // text_len
		self.emit_node_null(func); // left
		self.emit_node_null(func); // right
		self.emit_node_null(func); // meta
		func.instruction(&Instruction::StructNew(self.node_base_type));
	}

	/// Emit get_node_kind(ref node) -> i32 (returns tag field)
	fn emit_get_node_kind(&mut self) {
		let node_ref_nullable = RefType {
			nullable: true,
			heap_type: HeapType::Concrete(self.node_base_type),
		};
		let func_type = self.types.len();
		self.types
			.ty()
			.function(vec![Ref(node_ref_nullable)], vec![ValType::I32]);
		self.functions.function(func_type);

		let mut func = Function::new(vec![]);
		func.instruction(&Instruction::LocalGet(0));
		func.instruction(&Instruction::StructGet {
			struct_type_index: self.node_base_type,
			field_index: 2, // tag field is at indekind
		});
		func.instruction(&Instruction::End);

		self.code.function(&func);
		self.exports
			.export("get_node_kind", ExportKind::Func, self.next_func_idx);
		self.next_func_idx += 1;
	}

	/// Emit new_text(name*: i32, len: i32) -> (ref node)

	/// Emit new_symbol(name*: i32, len: i32) -> (ref node)

	/// Emit getter functions for unified node struct fields
	fn emit_node_field_getters(&mut self) {
		let node_ref = RefType {
			nullable: false,
			heap_type: HeapType::Concrete(self.node_base_type),
		};

		// get_tag(node) -> i32
		let func_type = self.types.len();
		self.types
			.ty()
			.function(vec![Ref(node_ref)], vec![ValType::I32]);
		self.functions.function(func_type);
		let mut func = Function::new(vec![]);
		func.instruction(&Instruction::LocalGet(0));
		func.instruction(&Instruction::StructGet {
			struct_type_index: self.node_base_type,
			field_index: 2, // tag fikind
		});
		func.instruction(&Instruction::End);
		self.code.function(&func);
		self.exports
			.export("get_tag", ExportKind::Func, self.next_func_idx);
		self.next_func_idx += 1;

		// get_int_value(node) -> i64
		let func_type = self.types.len();
		self.types
			.ty()
			.function(vec![Ref(node_ref)], vec![ValType::I64]);
		self.functions.function(func_type);
		let mut func = Function::new(vec![]);
		func.instruction(&Instruction::LocalGet(0));
		func.instruction(&Instruction::StructGet {
			struct_type_index: self.node_base_type,
			field_index: 3, // int_value field
		});
		func.instruction(&Instruction::End);
		self.code.function(&func);
		self.exports
			.export("get_int_value", ExportKind::Func, self.next_func_idx);
		self.next_func_idx += 1;

		// get_float_value(node) -> f64
		let func_type = self.types.len();
		self.types
			.ty()
			.function(vec![Ref(node_ref)], vec![ValType::F64]);
		self.functions.function(func_type);
		let mut func = Function::new(vec![]);
		func.instruction(&Instruction::LocalGet(0));
		func.instruction(&Instruction::StructGet {
			struct_type_index: self.node_base_type,
			field_index: 4, // float_value field
		});
		func.instruction(&Instruction::End);
		self.code.function(&func);
		self.exports
			.export("get_float_value", ExportKind::Func, self.next_func_idx);
		self.next_func_idx += 1;

		// get_name_len(node) -> i32
		let func_type = self.types.len();
		self.types
			.ty()
			.function(vec![Ref(node_ref)], vec![ValType::I32]);
		self.functions.function(func_type);
		let mut func = Function::new(vec![]);
		func.instruction(&Instruction::LocalGet(0));
		func.instruction(&Instruction::StructGet {
			struct_type_index: self.node_base_type,
			field_index: 1, // name_len field
		});
		func.instruction(&Instruction::End);
		self.code.function(&func);
		self.exports
			.export("get_name_len", ExportKind::Func, self.next_func_idx);
		self.next_func_idx += 1;
	}

	/// Emit a function that constructs and returns a specific Node
	pub fn emit_node_main(&mut self, node: &Node) {
		// Pre-allocate all strings in the node tree
		self.collect_and_allocate_strings(node);

		// Use the unified Node struct type
		let node_ref = RefType {
			nullable: false,
			heap_type: HeapType::Concrete(self.node_base_type),
		};

		let func_type = self.types.len();
		self.types.ty().function(vec![], vec![Ref(node_ref)]);
		self.functions.function(func_type);

		let mut func = Function::new(vec![]);

		// Encode the node into the unified struct format
		self.emit_node_instructions(&mut func, node);

		func.instruction(&Instruction::End);

		self.code.function(&func);
		self.exports
			.export("main", ExportKind::Func, self.next_func_idx);
		self.next_func_idx += 1;
	}

	/// Recursively collect and allocate all strings from a node tree
	fn collect_and_allocate_strings(&mut self, node: &Node) {
		let node = node.unwrap_meta();
		match node {
			Node::Text(s) | Node::Symbol(s) => {
				self.allocate_string(s);
			}
			Node::Key(key, value) => {
				self.allocate_string(key);
				self.collect_and_allocate_strings(value);
			}
			Node::Pair(left, right) => {
				self.collect_and_allocate_strings(left);
				self.collect_and_allocate_strings(right);
			}
			Node::List(items, _, _) => {
				for item in items {
					self.collect_and_allocate_strings(item);
				}
			}
			Node::Data(dada) => {
				self.allocate_string(&dada.type_name);
			}
			_ => {}
		}
	}

	fn emit_node_null(&self, func: &mut Function) {
		func.instruction(&Instruction::RefNull(HeapType::Concrete(
			self.node_base_type,
		)));
	}

	/// Emit WASM instructions to construct a Node in the unified struct format
	fn emit_node_instructions(&self, func: &mut Function, node: &Node) {
		// Unwrap metadata if present
		let node = node.unwrap_meta();

		match node {
			Node::Empty => {
				// self.emit_node_null(func);
				// expected (ref $type), found (ref null $type) currently not nullable
				// self.emit_empty_node(func, NodeKind::Empty);
				func.instruction(&Instruction::Call(self.function_indices["new_empty"]));
			}
			Node::Number(num) => {
				match num {
					Number::Int(i) => {
						func.instruction(&Instruction::I64Const(*i));
						func.instruction(&Instruction::Call(self.function_indices["new_int"]));
					}
					Number::Float(f) => {
						func.instruction(&Instruction::F64Const(Ieee64::new(f.to_bits())));
						func.instruction(&Instruction::Call(self.function_indices["new_float"]));
					}
					_ => {
						// Quotient, Complex not yet supported - emit empty node
						func.instruction(&Instruction::Call(self.function_indices["new_empty"]));
					}
				}
			}
			Node::Text(s) => {
				let (ptr, len) = self
					.string_table
					.get(s.as_str())
					.map(|&offset| (offset, s.len() as u32))
					.unwrap_or((0, s.len() as u32));
				func.instruction(&I32Const(ptr as i32));
				func.instruction(&I32Const(len as i32));
				func.instruction(&Instruction::Call(self.function_indices["new_text"]));
			}
			Node::Char(c) => {
				func.instruction(&I32Const(*c as i32));
				func.instruction(&Instruction::Call(self.function_indices["new_codepoint"]));
			}
			Node::Symbol(s) => {
				let (ptr, len) = self
					.string_table
					.get(s.as_str())
					.map(|&offset| (offset, s.len() as u32))
					.unwrap_or((0, s.len() as u32));
				func.instruction(&I32Const(ptr as i32));
				func.instruction(&I32Const(len as i32));
				func.instruction(&Instruction::Call(self.function_indices["new_symbol"]));
			}
			Node::Key(key, value) => {
				let (ptr, len) = self
					.string_table
					.get(key.as_str())
					.map(|&offset| (offset, key.len() as u32))
					.unwrap_or((0, key.len() as u32));
				func.instruction(&I32Const(ptr as i32));
				func.instruction(&I32Const(len as i32));
				self.emit_node_instructions(func, value);
				func.instruction(&Instruction::Call(self.function_indices["new_keyvalue"]));
			}
			Node::Pair(_left, _right) => {
				self.emit_node_instructions(func, _left);
				self.emit_node_instructions(func, _right);
				func.instruction(&Instruction::Call(self.function_indices["new_pair"]));
			}
			Node::List(items, bracket, _separator) => {
				// Special case: single-item lists emit the item directly
				if items.len() == 1 {
					self.emit_node_instructions(func, &items[0]);
					return;
				}
				// Empty list
				if items.is_empty() {
					func.instruction(&Instruction::Call(self.function_indices["new_empty"]));
					return;
				}
				// name_ptr, name_len
				func.instruction(&I32Const(0));
				func.instruction(&I32Const(0));
				// kind - Curly brackets map to Block kind, others to List kind
				let kind = match bracket {
					Bracket::Curly => NodeKind::Block,
					_ => NodeKind::List,
				};
				func.instruction(&I32Const(kind as i32));
				// int_value (store bracket info), float_value
				let bracket_val = match bracket {
					Bracket::Curly => 0i64,
					Bracket::Square => 1,
					Bracket::Round => 2,
				Bracket::Less => 5,
					Bracket::Other(_, _) => 3,
					Bracket::None => 4,
				};
				func.instruction(&Instruction::I64Const(bracket_val));
				func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits())));
				// text_ptr (item count), text_len
				func.instruction(&I32Const(items.len() as i32));
				func.instruction(&I32Const(0));
				// Encode items as linked list: left=first, right=rest
				// Emit first item
				self.emit_node_instructions(func, &items[0]);
				// Emit rest as a List or null if no more items
				if items.len() > 2 {
					// More than 2 items: create nested List with remaining items
					let rest = Node::List(items[1..].to_vec(), bracket.clone(), _separator.clone());
					self.emit_node_instructions(func, &rest);
				} else if items.len() == 2 {
					// Exactly 2 items: emit second item directly
					self.emit_node_instructions(func, &items[1]);
				} else {
					// Should never happen (len must be >= 2 here)
					self.emit_node_null(func);
				}
				// meta
				self.emit_node_null(func);
				func.instruction(&Instruction::StructNew(self.node_base_type));
			}
			Node::Data(dada) => {
				// name_ptr, name_len (store type_name - use actual allocated string)
				let (ptr, len) = self
					.string_table
					.get(dada.type_name.as_str())
					.map(|&offset| (offset, dada.type_name.len() as u32))
					.unwrap_or((0, dada.type_name.len() as u32));
				func.instruction(&I32Const(ptr as i32));
				func.instruction(&I32Const(len as i32));
				// kind
				func.instruction(&I32Const(NodeKind::Data as i32));
				// int_value (store data_type), float_value
				let data_type_val = match &dada.data_type {
					DataType::Vec => 0i64,
					DataType::Tuple => 1,
					DataType::Struct => 2,
					DataType::Primitive => 3,
					DataType::String => 4,
					DataType::Other => 5,
					DataType::Reference => 6,
					_ => todo!("Unhandled DataType variant"),
				};
				func.instruction(&Instruction::I64Const(data_type_val));
				func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits())));
				// text_ptr, text_len
				func.instruction(&I32Const(0));
				func.instruction(&I32Const(0));
				// left, right, meta
				self.emit_node_null(func);
				self.emit_node_null(func);
				self.emit_node_null(func);
				func.instruction(&Instruction::StructNew(self.node_base_type));
			}
			Node::Meta { .. } => {
				// Should not reach here since unwrap_meta is called at the start
				func.instruction(&I32Const(0));
				func.instruction(&I32Const(0));
				func.instruction(&I32Const(NodeKind::Empty as i32));
				func.instruction(&Instruction::I64Const(0));
				func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits())));
				func.instruction(&I32Const(0));
				func.instruction(&I32Const(0));
				self.emit_node_null(func);
				self.emit_node_null(func);
				self.emit_node_null(func);
				func.instruction(&Instruction::StructNew(self.node_base_type));
			}
			Node::Error(_) => {
				warn!("Unhandled Error {}", node);
				// panic!("Unhandled Error {}", node);
			}
			&Node::False => {
				func.instruction(&I32Const(0));
			}
			&Node::True => {
				func.instruction(&I32Const(1));
			}
			_ => todo!(),
		}
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

	/// Generate the final WASM module bytes
	pub fn finish(mut self) -> Vec<u8> {
		self.module.section(&self.types);
		self.module.section(&self.functions);
		self.module.section(&self.memory); // Memory section before code
		self.module.section(&self.exports);
		self.module.section(&self.code);
		self.module.section(&self.data); // Data section after code

		// Add comprehensive names for debugging
		self.emit_names();
		self.module.section(&self.names);

		let bytes = self.module.finish();

		// Save to file
		std::fs::write("test.wasm", &bytes).expect("Failed to write test.wasm");
		Self::validate_wasm(&bytes);
		bytes
	}

	/// Emit comprehensive WASM names for types, functions, and fields
	fn emit_names(&mut self) {
		// Module name
		self.names.module("wasp_node_ast");

		// Type names
		let mut type_names = NameMap::new();
		type_names.append(0, "node");
		type_names.append(1, "node_array");
		// Function signature types (created by emit_node_constructor and emit_node_field_getters)
		type_names.append(2, "func_new_empty");
		type_names.append(3, "func_new_int");
		type_names.append(4, "func_new_float");
		type_names.append(5, "func_new_codepoint");
		type_names.append(6, "func_new_text");
		type_names.append(7, "func_new_symbol");
		type_names.append(8, "func_new_tag");
		type_names.append(9, "func_new_pair");
		type_names.append(10, "func_new_keyvalue");
		type_names.append(11, "func_get_node_kind");
		type_names.append(12, "func_get_tag");
		type_names.append(13, "func_get_int_value");
		type_names.append(14, "func_get_float_value");
		type_names.append(15, "func_get_name_len");
		type_names.append(16, "func_main");
		self.names.types(&type_names);

		// Field names for the unified node struct
		let mut field_names = NameMap::new();
		field_names.append(0, "name_ptr");
		field_names.append(1, "name_len");
		field_names.append(2, "tag");
		field_names.append(3, "int_value");
		field_names.append(4, "float_value");
		field_names.append(5, "text_ptr");
		field_names.append(6, "text_len");
		field_names.append(7, "left");
		field_names.append(8, "right");
		field_names.append(9, "meta");

		let mut type_field_names = IndirectNameMap::new();
		type_field_names.append(self.node_base_type, &field_names);
		self.names.fields(&type_field_names);

		// Function names
		let mut func_names = NameMap::new();
		//
		func_names.append(0, "new_empty");
		func_names.append(1, "new_int");
		func_names.append(2, "new_float");
		func_names.append(3, "new_codepoint");
		func_names.append(4, "new_text");
		func_names.append(5, "new_symbol");
		func_names.append(6, "new_tag");
		func_names.append(7, "new_pair");
		func_names.append(8, "new_keyvalue");
		func_names.append(9, "get_node_kind");
		func_names.append(10, "get_tag");
		func_names.append(11, "get_int_value");
		func_names.append(12, "get_float_value");
		func_names.append(13, "get_name_len");
		func_names.append(14, "main");
		self.names.functions(&func_names);

		// Data segment names (based on string content)
		let mut data_names = NameMap::new();
		for (idx, name) in &self.data_segment_names {
			data_names.append(*idx, name);
		}
		self.names.data(&data_names);
	}
}

pub fn eval(code: &str) -> Node {
	let node = WaspParser::parse(code);
	if let Node::Error(e) = &node {
		panic!("Parse error: {}", e);
	}
	let mut emitter = WasmGcEmitter::new();
	emitter.emit();
	emitter.emit_node_main(&node);
	let wasm_bytes = emitter.finish();
	let obj = read_bytes(&wasm_bytes);
	match obj {
		Err(e) => panic!("Failed to read WASM bytes: {}", e),
		Ok(x) => Node::from_gc_object(&x),
	}
}
