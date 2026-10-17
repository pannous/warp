//! WIT flags (wiki examples.md, wasm.md `(flags <name>*) ≡ (record (field <name> bool)*)`): `flags virtues={fast, safe}`
//! declares the flags type, `virtues goal = fast+safe` a record with those flags true and the others false
use crate::common::fails_with;
use crate::is;

#[test]
fn test_flags_set_their_named_members() {
	is!("flags virtues={fast, safe, cheap}; virtues goal = fast+safe; goal.fast", 1);
	is!("flags virtues={fast, safe, cheap}; virtues goal = fast+safe; goal.cheap", 0);
	is!("flags virtues {fast safe}; virtues goal = safe; goal.safe + goal.fast", 1);
	is!("flags virtues={fast, safe}; virtues goal = fast|safe; goal.safe", 1);
}

#[test]
fn test_the_flags_type_is_the_empty_set() {
	is!("flags virtues={fast, safe}; virtues.fast", 0);
}

#[test]
fn test_an_unknown_flag_is_an_error() {
	fails_with("flags virtues={fast, safe}; virtues goal = quick", "virtues has no flag quick");
}

#[test]
fn test_wit_flags_parse_like_the_declaration() {
	use warp::warp_parser::{ParserOptions, WarpParser};
	let wit = WarpParser::parse_with_options("flags permissions {\n read,\n write,\n}", ParserOptions::wit());
	assert_eq!(wit.serialize().matches("read").count(), 1);
	assert_eq!(wit.size(), 3);
	is!("flags permissions {read, write}; permissions p = write; p.write - p.read", 1);
}
