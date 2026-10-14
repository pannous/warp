//! How a struct field of a declared type is stored, and the raw values of struct fields
use super::*;

/// How a struct field of a declared type is stored, by the field's type name
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FieldStorage {
	I64,
	I32,
	F64,
	F32,
	/// a `$String` reference
	Text,
	/// any Node: an untyped field (`class contact {name email?}`) and the other builtin type words (`real`, `ints`)
	Node,
	/// a declared type's own struct, or an unknown name
	Named,
}

/// The class a field of a class type names: `Node` of `Node?`
pub fn named_field_type(type_name: &str) -> &str {
	type_name.trim_end_matches(OPTIONAL_SUFFIX)
}

pub fn field_storage(type_name: &str) -> FieldStorage {
	let word = type_name.trim_end_matches(OPTIONAL_SUFFIX);
	let optional = word.len() < type_name.len();
	match word {
		// `age: int?` may be ø: a number has no room for it, a Node does (card optional-int)
		"Int" | "i64" | "long" | "Float" | "f64" | "double" | "i32" | "int" | "f32" | "float" if optional => {
			FieldStorage::Node
		}
		"Int" | "i64" | "long" => FieldStorage::I64,
		"Float" | "f64" | "double" => FieldStorage::F64,
		"i32" | "int" => FieldStorage::I32,
		"f32" | "float" => FieldStorage::F32,
		"Text" | "String" | "string" | "text" => FieldStorage::Text,
		"Node" | UNTYPED_FIELD => FieldStorage::Node,
		_ if crate::analyzer::union_parts(word).is_some() => FieldStorage::Node,
		_ if crate::analyzer::type_word_kind(word).is_some()
			|| crate::analyzer::plural_element_type(word).is_some()
			|| word == "list"
			|| word.starts_with("list of ") =>
		{
			FieldStorage::Node
		}
		_ => FieldStorage::Named,
	}
}

/// Convert FieldDef to ValType for function parameters
pub fn field_def_to_val_type(field: &FieldDef, emitter: &WasmGcEmitter) -> ValType {
	let reference = |heap_type| {
		Ref(RefType {
			nullable: true,
			heap_type,
		})
	};
	match field_storage(&field.type_name) {
		FieldStorage::I64 => ValType::I64,
		FieldStorage::F64 => ValType::F64,
		FieldStorage::I32 => ValType::I32,
		FieldStorage::F32 => ValType::F32,
		FieldStorage::Text => reference(HeapType::Concrete(emitter.type_manager.string_type)),
		FieldStorage::Node => reference(HeapType::Concrete(emitter.type_manager.node_type)),
		FieldStorage::Named => match emitter.ctx.user_type_indices.get(named_field_type(&field.type_name)) {
			Some(&type_idx) => reference(HeapType::Concrete(type_idx)),
			None => reference(any_heap_type()),
		},
	}
}

/// Helper to create abstract heap type refs
pub fn any_heap_type() -> HeapType {
	HeapType::Abstract {
		shared: false,
		ty: AbstractHeapType::Any,
	}
}

/// Raw field values for emit_raw_struct
/// todo we can most likely get rid of this. We already have three different containers for types
#[derive(Debug, Clone)]
pub enum RawFieldValue {
	I64(i64),
	I32(i32),
	F64(f64),
	F32(f32),
	String(String),
}

impl From<i64> for RawFieldValue {
	fn from(v: i64) -> Self {
		RawFieldValue::I64(v)
	}
}

impl From<i32> for RawFieldValue {
	fn from(v: i32) -> Self {
		RawFieldValue::I32(v)
	}
}

impl From<f64> for RawFieldValue {
	fn from(v: f64) -> Self {
		RawFieldValue::F64(v)
	}
}

impl From<f32> for RawFieldValue {
	fn from(v: f32) -> Self {
		RawFieldValue::F32(v)
	}
}

impl From<&str> for RawFieldValue {
	fn from(v: &str) -> Self {
		RawFieldValue::String(v.to_string())
	}
}

impl From<String> for RawFieldValue {
	fn from(v: String) -> Self {
		RawFieldValue::String(v)
	}
}
