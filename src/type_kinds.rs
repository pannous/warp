use wasm_encoder::{AbstractHeapType, HeapType, RefType, ValType};
use wasm_encoder::ValType::Ref;
use crate::type_kinds;
use crate::wasm_emitter::WasmGcEmitter;

/// A node's kind field holds the `Kind` in its low KIND_BITS bits and extra info above them:
/// the bracket of a list (Curly=0, Square=1, Round=2 …) or the operator of a key
pub const KIND_BITS: i64 = 8;
pub const KIND_MASK: i64 = 0xFF;
pub const CURLY_BRACKET_INFO: i64 = 0;
pub const SQUARE_BRACKET_INFO: i64 = 1;
pub const CURLY_LIST_KIND: i64 = (CURLY_BRACKET_INFO << KIND_BITS) | Kind::List as i64;
pub const SQUARE_LIST_KIND: i64 = (SQUARE_BRACKET_INFO << KIND_BITS) | Kind::List as i64;
/// true/false: a shallow type of its own (user, card bool-type), an Int 1/0 marked above the kind bits, so everything
/// that reads the kind masked takes it as the Int it is, and printing, `type` and reading back see a bool
pub const BOOL_INFO: i64 = 1;
pub const BOOL_KIND: i64 = (BOOL_INFO << KIND_BITS) | Kind::Int as i64;
/// The bit of a bool in the kind masks of run-time type tests (node_kind_in): no Kind is that high
pub const BOOL_MASK_BIT: i64 = 62;
/// P215: the head cell of a declared list carries the run-time kinds its element type admits, its element mark, in the
/// kind field above the bracket byte; everything that reads the bracket masks it with KIND_MASK
pub const ELEMENT_MARK_SHIFT: i64 = 16;
/// A mark holds the kind mask's bits of every Kind, room for new kinds included, then the bool bit
const MARK_KIND_BITS: i64 = 24;
const _: () = assert!((Kind::Function as i64) < MARK_KIND_BITS, "every Kind has its bit in the element mark");
pub const MARK_KINDS_MASK: i64 = (1 << MARK_KIND_BITS) - 1;
pub const MARK_BOOL_BIT: i64 = MARK_KIND_BITS;
/// A kind field without its element mark, for the comparisons of whole kinds (bracket and kind)
pub const UNMARKED_KIND: i64 = (1 << ELEMENT_MARK_SHIFT) - 1;
const _: () = assert!(MARK_BOOL_BIT + ELEMENT_MARK_SHIFT < 63, "the element mark fits in the kind field");

/// The element mark of a node_kind_in mask, shifted to its place in the kind field
pub fn element_mark(mask: i64) -> i64 {
	let compact = (mask & MARK_KINDS_MASK) | ((mask >> BOOL_MASK_BIT) & 1) << MARK_BOOL_BIT;
	compact << ELEMENT_MARK_SHIFT
}

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
	Function = 16, // closure: data = $Closure struct (typed function reference + captured values), value = name symbol
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
			Kind::Int => write!(f, "int"),      // warp uses "int" for i64
			Kind::Int32 => write!(f, "i32"),
			Kind::Float => write!(f, "float"),  // warp uses "float" for f64
			Kind::Float32 => write!(f, "f32"),
			Kind::Text => write!(f, "text"),    // warp uses "text" for strings
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
			Kind::Function => write!(f, "function"),
		}
	}
}

