//! wiki/charged.md (released 2026-10-05), stage 1: `x : e` is the uncharged block e, `x!`/`x!!` run it where they are written
//! (constant blocks are resolved at compile time), a bare `x` is the block, `x` as a value is a type error with the fix,
//! and `data e` is e as written
use crate::common::fails_with;
use warp::diagnostic::take_warnings;
use warp::is;
use warp::wasm_emitter::eval;

#[test]
fn test_bang_runs_a_block() {
	is!("a=1;b=2; x : a+b; x!", 3);
	is!("x : 1+2; x!!", 3);
	is!("f(x) := x * 2; w : f(3); w!", 6);
}

#[test]
fn test_names_in_a_block_resolve_where_it_runs() {
	is!("y = 3; w : y*y; y = 4; w!", 16);
}

#[test]
fn test_a_block_is_no_value() {
	assert_eq!(eval("x : 1+2; x").serialize().trim(), "1+2");
	fails_with("x : 1+2; x + 1", "x is a block (1+2), no value: run it with x!");
}

#[test]
fn test_literals_and_objects_after_colon_keep_their_meaning() {
	is!("x : 3; x + 1", 4);
	is!("person: {name: \"Alice\"}; person.name", "Alice");
	is!("x : 1+2; x = 5; x + 1", 6);
}

#[test]
fn test_a_computed_colon_warns_where_it_is_written() {
	take_warnings();
	eval("x : 1+2; x!");
	assert!(take_warnings().iter().any(|warning| warning.message.contains("write x = 1+2 for its value")));
}

#[test]
fn test_data_is_the_expression_as_written() {
	assert_eq!(eval("x = data 1+2; x").serialize().trim(), "1+2");
}
