//! Type annotations: the kind a type word, a declared `x:T` or a declared field gives

use super::*;
use crate::words::{ANY_TYPE, MAP_WORD, NUMBER_WORD};

/// `number` is the exact numeric tower (Int); the other builtin type names have their own kind;
/// a list type (`list`, `list of int`, `[int]`, `int[]`, `ints`) is a List
pub(crate) fn annotated_kind(type_node: &Node) -> Option<Kind> {
	if crate::type_constructor::instance_parts_marked(type_node) {
		return Some(Kind::Key); // `p:person` of a declared type (traits::lower_conformances): an instance
	}
	if crate::numbers::uncertain::is_interval_number(type_node) {
		return Some(Kind::Empty); // `a:number` given a ± value: a Node, checked at run time (wasm_emitter admitted)
	}
	let type_name = type_node.name();
	if bracketed_list_type(type_node).is_some() || names_list_type(&type_name) {
		return Some(Kind::List);
	}
	// `v:any`, as an untyped field: any value, held as a Node (lib/json.warp's to_json takes what parse_json gives)
	// `x: int or text` (an inline union, card inline-union) and `s: Circle or Square` (card union-types) likewise
	if names_empty_type(type_node) {
		return Some(Kind::Empty);
	}
	// `m:map`, `m: map of int`: a map, held as a Node; `{a:1}` given to it is upgraded to the map of its entry (one_entry_maps.rs)
	if type_name == ANY_TYPE || is_map_type(&type_name) || union_parts(&type_name).is_some() {
		return Some(Kind::Empty);
	}
	type_word_kind(&type_name)
}

pub(crate) fn is_map_type(type_name: &str) -> bool {
	type_name == MAP_WORD || type_name.starts_with(super::checks::MAP_TYPE_PREFIX)
}

/// `x:ø` / `x:nil` / `x:unit`: the empty type, ø is held as a Node
pub(super) fn names_empty_type(type_node: &Node) -> bool {
	matches!(type_node.drop_meta(), Node::Empty)
		|| crate::type_tests::canonical_spec_word(&type_node.name()) == crate::type_tests::EMPTY_TYPE
}

pub(crate) fn names_list_type(type_name: &str) -> bool {
	type_name == "list" || super::list_element_type(type_name).is_some()
}

/// The key under which Scope::function_kinds holds the declared kind of a field (as tuples::element_key holds a
/// tuple element's): `v.x` reads a float when every declared type with a field x declares it `float`
pub fn field_kind_key(field: &str) -> String {
	format!(".{field}")
}

/// The fields of the declared types whose declarations all agree on one builtin kind, keyed by field_kind_key
pub fn declared_field_kinds(registry: &crate::type_kinds::TypeRegistry) -> HashMap<String, Kind> {
	let mut kinds: HashMap<String, Option<Kind>> = HashMap::new();
	for field in registry.types().iter().flat_map(|type_def| &type_def.fields) {
		let type_name = field.type_name.to_lowercase();
		// a number field holds an int or a float: read at run time (card number-field)
		let kind = type_word_kind(&type_name).filter(|_| type_name != NUMBER_WORD);
		kinds
			.entry(field_kind_key(&field.name))
			.and_modify(|known| {
				if *known != kind {
					*known = None
				}
			})
			.or_insert(kind);
	}
	kinds.into_iter().filter_map(|(key, kind)| Some((key, kind?))).collect()
}

/// The declared field kinds of a program
pub(super) fn program_field_kinds(program: &Node) -> HashMap<String, Kind> {
	let mut registry = crate::type_kinds::TypeRegistry::new();
	collect_all_types(&mut registry, program);
	declared_field_kinds(&registry)
}

/// The kind a builtin type word names, as annotation (`x:double`) or constructor (`double 2`)
pub fn type_word_kind(type_name: &str) -> Option<Kind> {
	match type_name {
		NUMBER_WORD => Some(Kind::Int),
		_ => builtin_type_kind(type_name),
	}
}
