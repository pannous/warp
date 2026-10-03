//! D9 (user, 2026-10-03): every function has a suffix form (`square 2 == 2 squared`, wiki/function.md); an ungrouped mix
//! with infix operators (`1+2 squared`) asks `1+(2 squared)` or `(1+2) squared`
use crate::common::fails_with;
use warp::*;

const SQUARE: &str = "square:=it*it; ";

fn with_square(code: &str) -> String {
	format!("{SQUARE}{code}")
}

#[test]
fn a_function_name_with_ed_is_its_suffix_form() {
	is!(&with_square("3 squared"), 9);
	is!(&with_square("(1+2) squared"), 9);
	is!(&with_square("1+(2 squared)"), 5);
	is!("halve:=it/2; 8 halved", 4);
	is!(&with_square("x=4; x squared"), 16);
	is!(&with_square("2 squared+1"), 5);
}

#[test]
fn an_ungrouped_mix_asks() {
	fails_with(&with_square("1+2 squared"), "(too ambiguous to guess)");
	fails_with(&with_square("1+2 squared"), "`1+(2 squared)` for squared binds to 2");
	fails_with(&with_square("1+2 squared"), "`(1+2) squared` for squared applies to 1+2");
}

#[test]
fn each_reading_has_its_explicit_form() {
	is!(&with_square("1+(2 squared)"), 5);
	is!(&with_square("(1+2) squared"), 9);
}

#[test]
fn an_unknown_word_is_no_suffix_form() {
	assert!(warp::wasm_emitter::eval("3 painted").serialize().contains("painted"));
}

#[test] // samples/sudoku.wasp: `solved = solve(c); return solved` read as `solve(return)`
fn a_variable_is_no_suffix_form() {
	is!("def solve(n) { if n > 2 { return n }; solved = solve(n + 1); return solved }; solve(0)", 3);
}
