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

impl Serialize for DataValue {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut state = serializer.serialize_struct("DataValue", 2)?;
		state.serialize_field("type_name", &self.type_name)?;
		state.serialize_field("data_type", &self.data_type)?;
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

		let helper = DataValueHelper::deserialize(deserializer)?;
		// Create a placeholder DataValue with empty string
		Ok(DataValue {
			data: Box::new(String::from("<deserialized>")),
			type_name: helper.type_name,
			data_type: helper.data_type,
		})
	}
}
