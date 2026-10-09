//! `record`, `struct` and `class` declare the same kind of type: constructor call, field access, optional fields

use crate::common::fails_with;
use warp::Node;
use crate::{is, eq};
use warp::wasm_emitter::eval;

#[test]
fn a_record_is_a_struct() {
	is!("record point{x:int y:int}; point(1,2).y", 2);
	is!("record point{x:int y:int}; p=point(1,2); p.x+p.y", 3);
}

#[test]
fn a_class_has_untyped_fields() {
	is!("class contact {name email}; c=contact(\"Joe\" \"j@x\"); c.name", "Joe");
	is!("class contact {name email}; contact(\"Joe\" \"j@x\").email", "j@x");
	is!("class pair {a b}; p=pair(1 2); p.a+p.b", 3);
}

#[test]
fn an_optional_field_defaults_to_null() {
	eq!(eval("class contact {name email?}; contact(\"Joe\").email"), Node::Empty);
	is!("class contact {name email?}; contact(\"Joe\" \"j@x\").email", "j@x");
	is!("class contact {name email?}; contact(\"Joe\").name", "Joe");
}

#[test]
fn a_missing_required_field_is_an_error() {
	fails_with("class contact {name email}; contact(\"Joe\")", "contact takes");
	fails_with("class contact {name email?}; contact(1 2 3)", "contact takes");
}

#[test]
fn an_enum_names_its_cases_by_index() {
	is!("enum color {red green blue}; color.red", 0);
	is!("enum color {red green blue}; color.green", 1);
	is!("enum color {red green blue}; color.blue", 2);
}

#[test]
fn record_stays_usable_as_a_variable_name() {
	is!("record=5; record+1", 6);
	is!("value_of(record):=record*2; value_of(4)", 8);
}

/// wiki/class.md: a nested block of fields `address { street … }` declares the field address holding them
#[test]
fn a_class_declares_a_nested_block_of_fields() {
	let contact = "class contact {\n name\n email?\n phone?\n address {\n  street\n  city\n  zip?\n }\n}\n";
	is!(&format!("{contact}c = contact{{name:\"Ann\" address:{{street:\"Main\" city:\"X\"}}}}\nc.address.city"), "X");
	is!(&format!("{contact}c = contact{{name:\"Ann\" address:{{street:\"Main\" city:\"X\"}}}}\nc.name"), "Ann");
}

#[test]
fn a_constructor_takes_named_arguments_as_functions_do() {
	// card named-constructor: `P(y=2, x=7)` as `f(b=1, a=5)` (P37), in any order, mixed with positional ones
	is!("class P { x: int; y: int }; p = P(y=2, x=7); p.x * 10 + p.y", 72);
	is!("class P { x: int; y: int }; p = P(7, y=2); p.x * 10 + p.y", 72);
	is!("class P { x: int; y: int }; p = P(y: 2, x: 7); p.x * 10 + p.y", 72);
	fails_with("class P { x: int; y: int }; P(z=2, x=7)", "no field z");
	is!("class P { x: int; y = 5 }; p = P(x=7); p.x * 10 + p.y", 75); // no Java declaration `P x = 7`
}
