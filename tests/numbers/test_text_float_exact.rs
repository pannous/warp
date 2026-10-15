//! card text-float: a text read as a float at run time is the float of its literal, "+3.5e-2" is 0.035
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_text_read_as_a_float_at_run_time_is_its_literal() {
	assert_eq!(printed("f(t) := t as float; f(\"+3.5e-2\")"), "0.035");
	assert_eq!(printed("f(t) := float(t); f(\"0.035\")"), "0.035");
	assert_eq!(printed("f(t) := float(t); f(\"-12.5e3\")"), "-12500");
	assert_eq!(printed("f(t) := float(t); f(\"1.7e-5\")"), "0.000017");
}
