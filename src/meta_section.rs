//! The module's own metadata (notes/reflection.md "wasp.meta layout"): ONE custom section `wasp.meta`, a wasp map in
//! notation text, read back with the data parser. Entries: `units`, the units of main's result (static_units.rs).
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

pub const META_SECTION: &str = "wasp.meta";

/// The module with a `wasp.meta` section holding these entries
pub fn with_entries(mut bytes: Vec<u8>, entries: Vec<(&str, Node)>) -> Vec<u8> {
	let map = Node::List(entries.into_iter().map(|(key, value)| Node::Key(Box::new(Node::Symbol(key.into())), Op::Colon, Box::new(value))).collect(), Bracket::Curly, Separator::Space);
	let section = wasm_encoder::CustomSection { name: META_SECTION.into(), data: map.serialize().into_bytes().into() };
	wasm_encoder::Section::append_to(&section, &mut bytes);
	bytes
}

/// The entry `key` of the module's `wasp.meta` section
pub fn entry(bytes: &[u8], key: &str) -> Option<Node> {
	let data = wasmparser::Parser::new(0).parse_all(bytes).filter_map(Result::ok).find_map(|payload| match payload {
		wasmparser::Payload::CustomSection(section) if section.name() == META_SECTION => Some(section.data().to_vec()),
		_ => None,
	})?;
	let map = crate::wasp_parser::parse_data(&String::from_utf8(data).ok()?);
	match map[key].drop_meta() {
		Node::Empty => None,
		value => Some(value.clone()),
	}
}
