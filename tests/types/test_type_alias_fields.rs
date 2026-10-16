// Type aliases (card types-alias, samples/types.warp): `type Name = string` names the type string wherever a type is
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
	let sample = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/samples/types.warp")).unwrap();
	let printed = crate::common::printed(&sample.replace("match color", "color = rgb(1, 2, 3)\nmatch color"));
	assert_eq!(printed.trim(), "RGB: 1, 2, 3");
}

#[test]
fn is_tests_the_type_an_alias_names() {
	is!("type Age = int\nx = 3\nx is Age", true);
	is!("type Age = int\nx = \"a\"\nx is Age", false);
	is!("type Age = int\nx = 3\nx is an Age", true);
	is!("type Years = Age\ntype Age = int\nx = 3\nx is Years", true);
	is!("enum Color { red green }\ntype Paint = Color\nc = red\nc is Paint", true);
	is!("enum Color { red green }\ntype Paint = Color\nc = 3\nc is Paint", false);
}

/// card alias-is-literal: a literal is tested against the type an alias names, as a name holding it is
#[test]
fn a_literal_is_tested_against_the_type_an_alias_names() {
	is!("type Age = int\n30 is Age", true);
	is!("type Age = int\n\"a\" is Age", false);
	is!("type Name = text\n\"a\" is a Name", true);
	is!("enum Color { red green }\ntype Paint = Color\n3 is Paint", false);
}
