//! Function representation for WASM code generation
//!
//! Provides structured representation of functions with:
//! - Named parameters (Arg)
//! - Type signatures (Signature)
//! - Function metadata and variants (Function)
//!
//! Design based on wasp/source/Code.h Function/Signature classes.

use crate::node::Node;
use crate::type_kinds::Kind;
use std::collections::HashMap;

/// WASM value types for function signatures
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ValType {
	#[default]
	Void = 0x40,
	I32 = 0x7F,
	I64 = 0x7E,
	F32 = 0x7D,
	F64 = 0x7C,
	V128 = 0x7B,
	FuncRef = 0x70,
	ExternRef = 0x6F,
	AnyRef = 0x6E,
	EqRef = 0x6D,
	I31Ref = 0x6C,
	StructRef = 0x6B,
	ArrayRef = 0x6A,
	NullFuncRef = 0x73,
	NullExternRef = 0x72,
	NullRef = 0x71,
	// Reference to type index (for GC structs)
	Ref(u32) = 0x64,
	RefNull(u32) = 0x63,
}

impl ValType {
	pub fn is_numeric(&self) -> bool {
		matches!(self, ValType::I32 | ValType::I64 | ValType::F32 | ValType::F64)
	}

	pub fn is_ref(&self) -> bool {
		matches!(
			self,
			ValType::FuncRef
				| ValType::ExternRef
				| ValType::AnyRef
				| ValType::EqRef
				| ValType::I31Ref
				| ValType::StructRef
				| ValType::ArrayRef
				| ValType::Ref(_)
				| ValType::RefNull(_)
		)
	}

	/// Convert Kind to appropriate ValType
	pub fn from_kind(kind: Kind) -> Self {
		match kind {
			Kind::Int => ValType::I64,
			Kind::Float => ValType::F64,
			Kind::Codepoint => ValType::I32,
			_ => ValType::AnyRef, // Reference types use anyref
		}
	}
}

/// Primitive type enum matching wasp Type
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Type {
	#[default]
	Unknown = 0,
	Void = 1,
	Bool = 2,
	Int = 3,
	Int8 = 4,
	Int16 = 5,
	Int32 = 6,
	Int64 = 7,
	UInt = 8,
	UInt8 = 9,
	UInt16 = 10,
	UInt32 = 11,
	UInt64 = 12,
	Float = 13,
	Float32 = 14,
	Float64 = 15,
	Char = 16,
	String = 17,
	Symbol = 18,
	Node = 19,
	List = 20,
	Map = 21,
	Any = 22,
	Ref = 23,
	Struct = 24,
	Array = 25,
}

impl Type {
	/// Convert to WASM ValType
	pub fn to_valtype(&self) -> ValType {
		match self {
			Type::Void => ValType::Void,
			Type::Bool | Type::Int8 | Type::Int16 | Type::Int32 | Type::UInt8 | Type::UInt16 | Type::UInt32 | Type::Char => ValType::I32,
			Type::Int | Type::Int64 | Type::UInt | Type::UInt64 => ValType::I64,
			Type::Float | Type::Float32 => ValType::F32,
			Type::Float64 => ValType::F64,
			Type::String | Type::Symbol | Type::Node | Type::List | Type::Map | Type::Any | Type::Ref | Type::Struct | Type::Array => ValType::AnyRef,
			Type::Unknown => ValType::Void,
		}
	}

