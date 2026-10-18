use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::any::Any;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct LineInfo {
	// #cfg(DEBUG!)]{  DONE conditional compilation
	pub line_nr: usize,
	pub column: usize,
	#[cfg(debug_assertions)]
	pub line: String, // debug! expensive but useful
	/// The module file the position is in, None for the program itself (card module-error-file)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub file: Option<String>,
	/// A unit word the parser joined to the number before it (`3m`, `3 m`, not `3 * m`): where the program names a
	/// variable so, the unit wins there (card variable-named-like)
	#[serde(default, skip_serializing_if = "std::ops::Not::not")]
	pub after_number: bool,
}

/// How much of its source line a position keeps in debug builds
#[cfg(debug_assertions)]
const DEBUG_LINE_LENGTH: usize = 80;

impl LineInfo {
	/// A position the parser records, in the module file being parsed (parsing_file) if any
	/// (the start of the source `line`, kept in debug builds only: a whole long line in every node made a 30 KB literal
	/// take gigabytes, card big-literals)
	pub fn at(line_nr: usize, column: usize, #[cfg_attr(not(debug_assertions), allow(unused))] line: &str) -> Self {
		LineInfo {
			line_nr,
			column,
			#[cfg(debug_assertions)]
			line: line.chars().take(DEBUG_LINE_LENGTH).collect(),
			file: crate::compiler::compile_context::read(|context| context.parsed_file.clone()),
			after_number: false,
		}
	}

	/// `line:column`, or `file:line:column` in a module's file
	pub fn place(&self) -> String {
		place_text(self.file.as_deref(), self.line_nr, self.column)
	}
}

/// `line:column`, prefixed with the module's `file:` when the position is in one
pub fn place_text(file: Option<&str>, line: usize, column: usize) -> String {
	match file {
		Some(file) => format!("{file}:{line}:{column}"),
		None => format!("{line}:{column}"),
	}
}

/// The file of code a pass generates from a template: its positions are in the template, not in the program
pub const GENERATED_FILE: &str = "generated";

/// `parse` with the positions it records marked as in `file`: a module's, so its failures name it
pub fn parsing_file<T>(file: &str, parse: impl FnOnce() -> T) -> T {
	crate::compiler::compile_context::scoped(|context| &mut context.parsed_file, Some(file.to_string()), parse)
}

// Custom trait for cloneable Any types with equality support
pub trait CloneAny: Any {
	fn clone_any(&self) -> Box<dyn CloneAny>;
	fn as_any(&self) -> &dyn Any;
	fn eq_any(&self, other: &dyn CloneAny) -> bool;
}

impl<T: 'static + Clone + PartialEq> CloneAny for T {
	fn clone_any(&self) -> Box<dyn CloneAny> {
		Box::new(self.clone())
	}
	fn as_any(&self) -> &dyn Any {
		self
	}
	fn eq_any(&self, other: &dyn CloneAny) -> bool {
		if let Some(other_t) = other.as_any().downcast_ref::<T>() {
			self == other_t
		} else {
			false
		}
	}
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DataType {
	Primitive, // map to Node(Number) early! get rid? sometimes need f16 vs f32 vs f64?
	String,    // map to Node(String) early! get rid?
	Vec,       // map to Node::List(…) early or keep raw for efficiency!
	Tuple,     // - '' -
	Reference,
	Struct, // map to Node(…) early!! (if possible, else interesting Rust objects!)
	Other,  // <- only interesting cases
	None,   // <- only interesting cases
}

pub struct DataValue {
	pub(crate) data: Box<dyn CloneAny>,
	pub type_name: String,
	pub data_type: DataType,
}

// most generic container for any kind of data not captured by other node types
// Vec, tuples, primitives, custom structs, etc.
// let v = Node::data(vec![1, 2, 3]);
// let t = Node::data((42, "answer"));
// let n = Node::data(CustomData { id: 42, name: "test" });
// 💡 let extract = dada.downcast_ref::<MyType>(); 💡
impl DataValue {
	pub fn new<T: 'static + Clone + PartialEq>(data: T) -> Self {
		let type_name = std::any::type_name::<T>().to_string();
		let data_type = Self::infer_type(&type_name);
		DataValue { data: Box::new(data), type_name, data_type }
	}

	pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
		self.data.as_any().downcast_ref::<T>()
	}

	fn infer_type(type_name: &str) -> DataType {
		if type_name.starts_with("alloc::vec::Vec") || type_name.starts_with("std::vec::Vec") {
			DataType::Vec
		} else if type_name.starts_with('(') && type_name.ends_with(')') {
			DataType::Tuple
		} else if type_name.contains("::String") || type_name == "str" || type_name == "&str" {
			DataType::String // map to Node(String) early!
		} else if type_name.contains("::") {
			DataType::Struct
		} else if matches!(
			type_name,
			"i8" | "i16"
				| "i32" | "i64"
				| "i128" | "isize"
				| "u8" | "u16"
				| "u32" | "u64"
				| "u128" | "usize"
				| "f32" | "f64"
				| "bool" | "char"
		) {
			DataType::Primitive
		} else {
			DataType::Other
		}
	}
}

