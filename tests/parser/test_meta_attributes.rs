//! Meta information on objects (user decision 2026-10-03, notes/open_decisions.md "Meta information on objects"):
//! `@key:value` inside a literal, `x.@key` read and write, glued `x@key` in code, attributes survive emission,
//! and `x.key` falls back to the meta key `@key` when x has no field `key`.

use crate::common::fails_with;
use crate::is;
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

// Round 3 (user decision "Skip @ keys"): iteration, keys, values, entries and every count ignore meta entries

const WITH_META: &str = "p = {x:1 y:2 @s:30}; ";

fn with_meta(code: &str) -> String {
	format!("{WITH_META}{code}")
}

#[test]
fn test_iteration_skips_meta_entries() {
	is!(&with_meta("n=0; for k in p {n+=1}; n"), 2);
	is!(&with_meta("t=0; for k,v in p {t+=v}; t"), 3);
	is!("p = {x:1}; p.@s = 30; t=0; for k,v in p {t+=v}; t", 1);
}

#[test]
fn test_keys_values_entries_skip_meta_entries() {
	is!(&with_meta("#p.keys"), 2);
	is!(&with_meta("sum(p.values)"), 3);
	is!(&with_meta("#map_entries(p)"), 2);
}

#[test]
fn test_counts_and_positions_skip_meta_entries() {
	is!(&with_meta("p.count"), 2);
	is!(&with_meta("p.length"), 2);
	is!(&with_meta("count(p)"), 2);
	is!(&with_meta("#p"), 2);
	fails_with(&with_meta("p#3"), "index out of range");
}

#[test]
fn test_meta_stays_reachable() {
	is!(&with_meta("p.@s"), 30);
	is!(&with_meta("p@s"), 30);
	is!(&with_meta("p[\"@s\"]"), 30);
}

#[test]
fn test_meta_entries_stay_behind_the_fields() {
	is!("p = {x:1 @s:30 y:2}; p#2", eval("y:2"));
	is!("p = {x:1 @s:30 y:2}; t=0; for k,v in p {t+=v}; t", 3);
	is!("p = {x:1 @s:30}; p.z = 5; p#2", eval("z:5"));
	is!("p = {@s:30}; p.z = 5; p#1", eval("z:5"));
	is!("p = {x:1}; p.@s = 30; p.z = 5; #p.keys", 2);
}
