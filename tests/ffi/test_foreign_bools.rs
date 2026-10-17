//! card booleans-foreign: a boolean of a foreign runtime or a component is a warp bool (yes/no, notes/bool_type.md),
//! not the Int 1/0 it used to arrive as
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_python_boolean_is_a_bool() {
	assert_eq!(shown("use python math; math.isnan(1)"), "no");
	assert_eq!(shown("use python \"fractions\"; f = fractions.Fraction(1, 2); f < 1"), "yes");
	assert_eq!(shown("use python math; [math.isnan(1)]"), "[no]");
}

#[test]
fn a_js_boolean_is_a_bool() {
	assert_eq!(shown("use js Number; Number.isInteger(2)"), "yes");
}