	/// Parse type from string name
	pub fn from_name(name: &str) -> Self {
		match name.to_lowercase().as_str() {
			"void" | "()" | "nil" => Type::Void,
			"bool" | "boolean" => Type::Bool,
			"int" | "integer" => Type::Int,
			"i8" | "int8" | "byte" => Type::Int8,
			"i16" | "int16" | "short" => Type::Int16,
			"i32" | "int32" => Type::Int32,
			"i64" | "int64" | "long" => Type::Int64,
			"uint" | "unsigned" => Type::UInt,
			"u8" | "uint8" | "ubyte" => Type::UInt8,
			"u16" | "uint16" | "ushort" => Type::UInt16,
			"u32" | "uint32" => Type::UInt32,
			"u64" | "uint64" | "ulong" => Type::UInt64,
			"float" | "real" => Type::Float,
			"f32" | "float32" => Type::Float32,
			"f64" | "float64" | "double" => Type::Float64,
			"char" | "codepoint" => Type::Char,
			"string" | "str" | "text" => Type::String,
			"symbol" | "sym" => Type::Symbol,
			"node" => Type::Node,
			"list" | "array" => Type::List,
			"map" | "dict" | "object" => Type::Map,
			"any" | "*" => Type::Any,
			"ref" | "reference" => Type::Ref,
			"struct" => Type::Struct,
			_ => Type::Unknown,
		}
	}
}

/// ABI (Application Binary Interface) for function calling conventions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ABI {
	#[default]
	Native,          // Standard WASM calling convention
	Wasp,            // Multi-value return tuples (value, type)
	WaspSmartPointers, // Smart pointers for multi-value compatibility
	Canonical,       // WIT canonical ABI
}

/// Function argument/parameter with name and type
#[derive(Debug, Clone, PartialEq)]
pub struct Arg {
	pub name: String,
	pub type_: Type,
	pub modifiers: Vec<String>, // const, mut, ref, etc.
}

impl Arg {
	pub fn new(name: impl Into<String>, type_: Type) -> Self {
		Arg {
			name: name.into(),
			type_,
			modifiers: Vec::new(),
		}
	}

	pub fn with_modifier(mut self, modifier: impl Into<String>) -> Self {
		self.modifiers.push(modifier.into());
		self
	}

	/// Get WASM ValType for this argument
	pub fn valtype(&self) -> ValType {
		self.type_.to_valtype()
	}
}

/// Function type signature with parameters and return types
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Signature {
	pub type_index: i32,           // Index in WASM type section (-1 if not assigned)
	pub abi: ABI,
	pub parameters: Vec<Arg>,      // Named parameters
	pub return_types: Vec<Type>,   // Return type(s) - WASM supports multi-value
	pub is_handled: bool,          // Already emitted to WASM
}

impl Signature {
	pub fn new() -> Self {
		Signature {
			type_index: -1,
			abi: ABI::Native,
			parameters: Vec::new(),
			return_types: Vec::new(),
			is_handled: false,
		}
	}

	/// Add a parameter with type
	pub fn param(mut self, name: impl Into<String>, type_: Type) -> Self {
		self.parameters.push(Arg::new(name, type_));
		self
	}

	/// Add a parameter (chainable builder pattern)
	pub fn add(&mut self, name: impl Into<String>, type_: Type) -> &mut Self {
		self.parameters.push(Arg::new(name, type_));
		self
	}

	/// Set return type(s)
	pub fn returns(mut self, type_: Type) -> Self {
		if type_ != Type::Void {
			self.return_types.push(type_);
		}
		self
	}

	/// Add return type (mutable)
	pub fn add_return(&mut self, type_: Type) -> &mut Self {
		if type_ != Type::Void {
			self.return_types.push(type_);
		}
		self
	}

	/// Number of parameters
	pub fn len(&self) -> usize {
		self.parameters.len()
	}

	pub fn is_empty(&self) -> bool {
		self.parameters.is_empty()
	}

	/// Check if signature has a parameter with given name
	pub fn has(&self, name: &str) -> bool {
		self.parameters.iter().any(|p| p.name == name)
	}

	/// Get parameter by name
	pub fn get(&self, name: &str) -> Option<&Arg> {
		self.parameters.iter().find(|p| p.name == name)
	}

	/// Get parameter index by name
	pub fn index_of(&self, name: &str) -> Option<usize> {
		self.parameters.iter().position(|p| p.name == name)
	}

	/// Get WASM parameter types
	pub fn param_valtypes(&self) -> Vec<ValType> {
		self.parameters.iter().map(|p| p.valtype()).collect()
	}

	/// Get WASM return types
	pub fn return_valtypes(&self) -> Vec<ValType> {
		self.return_types.iter().map(|t| t.to_valtype()).collect()
	}

