//! Meta information on objects (user decision 2026-10-03, notes/open_decisions.md "Meta information on objects"):
//! `@key:value` inside a literal, `x.@key` read and write, glued `x@key` in code, attributes survive emission,
//! and `x.key` falls back to the meta key `@key` when x has no field `key`.

use crate::common::fails_with;
use warp::is;
use warp::wasm_emitter::eval;
use warp::wasp_parser::{parse, parse_data};

const POINT: &str = "class point{x:int y:int}; ";

fn with_point(code: &str) -> String {
	format!("{POINT}{code}")
}

#[test]
fn test_meta_key_inside_a_literal_is_no_field() {
	is!("p = {x:1 y:2 @source:\"gps\"}; p.@source", "gps");
	is!("p = {x:1 y:2 @source:\"gps\"}; #p", 2);
	is!("p = {x:1 y:2 @source:\"gps\"}; q = {x:1 y:2}; p == q", true);
	is!(&with_point("p = point{x:1 y:2 @source:\"gps\"}; p.y"), 2);
	is!(&with_point("p = point{x:1 y:2 @source:\"gps\"}; p.@source"), "gps");
	is!(&with_point("p = point{x:1 y:2 @source:\"gps\"}; q = point{x:1 y:2}; p == q"), true);
}

#[test]
fn test_meta_entries_survive_eval() {
	assert_eq!(eval("{x:1 @source:\"gps\"}").serialize(), "{x:1 @source:\"gps\"}");
}

#[test]
fn test_meta_key_in_parsed_data() {
	let point = parse("point{x:1 y:2 @source:\"gps\"}");
	assert_eq!(point["@source"], "gps");
	assert_eq!(point["y"], 2);
	assert_eq!(parse("@source(\"gps\") point{x:1}")["@source"], "gps");
}

#[test]
fn test_dot_at_reads_and_writes_meta() {
	is!("p = {x:1}; p.@source = \"gps\"; p.@source", "gps");
	is!("p = {x:1}; p.@source = \"gps\"; p.x", 1);
	is!("p = {x:1}; p.@source = \"gps\"; #p", 1);
}

#[test]
fn test_glued_at_reads_meta_in_code() {
	is!("p = {x:1 @unit:\"cm\"}; p@unit", "cm");
}

#[test]
fn test_at_in_data_stays_text() {
	let mail = parse_data("mail: info@pannous.com");
	assert_eq!(mail["mail"].serialize(), "info@pannous.com");
}

#[test]
fn test_prefix_attribute_survives_emission() {
	is!("p = @unit(\"cm\") {x:1}; p.@unit", "cm");
}

#[test]
fn test_field_first_then_meta_fallback() {
	is!("p = {x:1 @source:\"gps\"}; p.source", "gps");
	is!("p = {source:\"field\" @source:\"gps\"}; p.source", "field");
	fails_with(&with_point("p = point{x:1 y:2}; p.source"), "no field source");
}
