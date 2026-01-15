use crate::node::Node;

/// Local variable or function parameter (mirrors C++ struct Local)
/// In WASM, function arguments and locals share the same index space
#[derive(Clone, Debug, PartialEq)]
pub struct Local {
	pub name: String,
	pub type_node: Option<Box<Node>>,  // Type as Node (Symbol or complex type)
	pub position: u32,                  // WASM local index
	pub is_param: bool,                 // Parameter vs local variable
	pub kind: crate::type_kinds::Kind,  // Value kind (Int, Float, Text, etc.)
	pub data_pointer: u32,              // Linear memory offset for reference data
	pub data_length: u32,               // Length of data in memory (for strings)
}


impl Local {
	pub fn new(position: u32, name: impl Into<String>, kind: crate::type_kinds::Kind) -> Self {
		Local {
			position,
			name: name.into(),
			type_node: None,
			kind,
			is_param: false,
			data_pointer: 0,
			data_length: 0,
		}
	}

	pub fn param(position: u32, name: impl Into<String>, kind: crate::type_kinds::Kind) -> Self {
		Local {
			position,
			name: name.into(),
			type_node: None,
			kind,
			is_param: true,
			data_pointer: 0,
			data_length: 0,
		}
	}
}

pub type Variable = Local;