	/// Merge another signature into this one (fill empty fields)
	pub fn merge(&mut self, other: &Signature) {
		if self.type_index < 0 {
			self.type_index = other.type_index;
		}
		if self.return_types.is_empty() {
			self.return_types = other.return_types.clone();
		}
		if self.parameters.is_empty() {
			self.parameters = other.parameters.clone();
		}
	}

	/// Format signature for display
	pub fn format(&self) -> String {
		let params: Vec<String> = self.parameters
			.iter()
			.map(|p| format!("{}: {:?}", p.name, p.type_))
			.collect();
		let returns: Vec<String> = self.return_types
			.iter()
			.map(|t| format!("{:?}", t))
			.collect();
		if returns.is_empty() {
			format!("({})", params.join(", "))
		} else {
			format!("({}) -> {}", params.join(", "), returns.join(", "))
		}
	}
}

impl std::fmt::Display for Signature {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.format())
	}
}

/// Local variable within a function
#[derive(Debug, Clone, PartialEq)]
pub struct Local {
	pub position: u32,     // Index in locals array
	pub name: String,
	pub type_: Type,
	pub is_param: bool,    // Function parameter vs local variable
	pub data_pointer: u32, // Linear memory offset for reference data
}

impl Local {
	pub fn new(position: u32, name: impl Into<String>, type_: Type) -> Self {
		Local {
			position,
			name: name.into(),
			type_,
			is_param: false,
			data_pointer: 0,
		}
	}

	pub fn param(position: u32, name: impl Into<String>, type_: Type) -> Self {
		Local {
			position,
			name: name.into(),
			type_,
			is_param: true,
			data_pointer: 0,
		}
	}
}

/// Function representation for WASM code generation
#[derive(Debug, Clone, Default)]
pub struct Function {
	// Indices
	pub code_index: i32,     // Index in code section (-1 if not assigned)
	pub call_index: i32,     // Index for call instruction (code_index + import_count)
	pub type_index: i32,     // Index in type section

	// Names
	pub name: String,
	pub export_name: Option<String>,
	pub fullname: String,    // Original/mangled name

	// Signature
	pub signature: Signature,

	// Body
	pub body: Option<Box<Node>>,
	pub code: Vec<u8>,       // Compiled WASM bytecode

	// Flags
	pub is_import: bool,     // Not in code section, but in import section
	pub is_declared: bool,   // Exported or called
	pub is_host: bool,       // Host function (subset of imports)
	pub is_builtin: bool,    // Hardcoded builtin function
	pub is_runtime: bool,    // Runtime library function
	pub is_handled: bool,    // Already emitted
	pub is_used: bool,       // Called somewhere
	pub is_polymorphic: bool, // Has multiple type variants
	pub is_ffi: bool,        // Foreign function interface import
	pub ffi_library: Option<String>, // FFI library name

	// Polymorphism
	pub variants: Vec<Function>, // Type variants for polymorphic functions

	// Locals (including params at start)
	pub locals: HashMap<String, Local>,
}

impl Function {
	pub fn new(name: impl Into<String>) -> Self {
		let n = name.into();
		Function {
			name: n.clone(),
			fullname: n,
			code_index: -1,
			call_index: -1,
			type_index: -1,
			export_name: None,
			signature: Signature::new(),
			body: None,
			code: Vec::new(),
			is_import: false,
			is_declared: false,
			is_host: false,
			is_builtin: false,
			is_runtime: false,
			is_handled: false,
			is_used: false,
			is_polymorphic: false,
			is_ffi: false,
			ffi_library: None,
			variants: Vec::new(),
			locals: HashMap::new(),
		}
	}

	/// Create an import function
	pub fn import(name: impl Into<String>) -> Self {
		let mut f = Self::new(name);
		f.is_import = true;
		f
	}

	/// Create a host function (special import)
	pub fn host(name: impl Into<String>) -> Self {
		let mut f = Self::new(name);
		f.is_import = true;
		f.is_host = true;
		f
	}

