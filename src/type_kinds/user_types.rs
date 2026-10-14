//! User-defined types: their fields, tags and the registry that gives each its WASM GC struct
use super::*;

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
	pub tag: u32, // tag value (>= USER_TYPE_TAG_START)
	pub fields: Vec<FieldDef>,
	pub wasm_type_idx: Option<u32>, // WASM GC type index when emitted
}

pub(super) const OPTIONAL_SUFFIX: char = '?';
pub const UNTYPED_FIELD: &str = "any";

/// `email?: text` parses as the elvis `email ? email : text`: the optional field, as its `email: text`
pub(crate) fn optional_field(node: &crate::node::Node) -> Option<&crate::node::Node> {
	use crate::node::Node;
	use crate::operators::Op;
	match node.drop_meta() {
		Node::Key(name, Op::Question, field) if matches!(field.drop_meta(), Node::Key(same, Op::Colon, _) if same == name) => {
			Some(field)
		}
		_ => None,
	}
}

impl FieldDef {
	/// A field written as a bare word, `name`, optional as `email?`
	pub fn untyped(word: &str) -> FieldDef {
		let type_name = if word.ends_with(OPTIONAL_SUFFIX) {
			format!("{UNTYPED_FIELD}{OPTIONAL_SUFFIX}")
		} else {
			UNTYPED_FIELD.to_string()
		};
		FieldDef {
			name: word.trim_end_matches(OPTIONAL_SUFFIX).to_string(),
			type_name,
		}
	}

	/// A field declared `email?` or `x:int?` may be left out of the constructor call (it is then ø)
	pub fn is_optional(&self) -> bool {
		self.type_name.ends_with(OPTIONAL_SUFFIX)
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
	use crate::extensions::numbers::Number;
	use crate::node::Node;
	use crate::type_kinds::RawFieldValue;

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
			Node::Key(name, Op::Colon, value)
				if matches!(
					value.drop_meta(),
					Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::True | Node::False
				) =>
			{
				Some((name.drop_meta().to_string(), value.drop_meta().clone()))
			}
			_ => None,
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
			if body.is_empty_node() {
				return None;
			}
			let type_name = Self::written_name(name)?;
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
		Self::field_items(body).filter_map(Self::extract_field).collect()
	}

	/// The word or text a name is written as
	fn written_name(node: &crate::node::Node) -> Option<String> {
		use crate::node::Node;
		match node.drop_meta() {
			Node::Symbol(s) | Node::Text(s) => Some(s.clone()),
			_ => None,
		}
	}

	/// Extract a single FieldDef from a Key node (name:Type)
	fn extract_field(node: &crate::node::Node) -> Option<FieldDef> {
		use crate::node::Node;
		let node = node.drop_meta();
		// in a type body the optional typed field
		if let Some(field) = optional_field(node) {
			let field = Self::extract_field(field)?;
			return Some(FieldDef {
				type_name: format!("{}{OPTIONAL_SUFFIX}", field.type_name),
				..field
			});
		}
		match node {
			// `x:int=0`: the typed field x with a default
			Node::Key(target, crate::operators::Op::Assign, _)
				if matches!(target.drop_meta(), Node::Key(_, crate::operators::Op::Colon, _)) =>
			{
				Self::extract_field(target)
			}
			Node::Key(name_node, _, type_node) => {
				let name = Self::written_name(name_node)?;
				let type_name = match type_node.drop_meta() {
					Node::Type {
						name: type_name_node, ..
					} => Self::written_name(type_name_node),
					// `size: int | text`: the union's name, `int or text`
					written => Self::written_name(written).or_else(|| crate::analyzer::union_type_name(written)), // none for a default value `x=0` or `x:0`
				};
				let type_name = type_name.unwrap_or_else(|| UNTYPED_FIELD.to_string());
				Some(FieldDef { name, type_name })
			}
			Node::Symbol(word) => Some(FieldDef::untyped(word)),
			_ => None,
		}
	}
}
