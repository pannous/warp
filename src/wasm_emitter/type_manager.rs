//! Type management for WASM GC emitter

use crate::type_kinds::{any_heap_type, FieldDef, TypeDef, TypeRegistry};
use std::collections::HashMap;
use wasm_encoder::*;
use StorageType::Val;
use ValType::Ref;

/// Manages WASM type system: GC types, user-defined types, and type indices
pub struct TypeManager {
	/// Type section for WASM module
	types: TypeSection,

	/// Type index for $String struct
	pub string_type: u32,

	/// Type index for $i64box struct (boxed integers)
	pub i64_box_type: u32,

	/// Type index for $f64box struct (boxed floats)
	pub f64_box_type: u32,

	/// Type index for $Node struct
	pub node_type: u32,

	/// Next available type index
	next_type_idx: u32,

	/// Map from user type names to their WASM type indices
	user_type_indices: HashMap<String, u32>,
}

impl Default for TypeManager {
	fn default() -> Self {
		Self::new()
	}
}

impl TypeManager {
	/// Create a new type manager
	pub fn new() -> Self {
		Self {
			types: TypeSection::new(),
			string_type: 0,
			i64_box_type: 0,
			f64_box_type: 0,
			node_type: 0,
			next_type_idx: 0,
			user_type_indices: HashMap::new(),
		}
	}

	/// Emit core GC types: String, Node, i64box, f64box
	pub fn emit_gc_types(&mut self) {
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

		// Type 1: $Node = (struct (field $kind i64) (field $data anyref) (field $value (ref null $Node)))
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
			self.user_type_indices.insert(type_def.name.clone(), self.next_type_idx);
			self.next_type_idx += 1;
		}
	}

	/// Emit a single user-defined struct type
	pub fn emit_single_user_type(&mut self, type_def: &TypeDef) {
		let fields: Vec<FieldType> = type_def
			.fields
			.iter()
			.map(|f| self.field_def_to_wasm_field(f))
			.collect();

		self.types.ty().struct_(fields);
		self.user_type_indices.insert(type_def.name.clone(), self.next_type_idx);
		self.next_type_idx += 1;
	}

	/// Convert a FieldDef to a WASM FieldType
	pub fn field_def_to_wasm_field(&self, field: &FieldDef) -> FieldType {
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
					panic!("Unknown type: {}", other);
				}
			}
		};

		FieldType {
			element_type,
			mutable: false, // Fields are immutable by default
		}
	}

	/// Get a RefType for Node with specified nullability
	pub fn node_ref(&self, nullable: bool) -> RefType {
		RefType {
			nullable,
			heap_type: HeapType::Concrete(self.node_type),
		}
	}

	/// Get the WASM type index for a user-defined type
	pub fn get_user_type_idx(&self, name: &str) -> Option<u32> {
		self.user_type_indices.get(name).copied()
	}

	/// Add a function type and return its index
	pub fn add_function_type(&mut self, params: Vec<ValType>, results: Vec<ValType>) -> u32 {
		let idx = self.next_type_idx;
		self.types.ty().function(params, results);
		self.next_type_idx += 1;
		idx
	}

	/// Get the type section (for adding to WASM module)
	pub fn types(&self) -> &TypeSection {
		&self.types
	}

	/// Get mutable access to the type section
	pub fn types_mut(&mut self) -> &mut TypeSection {
		&mut self.types
	}

	/// Get the current type count (for creating new type indices)
	pub fn len(&self) -> u32 {
		self.types.len()
	}

	/// Get the next type index
	pub fn next_type_idx(&self) -> u32 {
		self.next_type_idx
	}

	/// Get mutable access to user type indices (for compatibility)
	pub fn user_type_indices_mut(&mut self) -> &mut HashMap<String, u32> {
		&mut self.user_type_indices
	}
}