	/// Create a builtin function
	pub fn builtin(name: impl Into<String>) -> Self {
		let mut f = Self::new(name);
		f.is_builtin = true;
		f
	}

	/// Mark as runtime function
	pub fn runtime(mut self) -> Self {
		self.is_runtime = true;
		self.is_declared = false;
		self
	}

	/// Mark as handled/emitted
	pub fn handled(mut self) -> Self {
		self.is_handled = true;
		self
	}

	/// Set the signature
	pub fn with_signature(mut self, sig: Signature) -> Self {
		self.signature = sig;
		self
	}

	/// Set export name
	pub fn exported(mut self, name: impl Into<String>) -> Self {
		self.export_name = Some(name.into());
		self.is_declared = true;
		self
	}

	/// Allocate a new local variable, returns its index
	pub fn allocate_local(&mut self, name: impl Into<String>, type_: Type) -> u32 {
		let n = name.into();
		let position = self.locals.len() as u32;
		self.locals.insert(n.clone(), Local::new(position, n, type_));
		position
	}

	/// Get local by name
	pub fn get_local(&self, name: &str) -> Option<&Local> {
		self.locals.get(name)
	}

	/// Get local index by name
	pub fn local_index(&self, name: &str) -> Option<u32> {
		self.locals.get(name).map(|l| l.position)
	}

	/// Add parameter and create corresponding local
	pub fn add_param(&mut self, name: impl Into<String>, type_: Type) {
		let n = name.into();
		let position = self.locals.len() as u32;
		self.signature.add(&n, type_);
		self.locals.insert(n.clone(), Local::param(position, n, type_));
	}

	/// Find best matching variant for given argument types
	pub fn find_variant(&self, arg_types: &[Type]) -> Option<usize> {
		if self.variants.is_empty() {
			return None;
		}
		for (i, variant) in self.variants.iter().enumerate() {
			let sig = &variant.signature;
			if sig.len() == arg_types.len() {
				let matches = sig.parameters.iter()
					.zip(arg_types.iter())
					.all(|(p, t)| p.type_ == *t || p.type_ == Type::Any);
				if matches {
					return Some(i);
				}
			}
		}
		None
	}
}

impl std::fmt::Display for Function {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}{}", self.name, self.signature)
	}
}

/// Registry of functions for a module
#[derive(Debug, Default)]
pub struct FunctionRegistry {
	functions: Vec<Function>,
	name_to_idx: HashMap<String, usize>,
	next_import_idx: u32,
	next_code_idx: u32,
}

impl FunctionRegistry {
	pub fn new() -> Self {
		Self::default()
	}

	/// Register a function, returns its call index
	pub fn register(&mut self, mut func: Function) -> u32 {
		let name = func.name.clone();

		// Assign indices based on import vs code function
		if func.is_import {
			func.call_index = self.next_import_idx as i32;
			self.next_import_idx += 1;
		} else {
			func.code_index = self.next_code_idx as i32;
			func.call_index = (self.next_import_idx + self.next_code_idx) as i32;
			self.next_code_idx += 1;
		}

		let idx = self.functions.len();
		self.functions.push(func);
		self.name_to_idx.insert(name, idx);
		self.functions[idx].call_index as u32
	}

	/// Get function by name
	pub fn get(&self, name: &str) -> Option<&Function> {
		self.name_to_idx.get(name).map(|&idx| &self.functions[idx])
	}

	/// Get mutable function by name
	pub fn get_mut(&mut self, name: &str) -> Option<&mut Function> {
		self.name_to_idx.get(name).copied().map(|idx| &mut self.functions[idx])
	}

	/// Get function by call index
	pub fn get_by_index(&self, call_index: u32) -> Option<&Function> {
		self.functions.iter().find(|f| f.call_index == call_index as i32)
	}

	/// Get all imports
	pub fn imports(&self) -> impl Iterator<Item = &Function> {
		self.functions.iter().filter(|f| f.is_import)
	}

	/// Get all code functions (non-imports)
	pub fn code_functions(&self) -> impl Iterator<Item = &Function> {
		self.functions.iter().filter(|f| !f.is_import)
	}

