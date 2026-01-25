use wasm_encoder::{AbstractHeapType, HeapType, RefType, ValType};
use wasm_encoder::ValType::Ref;
use crate::type_kinds;
use crate::wasm_emitter::WasmGcEmitter;

/// Node type tags for runtime type checking and WASM encoding
/// Compact repr(u8) for efficient storage in WASM GC structs
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Kind {
	#[default]
	Empty = 0,     // void / no value
	Int = 1,       // i64 value (boxed in $i64box)
	Float = 2,     // f64 value (boxed in $f64box)
	Text = 3,      // string (via $String struct)
	Codepoint = 4, // char/i32 as i31ref
	Symbol = 5,    // string (via $String struct)
	Key = 6,       // data=key node, value=value node (also used for pairs)
	Block = 7,     // curly braces {}
	List = 8,      // square brackets []
	Data = 9,      // arbitrary data container
	Meta = 10,     // metadata wrapper
	Error = 11,    // error node
	TypeDef = 12,  // type definition: name + body (fields)
	Pointer = 13,  // FFI pointer (i64 handle)
	Int32 = 14,    // explicit i32 (for FFI)
	Float32 = 15,  // explicit f32 (for FFI)
}

impl Kind {
	/// Check if this is an integer type (WASM i64 local)
	pub fn is_int(&self) -> bool { matches!(self, Kind::Int | Kind::Int32) }

	/// Check if this is a float type (WASM f64 local)
	pub fn is_float(&self) -> bool { matches!(self, Kind::Float | Kind::Float32) }

	/// Check if this is a primitive numeric type (stored as WASM primitive)
	pub fn is_primitive(&self) -> bool {
		matches!(self, Kind::Int | Kind::Int32 | Kind::Float | Kind::Float32 | Kind::Codepoint | Kind::Pointer)
	}

	/// Check if this is a reference type (stored as WASM ref $Node)
	pub fn is_ref(&self) -> bool { !self.is_primitive() }

	/// Check if this is a pointer type (FFI)
	pub fn is_pointer(&self) -> bool { matches!(self, Kind::Pointer | Kind::Text) }

	/// Parse a C type string to Kind with smart defaults
	pub fn from_c_type(s: &str) -> Kind {
		let s = s.trim();

		// Handle pointer types
		if s.contains('*') {
			if s.contains("char") {
				return Kind::Text; // char* is a string
			}
			return Kind::Pointer; // other pointers as i64 handles
		}

		// Strip qualifiers
		let s = s.replace("const ", "").replace("unsigned ", "").replace("signed ", "");
		let s = s.trim();

		match s {
			"void" => Kind::Empty,
			"int" | "int32_t" | "uint32_t" | "Uint32" => Kind::Int32,
			"long" | "long int" | "int64_t" | "uint64_t" | "long long" => Kind::Int,
			"float" => Kind::Float32,
			"double" => Kind::Float,
			"size_t" | "ssize_t" | "ptrdiff_t" => Kind::Int,
			"bool" | "_Bool" => Kind::Int32,
			"short" | "int16_t" | "uint16_t" => Kind::Int32,
			"char" | "int8_t" | "uint8_t" => Kind::Codepoint,
			"Color" => Kind::Int32, // raylib Color is 4 bytes packed
			_ => Kind::Data, // unknown types as generic data
		}
	}
}

impl std::fmt::Display for Kind {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Kind::Empty => write!(f, "empty"),
			Kind::Int => write!(f, "int"),      // wasp uses "int" for i64
			Kind::Int32 => write!(f, "i32"),
			Kind::Float => write!(f, "float"),  // wasp uses "float" for f64
			Kind::Float32 => write!(f, "f32"),
			Kind::Text => write!(f, "text"),    // wasp uses "text" for strings
			Kind::Codepoint => write!(f, "codepoint"),
			Kind::Symbol => write!(f, "symbol"),
			Kind::Key => write!(f, "key"),
			Kind::Block => write!(f, "block"),
			Kind::List => write!(f, "list"),
			Kind::Data => write!(f, "data"),
			Kind::Meta => write!(f, "meta"),
			Kind::Error => write!(f, "error"),
			Kind::TypeDef => write!(f, "typedef"),
			Kind::Pointer => write!(f, "pointer"),
		}
	}
}


/// Alias for backward compatibility
pub type NodeKind = Kind;

/// First tag value for user-defined types (built-ins use 0-255)
pub const USER_TYPE_TAG_START: u32 = 0x10000;

/// Field definition within a type
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDef {
	pub name: String,
	pub type_name: String, // type as string for now, could be TypeRef later
}

