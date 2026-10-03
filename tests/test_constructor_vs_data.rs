//! D4 (user, 2026-10-03): "Distinguish". `T{…}` with a known type T constructs and validates the instance,
//! `T:{…}` is plain data; the two are not equal (wiki/constructor.md, data.md "Significant colon").

use crate::common::fails_with;
use warp::is;
use warp::wasm_emitter::eval;

const POINT: &str = "class point{x:int y:int}; ";

fn with_point(code: &str) -> String {
	format!("{POINT}{code}")
}

#[test]
fn test_construction_reads_its_fields() {
	is!(&with_point("p = point{x:1 y:2}; p.y"), 2);
	is!("class contact{name email?}; c = contact{name:'Jo'}; c.name", "Jo");
}

#[test]
fn test_construction_is_not_data() {
	is!(&with_point("point{x:1 y:2} == (point:{x:1 y:2})"), false);
	is!(&with_point("p = point{x:1 y:2}; q = point:{x:1 y:2}; p == q"), false);
	is!(&with_point("(point:{x:1 y:2}) == (point:{x:1 y:2})"), true);
	is!(&with_point("p = point{x:1 y:2}; q = point{x:1 y:2}; p == q"), true);
}

#[test]
fn test_construction_rejects_an_undeclared_field() {
	fails_with(&with_point("point{x:1 z:2}"), "point has no field z");
}

#[test]
fn test_construction_checks_field_types() {
	fails_with(&with_point("point{x:1 y:'a'}"), "point.y is int");
}

#[test]
fn test_data_is_never_validated() {
	is!(&with_point("p = point:{x:1 z:2}; p.z"), 2);
}
