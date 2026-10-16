//! A unit after a variable is no quantity, units bind only after a number (notes/decisions.md): a constructor refusing
//! `D(n inch)` names the way that works, `n * 1 inch` (card diagonal-holds)
use crate::common::fails_with;
use warp::wasm_emitter::eval;

const DISPLAY: &str = "class D{diagonal: inch}\nn = 96";

#[test]
fn a_unit_after_a_variable_hints_the_multiplication() {
	fails_with(&format!("{DISPLAY}\nD(n inch)"), "write n * 1 inch");
	fails_with(&format!("{DISPLAY}\nD{{diagonal: n inch}}"), "write n * 1 inch");
	assert_eq!(eval(&format!("{DISPLAY}\nD(n * 1 inch).diagonal")).serialize().trim(), "96inch");
}