/// Definition of a user-defined type
#[derive(Debug, Clone, PartialEq)]
pub struct TypeDef {
	pub name: String,
	pub tag: u32,               // tag value (>= USER_TYPE_TAG_START)
	pub fields: Vec<FieldDef>,
	pub wasm_type_idx: Option<u32>, // WASM GC type index when emitted
}

impl TypeDef {
	/// Extract TypeDef from a parsed class definition Node
	/// Expected structure: Type { name: Symbol("Person"), body: List([Key(name, :, Type), ...]) }
	pub fn from_node(node: &crate::node::Node) -> Option<Self> {
		use crate::node::Node;
		match node.drop_meta() {
			Node::Type { name, body } => {
				let type_name = name.drop_meta().to_string();
				let fields = Self::extract_fields(body);
				Some(TypeDef {
					name: type_name,
					tag: USER_TYPE_TAG_START, // will be assigned by registry
					fields,
					wasm_type_idx: None,
				})
			}
			_ => None,
		}
	}

	fn extract_fields(body: &crate::node::Node) -> Vec<FieldDef> {
		use crate::node::Node;
		let mut fields = Vec::new();

		// body is List([Key(field_name, op, Type), ...], bracket, separator)
		let items = match body.drop_meta() {
			Node::List(items, _, _) => items,
			_ => return fields,
		};

		for item in items.iter() {
			// Key(key_node, op, value_node) tuple struct
			if let Node::Key(key, _op, value) = item.drop_meta() {
				let field_name = key.drop_meta().to_string();
				// value is Type { name: Symbol("String"), body: Empty }
				let type_name = match value.drop_meta() {
					Node::Type { name, .. } => name.drop_meta().to_string(),
					Node::Symbol(s) => s.to_string(),
					other => other.to_string(),
				};
				fields.push(FieldDef { name: field_name, type_name });
			}
		}
		fields
	}
}

/// Extract field values from a class instance Node
/// Instance structure: Key("Person", Colon, List([Key("name", Colon, Text), Key("age", Colon, Number)]))
/// Returns (type_name, field_values) for use with emit_raw_struct
pub fn extract_instance_values(node: &crate::node::Node) -> Option<(String, Vec<RawFieldValue>)> {
	use crate::node::Node;
	use crate::type_kinds::RawFieldValue;
	use crate::extensions::numbers::Number;

	// Instance is Key(TypeName, :, List([Key(field, :, value), ...]))
	match node.drop_meta() {
		Node::Key(type_name, _, body) => {
			let name = type_name.drop_meta().to_string();
			let values = extract_field_values(body);
			Some((name, values))
		}
		_ => None,
	}
}

fn extract_field_values(body: &crate::node::Node) -> Vec<type_kinds::RawFieldValue> {
	use crate::node::Node;
	use crate::type_kinds::RawFieldValue;
	use crate::extensions::numbers::Number;

	let mut values = Vec::new();

	// body is List([Key(field_name, :, value), ...])
	let items = match body.drop_meta() {
		Node::List(items, _, _) => items,
		_ => return values,
	};

	for item in items.iter() {
		if let Node::Key(_field_name, _op, value) = item.drop_meta() {
			let raw_value = match value.drop_meta() {
				Node::Text(s) => RawFieldValue::String(s.to_string()),
				Node::Symbol(s) => RawFieldValue::String(s.to_string()),
				Node::Number(Number::Int(i)) => RawFieldValue::I64(*i),
				Node::Number(Number::Float(f)) => RawFieldValue::F64(*f),
				Node::Char(c) => RawFieldValue::I32(*c as i32),
				Node::True => RawFieldValue::I32(1),
				Node::False => RawFieldValue::I32(0),
				_ => continue, // Skip unsupported types
			};
			values.push(raw_value);
		}
	}
	values
}

/// Registry for user-defined types
/// Maps type names to TypeDef and provides tag allocation
#[derive(Debug, Default)]
pub struct TypeRegistry {
	types: Vec<TypeDef>,
	name_to_idx: std::collections::HashMap<String, usize>,
}

impl TypeRegistry {
	pub fn new() -> Self {
		Self::default()
	}

	/// Register a new type, returns its tag
	pub fn register(&mut self, name: String, fields: Vec<FieldDef>) -> u32 {
		if let Some(&idx) = self.name_to_idx.get(&name) {
			return self.types[idx].tag;
		}
		let tag = USER_TYPE_TAG_START + self.types.len() as u32;
		let idx = self.types.len();
		self.types.push(TypeDef {
			name: name.clone(),
			tag,
			fields,
			wasm_type_idx: None,
		});
		self.name_to_idx.insert(name, idx);
		tag
	}

