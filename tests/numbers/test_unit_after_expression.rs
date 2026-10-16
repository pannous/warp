//! Units bind only after a number (decision unit-definitions), after any other amount alike: a field, a call or a sum
//! before a unit word is refused the way a variable is, naming `… * 1 km` (reviewer on card decisions-md)
use crate::common::fails_with;
use warp::wasm_emitter::eval;

const SETUP: &str = "class D{diagonal: inch}\nclass P{n}\np = P(96)\nf(x) := x * 2\na = 1\nb = 2";

fn shown(rest: &str) -> String {
	eval(&format!("{SETUP}\n{rest}")).serialize().trim().to_string()
}

#[test]
fn a_unit_after_an_expression_is_refused_like_after_a_variable() {
	fails_with(&format!("{SETUP}\ny = f(3) km\ny"), "* 1 km");
	fails_with(&format!("{SETUP}\ny = (a + b) km\ny"), "write (a+b) * 1 km");
	fails_with(&format!("{SETUP}\nD(p.n inch)"), "write p.n * 1 inch");
	fails_with(&format!("{SETUP}\nD{{diagonal: f(48) inch}}"), "* 1 inch");
}

#[test]
fn the_advised_forms_work() {
	assert_eq!(shown("f(3) * 1 km"), "6km");
	assert_eq!(shown("(a + b) * 1 km"), "3km");
	assert_eq!(shown("D(p.n * 1 inch).diagonal"), "96inch");
	assert_eq!(shown("-3 km"), "-3km");
	assert_eq!(shown("(5) km"), "5km");
}