impl Clone for DataValue {
	fn clone(&self) -> Self {
		DataValue { data: self.data.clone_any(), type_name: self.type_name.clone(), data_type: self.data_type.clone() }
	}
}

impl fmt::Display for DataValue {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		// trace!(f, "DataValue({:?}:{})", self.data_type, self.type_name);
		write!(f, "«DataValue({})»", self.type_name)
	}
}

impl fmt::Debug for DataValue {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		// Special handling for GcObject to show actual values
		#[cfg(feature = "native")]
		if self.type_name.contains("GcObject") {
			if let Some(gc_obj) = self.downcast_ref::<crate::gc_traits::GcObject>() {
				return write!(f, "{:?}", gc_obj);
			}
		}
		write!(f, "DataValue({:?}:{})", self.data_type, self.type_name)
	}
}

impl PartialEq for DataValue {
	fn eq(&self, other: &Self) -> bool {
		self.data.eq_any(other.data.as_ref())
	}
}

/// A Data payload whose value serde keeps in binary formats (a lowered module kept on disk, modules/lowered_modules.rs):
/// its type name and how its value is written as JSON and read back. Other payloads read back as a placeholder text
struct PayloadCodec {
	type_name: &'static str,
	write: fn(&dyn CloneAny) -> Option<String>,
	read: fn(&str) -> Option<Box<dyn CloneAny>>,
}

fn codec<T: Serialize + serde::de::DeserializeOwned + Clone + PartialEq + 'static>() -> PayloadCodec {
	PayloadCodec {
		type_name: std::any::type_name::<T>(),
		write: |value| serde_json::to_string(value.as_any().downcast_ref::<T>()?).ok(),
		read: |text| Some(Box::new(serde_json::from_str::<T>(text).ok()?)),
	}
}

/// The payloads lowered standard modules hold
static KEPT_PAYLOADS: std::sync::LazyLock<Vec<PayloadCodec>> = std::sync::LazyLock::new(|| {
	vec![
		codec::<LineInfo>(),
		codec::<crate::warp_parser::SpacedMinus>(),
		codec::<crate::warp_parser::WrittenIndex>(),
		codec::<crate::warp_parser::SubscriptOffset>(),
		codec::<crate::warp_parser::PathLiteral>(),
	]
});

fn payload_codec(type_name: &str) -> Option<&'static PayloadCodec> {
	KEPT_PAYLOADS.iter().find(|codec| codec.type_name == type_name)
}

impl DataValue {
	/// Whether serde keeps this payload's value in binary formats
	pub fn is_kept_payload(&self) -> bool {
		payload_codec(&self.type_name).is_some()
	}
}

/// type name and data type; in binary formats also the payload's value as JSON, when its type has a PayloadCodec
impl Serialize for DataValue {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let binary = !serializer.is_human_readable();
		let mut state = serializer.serialize_struct("DataValue", 2 + usize::from(binary))?;
		state.serialize_field("type_name", &self.type_name)?;
		state.serialize_field("data_type", &self.data_type)?;
		if binary {
			let payload = payload_codec(&self.type_name).and_then(|codec| (codec.write)(self.data.as_ref()));
			state.serialize_field("payload", &payload)?;
		}
		state.end()
	}
}

impl<'de> Deserialize<'de> for DataValue {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		#[derive(Deserialize)]
		struct DataValueHelper {
			type_name: String,
			data_type: DataType,
		}
		#[derive(Deserialize)]
		struct BinaryDataValueHelper {
			type_name: String,
			data_type: DataType,
			payload: Option<String>,
		}

		let (type_name, data_type, payload) = match deserializer.is_human_readable() {
			true => {
				DataValueHelper::deserialize(deserializer).map(|helper| (helper.type_name, helper.data_type, None))?
			}
			false => BinaryDataValueHelper::deserialize(deserializer)
				.map(|helper| (helper.type_name, helper.data_type, helper.payload))?,
		};
		let kept = payload.zip(payload_codec(&type_name)).and_then(|(payload, codec)| (codec.read)(&payload));
		let placeholder = || Box::new(String::from("<deserialized>")) as Box<dyn CloneAny>;
		Ok(DataValue { data: kept.unwrap_or_else(placeholder), type_name, data_type })
	}
}
