// Type aliases (card types-alias, samples/types.wasp): `type Name = string` names the type string wherever a type is
// written: fields, optional fields, parameters, annotations
use crate::is;

#[test]
fn alias_as_field_type() {
	is!("type Name = string; type P: { name: Name }; p = P{name: \"Al\"}; p.name", "Al");
	is!("type Age = int; type Years = Age; class P { age: Years }; P(3).age + 1", 4);
}

#[test]
fn alias_as_optional_field_and_parameter() {
	is!("type Email = string; type P: { name: string; email: Email? }; p = P{name: \"Al\"}; p.name", "Al");
	is!("type Age = int; older(a: Age) := a + 1; older(41)", 42);
}

/// the whole sample, its color given
#[cfg(feature = "native")]
#[test]
fn types_sample_runs() {
	let sample = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/samples/types.wasp")).unwrap();
	let printed = crate::common::printed(&sample.replace("match color", "color = rgb(1, 2, 3)\nmatch color"));
	assert_eq!(printed.trim(), "RGB: 1, 2, 3");
}
