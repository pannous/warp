//! The module's data: text literals in one passive segment, which array.new_data turns into $Text arrays, and the
//! bytes that live in linear memory (lookup tables, and texts handed to the host by address) in active segments

use crate::node::Node;
use crate::operators::Op;
use std::collections::HashMap;
use wasm_encoder::{ConstExpr, DataSection};

/// Bytes at the start of linear memory reserved for the WASI iovec and nwritten slots (see `emit_wasi_puts`)
const WASI_SCRATCH_BYTES: u32 = 16;
/// The index of the passive segment holding the text literals: the first segment
pub const TEXT_SEGMENT: u32 = 0;

pub struct StringTable {
	/// Offsets of the text literals in the passive segment
	literals: HashMap<String, u32>,
	literal_bytes: Vec<u8>,
	/// Offsets in linear memory of the bytes placed there, by text or by "\0" + table name
	in_memory: HashMap<String, u32>,
	memory_segments: Vec<(u32, Vec<u8>)>,
	/// Next free offset in linear memory
	next_offset: u32,
}

impl Default for StringTable {
	fn default() -> Self {
		Self {
			literals: HashMap::new(),
			literal_bytes: vec![],
			in_memory: HashMap::new(),
			memory_segments: vec![],
			next_offset: WASI_SCRATCH_BYTES,
		}
	}
}

impl StringTable {
	/// The text `s` in the literal segment: (offset, length) for array.new_data, deduplicated
	pub fn literal(&mut self, s: &str) -> (u32, u32) {
		let length = s.len() as u32;
		if let Some(&offset) = self.literals.get(s) {
			return (offset, length);
		}
		let offset = self.literal_bytes.len() as u32;
		self.literal_bytes.extend_from_slice(s.as_bytes());
		self.literals.insert(s.to_string(), offset);
		(offset, length)
	}

	/// The text `s` in linear memory, for a host that reads it by address: (ptr, len), deduplicated
	pub fn in_memory(&mut self, s: &str) -> (u32, u32) {
		(self.place(s.to_string(), s.as_bytes()), s.len() as u32)
	}

	/// Binary data (a lookup table) in linear memory under a name that no text can have, returning its offset
	pub fn allocate_bytes(&mut self, name: &str, bytes: &[u8]) -> u32 {
		self.place(format!("\0{name}"), bytes)
	}

	fn place(&mut self, key: String, bytes: &[u8]) -> u32 {
		if let Some(&offset) = self.in_memory.get(&key) {
			return offset;
		}
		let offset = self.next_offset;
		self.memory_segments.push((offset, bytes.to_vec()));
		self.in_memory.insert(key, offset);
		self.next_offset += bytes.len() as u32;
		offset
	}

	/// Collect the text literals of a Node tree; a text assigned to a variable is placed in memory too, where WASI
	/// puts and FFI read it by address (the Local's data pointer)
	pub fn collect_from_node(&mut self, node: &Node, scope: &mut crate::analyzer::Scope) {
		let node = node.drop_meta();
		match node {
			Node::Text(s) | Node::Symbol(s) => {
				self.literal(s);
			}
			Node::Key(key, op, value) => {
				if matches!(op, Op::Assign | Op::Define) {
					if let (Node::Symbol(var_name), Node::Text(s)) = (key.drop_meta(), value.drop_meta()) {
						let (ptr, len) = self.in_memory(s);
						scope.set_local_data(var_name, ptr, len);
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
				self.literal(&dada.type_name);
			}
			Node::Type { name, body } => {
				self.collect_from_node(name, scope);
				self.collect_from_node(body, scope);
			}
			_ => {}
		}
	}

	/// The data section: the literal segment first (TEXT_SEGMENT), then the memory segments
	pub fn data_section(&self) -> DataSection {
		let mut section = DataSection::new();
		section.passive(self.literal_bytes.iter().copied());
		for (offset, bytes) in &self.memory_segments {
			section.active(0, &ConstExpr::i32_const(*offset as i32), bytes.iter().copied());
		}
		section
	}

	/// The number of data segments, for the data count section array.new_data needs
	pub fn segment_count(&self) -> u32 {
		1 + self.memory_segments.len() as u32
	}

	/// The bytes placed in memory take this much: the module's memory holds at least these
	pub fn next_offset(&self) -> u32 {
		self.next_offset
	}
}
