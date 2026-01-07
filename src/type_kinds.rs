/// Node type tags for runtime type checking and WASM encoding
/// Compact repr(u8) for efficient storage in WASM GC structs
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
	Empty = 0,
	Int = 1,       // i64 value (boxed in $i64box)
	Float = 2,     // f64 value (boxed in $f64box)
	Text = 3,      // string (via $String struct)
	Codepoint = 4, // char as i31ref
	Symbol = 5,    // string (via $String struct)
	Key = 6,      // data=key node, value=value node (also used for pairs)
	Block = 7,    // curly braces {}
	List = 8,     // square brackets []
	Data = 9,     // arbitrary data container
	Meta = 10,    // metadata wrapper
	Error = 11,   // error node
	TypeDef = 12, // type definition: name + body (fields)
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

