//! A trap with no friendlier wording names its cause, not a wasm backtrace

mod common;
use common::fails_with;

#[test]
fn integer_modulo_by_zero_names_the_division() {
	fails_with("5%0", "divide by zero");
	fails_with("x=0; 7 mod x", "divide by zero");
}
