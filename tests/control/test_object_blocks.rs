//! wiki/charged.md section 4, stage 2: `{statements}` assigned is a block, an object's computed `key: value` entry is an
//! uncharged block (`o.s1!` runs it, `o.s1 + 1` is a type error), `key = value` / `key := value` entries are values, and `obj!` runs an
//! object as code: each `key: value` is the call `key(value)` (`help!`)
use crate::common::fails_with;
use warp::diagnostic::take_warnings;
use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn test_statements_in_braces_are_a_block() {
	is!("x = {1+2}; x!", 3);
	assert_eq!(eval("x = {1+2}; x").serialize().trim(), "1+2");
	is!("a = 2; x = {a*10}; a = 3; x!", 30);
}

#[test]
fn test_computed_entries_are_uncharged() {
	is!("a=1;b=2; o = {s1: a+b, s3 = a+b}; o.s3", 3);
	is!("a=1;b=2; o = {s1: a+b}; o.s1!", 3);
	fails_with("a=1;b=2; o = {s1: a+b}; o.s1 + 1", "o.s1 is a block (a+b), no value");
	is!("o = {age: 3, color: red}; o.age", 3);
}

#[test]
fn test_a_computed_entry_warns_where_it_is_written() {
	take_warnings();
	eval("a=1;b=2; o = {s1: a+b}; o.s1!");
	assert!(take_warnings().iter().any(|warning| warning.message.contains("write s1 = a+b for its value")));
	take_warnings();
	eval("o = {age: 3, color: red}; o.age");
	assert!(take_warnings().iter().all(|warning| !warning.message.contains("keeps the block")));
}

#[test]
fn test_an_object_runs_as_code() {
	is!("help = {print: \"there is help\"}; help!", "there is help");
}

#[test]
fn test_charged_object_fields_are_value_now() {
	// g-qUmA: object `:=` is value-now like `=` (top-level P71 := unchanged)
	is!("o = {a: 1, s := clock()}; o.a", 1);
	is!("o = {a: 1, s := clock()}; o.s > 1700000000000", true);
}
