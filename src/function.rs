//! Function representation for WASM code generation
//!
//! Provides structured representation of functions with:
//! - Named parameters (Arg)
//! - Type signatures (Signature)
//! - Function metadata and variants (Function)
//!
//! Design based on wasp/source/Code.h Function/Signature classes.

use crate::local::Local;
use crate::node::Node;
use crate::type_kinds::Kind;
use std::collections::HashMap;
use wasm_encoder::ValType;

/// ABI (Application Binary Interface) for function calling conventions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ABI {
	#[default]
	Native,            // Standard WASM calling convention
	Wasp,              // Multi-value return tuples (value, type)
	WaspSmartPointers, // Smart pointers for multi-value compatibility
	Canonical,         // WIT canonical ABI
}

/// Convert Kind to WASM ValType
pub fn kind_to_valtype(kind: Kind) -> ValType {
	match kind {
		Kind::Int => ValType::I64,
		Kind::Float => ValType::F64,
		Kind::Codepoint => ValType::I32,
		_ => ValType::Ref(wasm_encoder::RefType {
			nullable: true,
			heap_type: wasm_encoder::HeapType::Abstract {
				shared: false,
				ty: wasm_encoder::AbstractHeapType::Any,
			},
		}),
	}
}

/// Function argument/parameter with name and type
#[derive(Debug, Clone, PartialEq)]
pub struct Arg {
	pub name: String,
	pub kind: Kind,
	pub modifiers: Vec<String>, // const, mut, ref, etc.
}

impl Arg {
	pub fn new(name: impl Into<String>, kind: Kind) -> Self {
		Arg {
			name: name.into(),
			kind,
			modifiers: Vec::new(),
		}
	}

	pub fn with_modifier(mut self, modifier: impl Into<String>) -> Self {
		self.modifiers.push(modifier.into());
		self
	}

	/// Get WASM ValType for this argument
	pub fn valtype(&self) -> ValType {
		kind_to_valtype(self.kind)
	}
}

/// Function type signature with parameters and return types
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Signature {
	pub type_index: i32,         // Index in WASM type section (-1 if not assigned)
	pub abi: ABI,
	pub parameters: Vec<Arg>,    // Named parameters
	pub return_types: Vec<Kind>, // Return type(s) - WASM supports multi-value
	pub is_handled: bool,        // Already emitted to WASM
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
	pub fn param(mut self, name: impl Into<String>, kind: Kind) -> Self {
		self.parameters.push(Arg::new(name, kind));
		self
	}

	/// Add a parameter (chainable builder pattern)
	pub fn add(&mut self, name: impl Into<String>, kind: Kind) -> &mut Self {
		self.parameters.push(Arg::new(name, kind));
		self
	}

	/// Set return type(s)
	pub fn returns(mut self, kind: Kind) -> Self {
		if kind != Kind::Empty {
			self.return_types.push(kind);
		}
		self
	}

	/// Add return type (mutable)
	pub fn add_return(&mut self, kind: Kind) -> &mut Self {
		if kind != Kind::Empty {
			self.return_types.push(kind);
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
		self.return_types.iter().map(|k| kind_to_valtype(*k)).collect()
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
			.map(|p| format!("{}: {:?}", p.name, p.kind))
			.collect();
		let returns: Vec<String> = self.return_types
			.iter()
			.map(|k| format!("{:?}", k))
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
	pub fn allocate_local(&mut self, name: impl Into<String>, kind: Kind) -> u32 {
		let n = name.into();
		let position = self.locals.len() as u32;
		self.locals.insert(n.clone(), Local::new(position, n, kind));
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
	pub fn add_param(&mut self, name: impl Into<String>, kind: Kind) {
		let n = name.into();
		let position = self.locals.len() as u32;
		self.signature.add(&n, kind);
		self.locals.insert(n.clone(), Local::param(position, n, kind));
	}

	/// Find best matching variant for given argument types
	pub fn find_variant(&self, arg_kinds: &[Kind]) -> Option<usize> {
		if self.variants.is_empty() {
			return None;
		}
		for (i, variant) in self.variants.iter().enumerate() {
			let sig = &variant.signature;
			if sig.len() == arg_kinds.len() {
				let matches = sig.parameters.iter()
					.zip(arg_kinds.iter())
					.all(|(p, k)| p.kind == *k || p.kind == Kind::Data); // Data as "Any"
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

impl std::ops::Index<&str> for FunctionRegistry {
	type Output = Function;

	fn index(&self, name: &str) -> &Self::Output {
		self.get(name).unwrap_or_else(|| panic!("Function '{}' not found", name))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_arg() {
		let arg = Arg::new("x", Kind::Int);
		assert_eq!(arg.name, "x");
		assert_eq!(arg.kind, Kind::Int);
		assert_eq!(arg.valtype(), ValType::I64);
	}

	#[test]
	fn test_signature_builder() {
		let sig = Signature::new()
			.param("x", Kind::Int)
			.param("y", Kind::Float)
			.returns(Kind::Int);

		assert_eq!(sig.len(), 2);
		assert!(sig.has("x"));
		assert!(sig.has("y"));
		assert!(!sig.has("z"));
		assert_eq!(sig.return_types.len(), 1);
	}

	#[test]
	fn test_signature_format() {
		let sig = Signature::new()
			.param("a", Kind::Int)
			.param("b", Kind::Text)
			.returns(Kind::Int);
		let formatted = sig.format();
		assert!(formatted.contains("a: Int"));
		assert!(formatted.contains("b: Text"));
		assert!(formatted.contains("Int"));
	}

	#[test]
	fn test_function_builder() {
		let func = Function::new("add")
			.with_signature(
				Signature::new()
					.param("a", Kind::Int)
					.param("b", Kind::Int)
					.returns(Kind::Int)
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
		func.add_param("x", Kind::Int);
		func.allocate_local("temp", Kind::Float);

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
				.param("url", Kind::Text)
				.returns(Kind::Text));
		let fetch_idx = reg.register(fetch);
		assert_eq!(fetch_idx, 0);

		let add = Function::new("add")
			.with_signature(Signature::new()
				.param("a", Kind::Int)
				.param("b", Kind::Int)
				.returns(Kind::Int));
		let add_idx = reg.register(add);
		assert_eq!(add_idx, 1); // 1 import + 0 code = 1

		assert_eq!(reg.import_count(), 1);
		assert_eq!(reg.code_count(), 1);
		assert!(reg.contains("fetch"));
		assert!(reg.contains("add"));
	}

	#[test]
	fn test_kind_to_valtype() {
		assert_eq!(kind_to_valtype(Kind::Int), ValType::I64);
		assert_eq!(kind_to_valtype(Kind::Float), ValType::F64);
		assert_eq!(kind_to_valtype(Kind::Codepoint), ValType::I32);
		// Text returns a ref type
		matches!(kind_to_valtype(Kind::Text), ValType::Ref(_));
	}
}
