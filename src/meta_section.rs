//! The module's own metadata (notes/reflection.md "warp.meta layout"): ONE custom section `warp.meta`, a warp map in
//! notation text, read back with the data parser. Entries: `units`, the units of main's result (static_units.rs);
//! `classes`, each class's fields and methods (reflection.rs), which an importer of the module reflects.
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

pub const META_SECTION: &str = "warp.meta";

/// A warp map of these entries, `{key: value …}`
pub fn map<K: Into<String>>(entries: impl IntoIterator<Item = (K, Node)>) -> Node {
	Node::List(entries.into_iter().map(|(key, value)| Node::Key(Box::new(Node::Symbol(key.into())), Op::Colon, Box::new(value))).collect(), Bracket::Curly, Separator::Space)
}

/// The module with a `warp.meta` section holding these entries
fn with_entries(mut bytes: Vec<u8>, entries: Vec<(&str, Node)>) -> Vec<u8> {
	let section = wasm_encoder::CustomSection { name: META_SECTION.into(), data: map(entries).serialize().into_bytes().into() };
	wasm_encoder::Section::append_to(&section, &mut bytes);
	bytes
}

/// The module with the entries the last lowered program left: its result's units, its classes
pub fn with_program_entries(bytes: Vec<u8>) -> Vec<u8> {
	let entries: Vec<(&str, Node)> = [
		(crate::units::static_units::UNITS_ENTRY, crate::units::static_units::result_units_entry()),
		(crate::reflection::CLASSES_ENTRY, crate::reflection::take_classes_entry()),
	].into_iter().filter_map(|(key, value)| Some((key, value?))).collect();
	match entries.is_empty() {
		true => bytes,
		false => with_entries(bytes, entries),
	}
}

/// The entry `key` of the module's `warp.meta` section
pub fn entry(bytes: &[u8], key: &str) -> Option<Node> {
	let data = wasmparser::Parser::new(0).parse_all(bytes).filter_map(Result::ok).find_map(|payload| match payload {
		wasmparser::Payload::CustomSection(section) if section.name() == META_SECTION => Some(section.data().to_vec()),
		_ => None,
	})?;
	let map = crate::warp_parser::parse_data(&String::from_utf8(data).ok()?);
	match map[key].drop_meta() {
		Node::Empty => None,
		value => Some(value.clone()),
	}
}
