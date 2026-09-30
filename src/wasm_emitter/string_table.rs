//! String table management for WASM data section

use crate::node::Node;
use crate::operators::Op;
use std::collections::HashMap;
use wasm_encoder::{ConstExpr, DataSection};

/// Manages string allocation in WASM linear memory
pub struct StringTable {
	/// Maps strings to their offsets in linear memory
	table: HashMap<String, u32>,
	/// Next available offset in linear memory
	next_offset: u32,
	/// WASM data section for storing strings
	data_section: DataSection,
}

impl Default for StringTable {
	fn default() -> Self {
		Self::new()
	}
}

impl StringTable {
	/// Create a new empty string table
	pub fn new() -> Self {
		Self {
			table: HashMap::new(),
			next_offset: 0,
			data_section: DataSection::new(),
		}
	}

	/// Allocate a string in linear memory, returning (ptr, len)
	/// Deduplicates strings - if the string is already allocated, returns existing offset
	pub fn allocate(&mut self, s: &str) -> (u32, u32) {
		if let Some(&offset) = self.table.get(s) {
			return (offset, s.len() as u32);
		}
		let offset = self.next_offset;
		let bytes = s.as_bytes();
		self.data_section
			.active(0, &ConstExpr::i32_const(offset as i32), bytes.iter().copied());
		self.table.insert(s.to_string(), offset);
		self.next_offset += bytes.len() as u32;
		(offset, bytes.len() as u32)
	}

	/// Recursively collect and allocate all strings from a Node tree
	/// Also updates scope with string variable data (for WASI puts)
	pub fn collect_from_node(&mut self, node: &Node, scope: &mut crate::analyzer::Scope) {
		let node = node.drop_meta();
		match node {
			Node::Text(s) | Node::Symbol(s) => {
				self.allocate(s);
			}
			Node::Key(key, op, value) => {
				// Track string variable assignments: x='hello' or x:='hello'
				// Store data pointer/length in the Local for later use by WASI puts
				if matches!(op, Op::Assign | Op::Define) {
					if let Node::Symbol(var_name) = key.drop_meta() {
						if let Node::Text(s) = value.drop_meta() {
							let (ptr, len) = self.allocate(s);
							scope.set_local_data(var_name, ptr, len);
						}
					}
				}
				self.collect_from_node(key, scope);
				self.collect_from_node(value, scope);
			}
			Node::List(items, _, _) => {
				for item in items {
					self.collect_from_node(item, scope);
				}
			}
			Node::Data(dada) => {
				self.allocate(&dada.type_name);
			}
			Node::Type { name, body } => {
				self.collect_from_node(name, scope);
				self.collect_from_node(body, scope);
			}
			_ => {}
		}
	}

	/// Get a reference to the data section (for adding to WASM module)
	pub fn data_section(&self) -> &DataSection {
		&self.data_section
	}

	/// Get the current data offset (for tracking memory usage)
	pub fn next_offset(&self) -> u32 {
		self.next_offset
	}

	/// Get a reference to the underlying table
	pub fn table(&self) -> &HashMap<String, u32> {
		&self.table
	}
}
