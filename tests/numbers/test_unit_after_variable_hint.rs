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

#[test]
fn a_unit_after_a_variable_holding_a_number_is_refused_everywhere() {
	fails_with("n = 3\ny = n km\ny", "write n * 1 km");
	fails_with("n = 96\nx: inch = n inch\nx", "write n * 1 inch");
	fails_with("f(n) := n km\nf(2)", "write n * 1 km");
	assert_eq!(eval("n = 3\nn * 1 km").serialize().trim(), "3km");
	assert_eq!(eval("xs = [1, 2]\nxs cm").serialize().trim(), "[1cm 2cm]");
}

#[test]
fn a_unit_field_error_names_the_unit_the_field_is_written_in() {
	fails_with(&format!("{DISPLAY}\nD(5)"), "D.diagonal holds inch, is given a plain number");
	fails_with(&format!("{DISPLAY}\nd = D(1 inch)\nd.diagonal = 3 s"), "d.diagonal holds inch, is given s");
}