	/// Look up type by name
	pub fn get_by_name(&self, name: &str) -> Option<&TypeDef> {
		self.name_to_idx.get(name).map(|&idx| &self.types[idx])
	}

	/// Look up type by tag
	pub fn get_by_tag(&self, tag: u32) -> Option<&TypeDef> {
		if tag < USER_TYPE_TAG_START {
			return None;
		}
		let idx = (tag - USER_TYPE_TAG_START) as usize;
		self.types.get(idx)
	}

	/// Check if tag is a user-defined type
	pub fn is_user_type(tag: u32) -> bool {
		tag >= USER_TYPE_TAG_START
	}

	/// Get all registered types
	pub fn types(&self) -> &[TypeDef] {
		&self.types
	}

	/// Set WASM type index for a registered type
	pub fn set_wasm_type_idx(&mut self, name: &str, wasm_idx: u32) {
		if let Some(&idx) = self.name_to_idx.get(name) {
			self.types[idx].wasm_type_idx = Some(wasm_idx);
		}
	}

	/// Register a type from a Type node, extracting name and fields
	/// Returns the assigned tag, or None if the node isn't a valid Type definition
	/// Skips type references (Type nodes with empty body - just type names)
	pub fn register_from_node(&mut self, node: &crate::node::Node) -> Option<u32> {
		use crate::node::Node;
		// Handle Meta wrapper - recursively unwrap
		let node = node.drop_meta();
		if let Node::Type { name, body } = node {
			// Skip type references (empty body) - only register actual definitions
			if matches!(body.drop_meta(), Node::Empty) {
				return None;
			}
			let type_name = match name.drop_meta() {
				Node::Symbol(s) | Node::Text(s) => s.clone(),
				_ => return None,
			};
			let fields = Self::extract_fields(body);
			Some(self.register(type_name, fields))
		} else {
			None
		}
	}

	/// Extract FieldDefs from a type body (typically a List of Key nodes)
	fn extract_fields(body: &crate::node::Node) -> Vec<FieldDef> {
		use crate::node::Node;
		let mut fields = Vec::new();
		// Unwrap any Meta wrappers
		let body = body.drop_meta();
		match body {
			Node::List(items, _, _) => {
				for item in items {
					if let Some(field) = Self::extract_field(item) {
						fields.push(field);
					}
				}
			}
			// Single field without list wrapper
			other => {
				if let Some(field) = Self::extract_field(other) {
					fields.push(field);
				}
			}
		}
		fields
	}

	/// Extract a single FieldDef from a Key node (name:Type)
	fn extract_field(node: &crate::node::Node) -> Option<FieldDef> {
		use crate::node::Node;
		let node = node.drop_meta();
		match node {
			Node::Key(name_node, _, type_node) => {
				let name = match name_node.drop_meta() {
					Node::Symbol(s) | Node::Text(s) => s.clone(),
					_ => return None,
				};
				let type_name = match type_node.drop_meta() {
					Node::Symbol(s) | Node::Text(s) => s.clone(),
					Node::Type { name: type_name_node, .. } => {
						match type_name_node.drop_meta() {
							Node::Symbol(s) | Node::Text(s) => s.clone(),
							_ => "Any".to_string(),
						}
					}
					_ => "Any".to_string(), // default type
				};
				Some(FieldDef { name, type_name })
			}
			_ => None,
		}
	}
}

pub enum AstKind {
	Declaration,
	Expression,
	Statement,
	While,
	For,
	If,
	Function,
	Return,
	Call,
	Parameter,
	Body,
	Assignment,
	Literal,
	Identifier,
}


/// Convert FieldDef to ValType for function parameters
pub fn field_def_to_val_type(field: &FieldDef, emitter: &WasmGcEmitter) -> ValType {
	match field.type_name.as_str() {
		"Int" | "i64" | "long" => ValType::I64,
		"Float" | "f64" | "double" => ValType::F64,
		"i32" | "int" => ValType::I32,
		"f32" | "float" => ValType::F32,
		"Text" | "String" | "string" => Ref(RefType {
			nullable: true,
			heap_type: HeapType::Concrete(emitter.type_manager.string_type),
		}),
		"Node" => Ref(RefType {
			nullable: true,
			heap_type: HeapType::Concrete(emitter.type_manager.node_type),
		}),
		other => {
			if let Some(&type_idx) = emitter.ctx.user_type_indices.get(other) {
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