/// Number type aliases, resolved in one place: literals are exact by default, IEEE 754 is opt-in.
/// `exact` (alias `real`: strictly the rationals ℚ for now, √2 is not exact) and `float` (aliases `fast`, `f64`, `float64`, `double`)
pub fn canonical_type_name(name: &str) -> &str {
	match name {
		"exact" | "real" | "rational" => "exact",
		"float" | "fast" | "f64" | "double" | "float64" => "float",
		other => other,
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

const OPTIONAL_SUFFIX: char = '?';
pub const UNTYPED_FIELD: &str = "any";

impl FieldDef {
	/// A field written as a bare word, `name`, optional as `email?`
	pub fn untyped(word: &str) -> FieldDef {
		let type_name = if word.ends_with(OPTIONAL_SUFFIX) { format!("{UNTYPED_FIELD}{OPTIONAL_SUFFIX}") } else { UNTYPED_FIELD.to_string() };
		FieldDef { name: word.trim_end_matches(OPTIONAL_SUFFIX).to_string(), type_name }
	}

	/// A field declared `email?` or `x:int?` may be left out of the constructor call (it is then ø)
	pub fn is_optional(&self) -> bool {
		self.type_name.ends_with(OPTIONAL_SUFFIX)
	}
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
			} else if let Node::Symbol(word) = item.drop_meta() {
				fields.push(FieldDef::untyped(word));
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
	/// (type, field) pairs declared with a default value, `class p{x=0}` or `x:int=0`: a construction may leave them out
	defaults: std::collections::HashMap<(String, String), crate::node::Node>,
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

	/// A construction `T{…}` must give this field: neither optional (`email?`) nor declared with a default
	pub fn is_required(&self, type_def: &TypeDef, field: &FieldDef) -> bool {
		!field.is_optional() && self.default_of(type_def, field).is_none()
	}

	pub fn default_of(&self, type_def: &TypeDef, field: &FieldDef) -> Option<&crate::node::Node> {
		self.defaults.get(&(type_def.name.clone(), field.name.clone()))
	}

	fn field_items(body: &crate::node::Node) -> impl Iterator<Item = &crate::node::Node> {
		use crate::node::Node;
		match body.drop_meta() {
			Node::List(items, _, _) => items.iter().collect::<Vec<_>>().into_iter(),
			other => vec![other].into_iter(),
		}
	}

	/// The name and default value of a field declared `x=0`, `x:int=0` or `x:0` (a literal after the colon is no type)
	fn field_default(item: &crate::node::Node) -> Option<(String, crate::node::Node)> {
		use crate::node::Node;
		use crate::operators::Op;
		match item.drop_meta() {
			Node::Key(target, Op::Assign, value) => match target.drop_meta() {
				Node::Key(name, Op::Colon, _) => Some((name.drop_meta().to_string(), value.drop_meta().clone())),
				name => Some((name.to_string(), value.drop_meta().clone())),
			},
			Node::Key(name, Op::Colon, value) if matches!(value.drop_meta(), Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::True | Node::False) => {
				Some((name.drop_meta().to_string(), value.drop_meta().clone()))
			}
			_ => None,
		}
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
			for (field, value) in Self::field_items(body).filter_map(Self::field_default) {
				self.defaults.insert((type_name.clone(), field), value);
			}
			Some(self.register(type_name, fields))
		} else {
			None
		}
	}

	/// Extract FieldDefs from a type body (typically a List of Key nodes)
	fn extract_fields(body: &crate::node::Node) -> Vec<FieldDef> {
		use crate::node::Node;
		let mut fields = Vec::new();
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
			// `x:int=0`: the typed field x with a default
			Node::Key(target, crate::operators::Op::Assign, _) if matches!(target.drop_meta(), Node::Key(_, crate::operators::Op::Colon, _)) => {
				Self::extract_field(target)
			}
			// `email?: text` parses as the elvis `email ? email : text`; in a type body it is the optional typed field
			Node::Key(name_node, crate::operators::Op::Question, rest) if matches!(rest.drop_meta(), Node::Key(same, crate::operators::Op::Colon, _) if same == name_node) => {
				let field = Self::extract_field(rest)?;
				Some(FieldDef { type_name: format!("{}{OPTIONAL_SUFFIX}", field.type_name), ..field })
			}
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
							_ => UNTYPED_FIELD.to_string(),
						}
					}
					_ => UNTYPED_FIELD.to_string(), // a default value `x=0` or `x:0`, no type
				};
				Some(FieldDef { name, type_name })
			}
			Node::Symbol(word) => Some(FieldDef::untyped(word)),
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
		"Int" | "i64" | "long" | "Float" | "f64" | "double" | "i32" | "int" | "f32" | "float" if optional => FieldStorage::Node,
		"Int" | "i64" | "long" => FieldStorage::I64,
		"Float" | "f64" | "double" => FieldStorage::F64,
		"i32" | "int" => FieldStorage::I32,
		"f32" | "float" => FieldStorage::F32,
		"Text" | "String" | "string" | "text" => FieldStorage::Text,
		"Node" | UNTYPED_FIELD => FieldStorage::Node,
		_ if crate::analyzer::type_word_kind(word).is_some() || crate::analyzer::plural_element_type(word).is_some() || word == "list" || word.starts_with("list of ") => FieldStorage::Node,
		_ => FieldStorage::Named,
	}
}

/// Convert FieldDef to ValType for function parameters
pub fn field_def_to_val_type(field: &FieldDef, emitter: &WasmGcEmitter) -> ValType {
	let reference = |heap_type| Ref(RefType { nullable: true, heap_type });
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