	/// Get all functions
	pub fn all(&self) -> &[Function] {
		&self.functions
	}

	/// Number of imported functions
	pub fn import_count(&self) -> u32 {
		self.next_import_idx
	}

	/// Number of code functions
	pub fn code_count(&self) -> u32 {
		self.next_code_idx
	}

	/// Check if function exists
	pub fn contains(&self, name: &str) -> bool {
		self.name_to_idx.contains_key(name)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_arg() {
		let arg = Arg::new("x", Type::Int);
		assert_eq!(arg.name, "x");
		assert_eq!(arg.type_, Type::Int);
		assert_eq!(arg.valtype(), ValType::I64);
	}

	#[test]
	fn test_signature_builder() {
		let sig = Signature::new()
			.param("x", Type::Int)
			.param("y", Type::Float)
			.returns(Type::Bool);

		assert_eq!(sig.len(), 2);
		assert!(sig.has("x"));
		assert!(sig.has("y"));
		assert!(!sig.has("z"));
		assert_eq!(sig.return_types.len(), 1);
	}

	#[test]
	fn test_signature_format() {
		let sig = Signature::new()
			.param("a", Type::Int)
			.param("b", Type::String)
			.returns(Type::Bool);
		let formatted = sig.format();
		assert!(formatted.contains("a: Int"));
		assert!(formatted.contains("b: String"));
		assert!(formatted.contains("Bool"));
	}

	#[test]
	fn test_function_builder() {
		let func = Function::new("add")
			.with_signature(
				Signature::new()
					.param("a", Type::Int)
					.param("b", Type::Int)
					.returns(Type::Int)
			)
			.exported("add");

		assert_eq!(func.name, "add");
		assert_eq!(func.export_name, Some("add".to_string()));
		assert!(func.is_declared);
		assert_eq!(func.signature.len(), 2);
	}

	#[test]
	fn test_function_host() {
		let func = Function::host("fetch");
		assert!(func.is_import);
		assert!(func.is_host);
	}

	#[test]
	fn test_function_locals() {
		let mut func = Function::new("test");
		func.add_param("x", Type::Int);
		func.allocate_local("temp", Type::Float);

		assert_eq!(func.locals.len(), 2);
		assert_eq!(func.local_index("x"), Some(0));
		assert_eq!(func.local_index("temp"), Some(1));
		assert!(func.get_local("x").unwrap().is_param);
		assert!(!func.get_local("temp").unwrap().is_param);
	}

	#[test]
	fn test_function_registry() {
		let mut reg = FunctionRegistry::new();

		let fetch = Function::host("fetch")
			.with_signature(Signature::new()
				.param("url", Type::String)
				.returns(Type::String));
		let fetch_idx = reg.register(fetch);
		assert_eq!(fetch_idx, 0);

		let add = Function::new("add")
			.with_signature(Signature::new()
				.param("a", Type::Int)
				.param("b", Type::Int)
				.returns(Type::Int));
		let add_idx = reg.register(add);
		assert_eq!(add_idx, 1); // 1 import + 0 code = 1

		assert_eq!(reg.import_count(), 1);
		assert_eq!(reg.code_count(), 1);
		assert!(reg.contains("fetch"));
		assert!(reg.contains("add"));
	}

	#[test]
	fn test_valtype_from_kind() {
		assert_eq!(ValType::from_kind(Kind::Int), ValType::I64);
		assert_eq!(ValType::from_kind(Kind::Float), ValType::F64);
		assert_eq!(ValType::from_kind(Kind::Codepoint), ValType::I32);
		assert_eq!(ValType::from_kind(Kind::Text), ValType::AnyRef);
	}

	#[test]
	fn test_type_from_name() {
		assert_eq!(Type::from_name("int"), Type::Int);
		assert_eq!(Type::from_name("i64"), Type::Int64);
		assert_eq!(Type::from_name("string"), Type::String);
		assert_eq!(Type::from_name("Float64"), Type::Float64);
		assert_eq!(Type::from_name("unknown_type"), Type::Unknown);
	}
}
