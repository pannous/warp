//! D9 (user, 2026-10-03): every function has a suffix form (`square 2 == 2 squared`, wiki/function.md); an ungrouped mix
//! with infix operators (`1+2 squared`) asks `1+(2 squared)` or `(1+2) squared`
use crate::is;
use crate::common::fails_with;

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

#[test] // samples/sudoku.warp: `solved = solve(c); return solved` read as `solve(return)`
fn a_variable_is_no_suffix_form() {
	is!("def solve(n) { if n > 2 { return n }; solved = solve(n + 1); return solved }; solve(0)", 3);
}

#[test]
fn library_suffix_words() {
	crate::is!("3 squared", 9);
	crate::is!("2 cubed", 8);
	crate::is!("xs=[3 1 2]; xs sorted", warp::ints(vec![1, 2, 3]));
	crate::is!("[1 2] reversed", warp::ints(vec![2, 1]));
	crate::is!("x = 4; x squared + 1", 17);
}

#[test]
fn a_user_function_or_variable_takes_the_suffix_word() {
	crate::is!("square(x):=x*x*10; 3 squared", 90);
	crate::is!("sorted = 3; sorted", 3);
}

#[test] // card suffix-after-assign: `x = 7 squared` gave 7, the word applied to the whole assignment and was dropped
fn a_suffix_word_applies_to_the_assigned_value() {
	is!("x = 7 squared; x", 49);
	is!("x = 7 squared + 1; x", 50);
	is!("x := 3 squared; x", 9);
	is!("xs = [3, 1, 2] sorted; xs", warp::ints(vec![1, 2, 3]));
	is!("x = 2; x += 3 squared; x", 11);
	fails_with("x = 1 + 2 squared; x", "does `squared`");
}
