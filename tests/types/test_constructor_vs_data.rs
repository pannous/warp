//! D4 (user, 2026-10-03): "Distinguish". `T{…}` with a known type T constructs and validates the instance,
//! `T:{…}` is plain data; the two are not equal (wiki/constructor.md, data.md "Significant colon").

use crate::common::fails_with;
use crate::is;

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

#[test]
fn test_construction_needs_every_required_field() {
	fails_with(&with_point("point{x:1}"), "point needs field y");
	fails_with("class person{name!; email?}; person{email:'a@b'}", "person needs field name");
}

#[test]
fn test_construction_fills_defaults_and_leaves_optional_fields_out() {
	is!("class counter{n=0; step:int=1}; c = counter{}; c.step", 1);
	is!("class counter{n=0; step:int=1}; c = counter{n:5}; c.n", 5);
	is!("class contact{name; email?}; c = contact{name:'Jo'}; c.name", "Jo");
	is!("class counter{n=0; step:int=1}; c = counter(5); c.step", 1);
}

#[test]
fn test_an_unknown_word_with_a_block_stays_data() {
	is!("(a{x:1}) == (a:{x:1})", true);
}

/// `email?: text` (samples/types.wasp, the fix the 'needs field' error suggests) is the typed optional field, not the
/// elvis `email ?: text`
#[test]
fn test_a_typed_optional_field_puts_the_question_mark_on_its_name() {
	is!("class contact{name; email?: text}; c = contact{name:'Jo'}; c.name", "Jo");
	is!("class contact{name; email?: text}; c = contact{name:'Jo' email:'j@x'}; c.email", "j@x");
	fails_with("class contact{name; email?: text}; contact{name:'Jo' email:3}", "contact.email is text");
}
