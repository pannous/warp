//! The functions of an emitted module by name: imports first, then code functions, each with its call index

use std::collections::HashMap;

/// A function of the module: its name, whether it is imported, and the index a call instruction names
#[derive(Debug, Clone)]
pub struct Function {
	pub name: String,
	pub call_index: i32,
	pub is_import: bool,
}

impl Function {
	/// A host function: an import
	pub fn host(name: impl Into<String>) -> Self {
		Function { name: name.into(), call_index: -1, is_import: true }
	}

	/// A runtime function emitted into the code section
	pub fn builtin(name: impl Into<String>) -> Self {
		Function { is_import: false, ..Self::host(name) }
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
		if func.is_import {
			func.call_index = self.next_import_idx as i32;
			self.next_import_idx += 1;
		} else {
			func.call_index = (self.next_import_idx + self.next_code_idx) as i32;
			self.next_code_idx += 1;
		}
		let call_index = func.call_index as u32;
		self.name_to_idx.insert(func.name.clone(), self.functions.len());
		self.functions.push(func);
		call_index
	}

	/// Register a code function at the index the emitter declared it under: program functions and functions emitted
	/// after them take indices between the runtime functions
	pub fn register_at(&mut self, mut func: Function, call_index: u32) {
		func.call_index = call_index as i32;
		self.next_code_idx += 1;
		self.name_to_idx.insert(func.name.clone(), self.functions.len());
		self.functions.push(func);
	}

	pub fn get(&self, name: &str) -> Option<&Function> {
		self.name_to_idx.get(name).map(|&idx| &self.functions[idx])
	}

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

	pub fn contains(&self, name: &str) -> bool {
		self.name_to_idx.contains_key(name)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_function_registry() {
		let mut reg = FunctionRegistry::new();
		assert_eq!(reg.register(Function::host("fetch")), 0);
		assert_eq!(reg.register(Function::builtin("add")), 1); // 1 import + 0 code = 1
		assert_eq!(reg.import_count(), 1);
		assert_eq!(reg.code_count(), 1);
		assert!(reg.contains("fetch"));
		assert!(reg.contains("add"));
	}
}
