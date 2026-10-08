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

	/// $Limbs = (array (mut i32)): BigInt magnitude, little-endian base 2^32
	pub limbs_type: u32,

	/// $BigInt = (struct (field $negative i32) (field $limbs (ref null $Limbs)))
	pub big_int_type: u32,

	/// $Numbers = (array (mut (ref null any))): heap of $BigInt and $Ratio that Int handles index into
	pub big_heap_type: u32,

	/// $Ratio = (struct (field $num anyref) (field $den anyref)): exact non-integer, both Int payloads, see wasm_emitter/exact.rs
	pub ratio_type: u32,

	/// $IntArray = (array (mut i64)): the items of an $IntList, see wasm_emitter/list_dispatch.rs
	pub int_array_type: u32,

	/// $IntList = (struct (field $length (mut i32)) (field $items (mut (ref $IntArray)))): a growable list of ints
	pub int_list_type: u32,

	/// $FloatArray = (array (mut f64)), the items of a $FloatList, the growable list of floats
	pub float_array_type: u32,
	pub float_list_type: u32,

	/// $NodeArray = (array (mut (ref null $Node))), the items of a $NodeList: a list variable of any elements held as an
	/// array (list_dispatch.rs); $NodeList also keeps the kind of the list it stands for (its brackets)
	pub node_array_type: u32,
	pub node_list_type: u32,

	/// $NodeMap = (struct (field $count (mut i32)) (field $keys (mut (ref $NodeArray))) (field $values (mut (ref $NodeArray)))
	/// (field $slots (mut (ref $Limbs)))): a map variable held as a hash table (map_backend.rs)
	pub node_map_type: u32,

	/// Next available type index
	next_type_idx: u32,

	/// Struct fields declared with a type no one defines; the module must not run then
	pub type_errors: Vec<String>,

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
			limbs_type: 0,
			big_int_type: 0,
			big_heap_type: 0,
			ratio_type: 0,
			int_array_type: 0,
			int_list_type: 0,
			float_array_type: 0,
			float_list_type: 0,
			node_array_type: 0,
			node_list_type: 0,
			node_map_type: 0,
			next_type_idx: 0,
			type_errors: Vec::new(),
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
				mutable: true, // mutable for a new field grown in place: objects are references (P200b)
			}, // kind
			FieldType {
				element_type: Val(Ref(any_ref)),
				mutable: true, // mutable for index assignment
			}, // data
			FieldType {
				element_type: Val(Ref(node_ref)),
				mutable: true, // mutable for a field set in place: instances are references (P200)
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

		self.emit_big_int_types();
		(self.int_array_type, self.int_list_type) = self.emit_typed_list_types(ValType::I64);
		(self.float_array_type, self.float_list_type) = self.emit_typed_list_types(ValType::F64);
		self.emit_node_list_types();
		self.emit_node_map_type();
	}

	fn emit_node_map_type(&mut self) {
		let reference = |index: u32| Val(Ref(RefType { nullable: false, heap_type: HeapType::Concrete(index) }));
		self.types.ty().struct_(vec![
			FieldType { element_type: Val(ValType::I32), mutable: true }, // count
			FieldType { element_type: reference(self.node_array_type), mutable: true }, // keys, capacity = their length
			FieldType { element_type: reference(self.node_array_type), mutable: true }, // values
			FieldType { element_type: reference(self.limbs_type), mutable: true }, // slots: entry index or -1
		]);
		self.node_map_type = self.next_type_idx;
		self.next_type_idx += 1;
	}

	/// $NodeArray and $NodeList = (struct (field $length (mut i32)) (field $items (mut (ref $NodeArray))) (field $kind (mut i64)))
	fn emit_node_list_types(&mut self) {
		let element = Ref(self.node_ref(true));
		self.types.ty().array(&Val(element), true);
		let array = self.next_type_idx;
		let items = RefType { nullable: false, heap_type: HeapType::Concrete(array) };
		self.types.ty().struct_(vec![
			FieldType { element_type: Val(ValType::I32), mutable: true }, // length
			FieldType { element_type: Val(Ref(items)), mutable: true }, // items, capacity = their length
			FieldType { element_type: Val(ValType::I64), mutable: true }, // kind of the list node, with its brackets
		]);
		self.next_type_idx += 2;
		(self.node_array_type, self.node_list_type) = (array, array + 1);
	}

	/// The array of `element`s and the growable list holding it: (struct (field $length (mut i32)) (field $items (mut (ref $array))))
	fn emit_typed_list_types(&mut self, element: ValType) -> (u32, u32) {
		self.types.ty().array(&Val(element), true);
		let array = self.next_type_idx;
		let items = RefType { nullable: false, heap_type: HeapType::Concrete(array) };
		self.types.ty().struct_(vec![
			FieldType { element_type: Val(ValType::I32), mutable: true }, // length
			FieldType { element_type: Val(Ref(items)), mutable: true }, // items, capacity = their length
		]);
		self.next_type_idx += 2;
		(array, array + 1)
	}

	/// Types behind unbounded Int, see wasm_emitter/big_int.rs
	fn emit_big_int_types(&mut self) {
		self.types.ty().array(&Val(ValType::I32), true);
		self.limbs_type = self.next_type_idx;
		self.next_type_idx += 1;

		let limbs_ref = RefType { nullable: true, heap_type: HeapType::Concrete(self.limbs_type) };
		self.types.ty().struct_(vec![
			FieldType { element_type: Val(ValType::I32), mutable: false }, // negative
			FieldType { element_type: Val(Ref(limbs_ref)), mutable: false }, // limbs
		]);
		self.big_int_type = self.next_type_idx;
		self.next_type_idx += 1;

		let any_ref = RefType { nullable: true, heap_type: any_heap_type() };
		self.types.ty().array(&Val(Ref(any_ref)), true);
		self.big_heap_type = self.next_type_idx;
		self.next_type_idx += 1;

		self.types.ty().struct_(vec![
			FieldType { element_type: Val(Ref(any_ref)), mutable: false }, // numerator payload
			FieldType { element_type: Val(Ref(any_ref)), mutable: false }, // positive denominator payload
		]);
		self.ratio_type = self.next_type_idx;
		self.next_type_idx += 1;
	}

	/// Emit user-defined struct types from TypeRegistry
	pub fn emit_user_types(&mut self, registry: &TypeRegistry) {
		for type_def in registry.types() {
			let fields: Vec<FieldType> = type_def
				.fields
				.iter()
				.map(|f| self.field_def_to_wasm_field(f, &type_def.name))
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
			.map(|f| self.field_def_to_wasm_field(f, &type_def.name))
			.collect();

		self.types.ty().struct_(fields);
		self.user_type_indices.insert(type_def.name.clone(), self.next_type_idx);
		self.next_type_idx += 1;
	}

	/// Convert a FieldDef to a WASM FieldType; a field of the class itself (`left: Node?` in Node) refers to the index the
	/// class is getting
	pub fn field_def_to_wasm_field(&mut self, field: &FieldDef, class_name: &str) -> FieldType {
		use crate::type_kinds::FieldStorage;
		let reference = |heap_type| Val(Ref(RefType { nullable: true, heap_type }));
		let element_type = match crate::type_kinds::field_storage(&field.type_name) {
			FieldStorage::I64 => Val(ValType::I64),
			FieldStorage::F64 => Val(ValType::F64),
			FieldStorage::I32 => Val(ValType::I32),
			FieldStorage::F32 => Val(ValType::F32),
			FieldStorage::Text => reference(HeapType::Concrete(self.string_type)),
			FieldStorage::Node => reference(HeapType::Concrete(self.node_type)),
			FieldStorage::Named => match self.user_type_indices.get(crate::type_kinds::named_field_type(&field.type_name)) {
				Some(&type_idx) => reference(HeapType::Concrete(type_idx)),
				None if crate::type_kinds::named_field_type(&field.type_name) == class_name => reference(HeapType::Concrete(self.next_type_idx)),
				None => {
					self.type_errors.push(format!("unknown type: {} of field {}", field.type_name, field.name));
					Val(Ref(self.node_ref(true)))
				}
			},
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

	/// Add a struct type and return its index
	pub fn add_struct_type(&mut self, fields: Vec<FieldType>) -> u32 {
		let idx = self.next_type_idx;
		self.types.ty().struct_(fields);
		self.next_type_idx += 1;
		idx
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

	pub fn is_empty(&self) -> bool {
		self.len() == 0
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

