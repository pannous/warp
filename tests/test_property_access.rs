//! Property access on objects: `o.a`, `a of o`, `o's a` and `o["a"]` are one lookup; a missing key is a loud error value
use warp::*;
mod common;
use common::fails_with;

#[test]
fn test_dot_on_an_object_literal() {
	is!("{a:1 b:2}.a", 1);
	is!("{a:1 b:2}.b", 2);
}

#[test]
fn test_dot_on_a_variable_and_nested() {
	is!("p={a:1 b:2}; p.a", 1);
	is!("p={a:1 b:{c:3}}; p.b.c", 3);
	is!("p={name:\"Joe\"}; p.name", "Joe");
}

#[test]
fn test_of_and_possessive() {
	is!("p={name:\"Joe\"}; name of p", "Joe");
	is!("p={name:\"Joe\"}; p's name", "Joe");
	is!("p={a:1 b:{c:3}}; c of b of p", 3);
}

#[test]
fn test_bracket_lookup_by_text_never_traps() {
	is!("p={name:\"Joe\"}; p[\"name\"]", "Joe");
	is!("p={name:\"Joe\"}; p[name]", "Joe");
	is!("{a:1 b:2}[\"b\"]", 2);
}

#[test]
fn test_missing_key_is_a_loud_error() {
	fails_with("p={a:1}; p.x", "no field x");
	fails_with("p={a:1}; x of p", "no field x");
	fails_with("p={a:1}; p[\"x\"]", "no field x");
	fails_with("p={a:1 b:2}; p.b.c", "no field c");
	fails_with("{a:1}.zzz", "no field zzz");
}

#[test]
fn test_field_names_win_over_library_words_on_an_object() {
	is!("p={first:\"A\" last:\"B\"}; p.last", "B");
	is!("p={count:7}; p.count", 7);
	is!("p={a:1 b:2}; p.length", 2); // no field of that name: the counting word
}

#[test]
fn test_non_objects_keep_the_undefined_function_rule() {
	fails_with("x=5; x.foo", "undefined function: foo");
	fails_with("x=\"hello\"; x.shout", "undefined function: shout");
	fails_with("x=[1 2 3]; x.frobnicate", "undefined function: frobnicate");
}

#[test]
fn test_object_data_stays_data() {
	is!("p={a:1 b:2}; p.a + p.b", 3);
	assert_eq!(warp::wasm_emitter::eval("{a:1 b:2}").serialize(), "{a:1 b:2}");
}
